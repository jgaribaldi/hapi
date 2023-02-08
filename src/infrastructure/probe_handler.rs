use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use uuid::Uuid;
use crate::errors::HapiError;
use crate::events::commands::Command;
use crate::events::events::Event;
use crate::infrastructure::settings::{HapiSettings, ProbeSettings};
use crate::modules::core::upstream::UpstreamAddress;
use crate::modules::probe::Poller;

pub(crate) async fn handle_probes(
    mut recv_evt: Receiver<Event>,
    send_cmd: Sender<Command>,
    _send_evt: Sender<Event>,
) {
    // TODO: remove unwrap()
    let settings = HapiSettings::load_from_file("settings.json").unwrap();
    let mut probe_controller = ProbeController::build(send_cmd, settings.probes);

    while let Ok(event) = recv_evt.recv().await {
        match event {
            Event::RouteWasAdded { cmd_id, route } => {
                for upstream in route.upstreams {
                    probe_controller.add_probe(&upstream.address);
                }
            },
            Event::RouteWasRemoved { cmd_id, route } => {
                for upstream in route.upstreams {
                    probe_controller.remove_probe(&upstream.address);
                }
            },
            _ => {},
        }
    }
}

struct ProbeController {
    probes_status: HashMap<UpstreamAddress, JoinHandle<()>>,
    upstream_counter: HashMap<UpstreamAddress, u64>, // how many routes point to this upstream
    send_cmd: Sender<Command>,
    default_probes: Option<HashMap<String, ProbeSettings>>,
}

impl ProbeController {
    fn build(send_cmd: Sender<Command>, default_probes: Option<Vec<ProbeSettings>>) -> Self {
        let mut probes_map = default_probes.map_or(None, |dp| {
            let mut map = HashMap::new();
            for p in dp.iter() {
                map.insert(p.upstream_address.clone(), p.clone());
            }
            Some(map)
        });

        ProbeController {
            probes_status: HashMap::new(),
            upstream_counter: HashMap::new(),
            send_cmd,
            default_probes: probes_map,
        }
    }

    fn add_probe(&mut self, to_add: &UpstreamAddress) -> Option<UpstreamAddress> {
        if let Some(current_count) = self.upstream_counter.get_mut(to_add) {
            // we are already probing for the given upstream, just know that there's another route
            // using the same upstream
            log::debug!("Upstream {} is already being probed with count {}. Increasing 1", to_add, current_count);
            *current_count = *current_count + 1;
            None
        } else {
            // we need to start probing the given upstream
            log::debug!("Upstream {} is not being probed, launching new probe", to_add);
            self.do_add_probe(to_add);
            self.upstream_counter.insert(to_add.clone(), 1);
            Some(to_add.clone())
        }
    }

    fn remove_probe(&mut self, to_remove: &UpstreamAddress) -> Option<UpstreamAddress> {
        if let Some(current_count) = self.upstream_counter.get_mut(to_remove) {
            if *current_count == 1 {
                log::debug!("Current count for upstream {} is 1, removing", to_remove);
                self.do_remove_probe(to_remove);
                self.upstream_counter.remove(to_remove);
                Some(to_remove.clone())
            } else {
                log::debug!("Current count for upstream {} is {}, decreasing counter", to_remove, current_count);
                *current_count = *current_count - 1;
                None
            }
        } else {
            log::warn!("Given probe to remove {} does not exist in the probe controller state", to_remove);
            None
        }
    }

    /// Spawn a new probing task for the given upstream and add it to the probe handler state
    fn do_add_probe(&mut self, to_add: &UpstreamAddress) {
        let probe_settings = self.probe_settings_for(to_add);
        log::debug!("Spawning upstream probe for {:?} with settings {:?}", to_add, probe_settings);

        let to_add_2 = to_add.clone();
        let send_cmd = self.send_cmd.clone();
        let handle = tokio::spawn(async move {
            let to_add_2 = to_add_2.clone();
            let upstream_address = to_add_2.to_string();
            probe_upstream(upstream_address, send_cmd, probe_settings).await
        });

        let to_add = to_add.clone();
        match self.probes_status.insert(to_add.clone(), handle) {
            None => {}
            Some(old_handle) => old_handle.abort(),
        }
    }

    fn probe_settings_for(&self, upstream_address: &UpstreamAddress) -> ProbeSettings {
        if self.default_probes.is_some() {
            let maybe_default = self.default_probes.as_ref().unwrap().get(upstream_address.to_string().as_str());
            if maybe_default.is_some() {
                maybe_default.unwrap().clone()
            } else {
                ProbeSettings::default(upstream_address.to_string().as_str())
            }
        } else {
            ProbeSettings::default(upstream_address.to_string().as_str())
        }
    }

    /// Kill the probing task for the given upstream and remove it from the probe handler state
    fn do_remove_probe(&mut self, to_remove: &UpstreamAddress) {
        log::info!("Shutting down upstream probe for {:?}", to_remove);
        match self.probes_status.remove(to_remove) {
            Some(handle) => handle.abort(),
            None => log::warn!(
                "Given upstream to remove is not present in the current state {:?}",
                to_remove
            ),
        }
    }
}

/// Task that probes an upstream according to the given configuration (probe): if it detects that
/// the upstream is down, it disables it in the current context. If it detects that the upstream is
/// back up, it enables it in the current context.
/// The test to see if a given upstream is "up" is done establishing a TCP connection to the
/// upstream address.
async fn probe_upstream(
    upstream_address: String,
    send_cmd: Sender<Command>,
    probe_settings: ProbeSettings,
) {
    let mut poller = Poller::build(probe_settings.error_count, probe_settings.success_count);

    loop {
        sleep(Duration::from_millis(probe_settings.poll_interval_ms)).await;
        let poll_result = TcpStream::connect(&upstream_address).await;

        match poll_result {
            Ok(_) => {
                let upstream_was_enabled = poller.check_and_enable_upstream();
                if upstream_was_enabled {
                    log::info!(
                        "Reached success count for upstream {:?}: re-enabling",
                        upstream_address,
                    );
                    // send enable upstream command to core
                    let cmd_uuid = Uuid::new_v4();
                    let command = Command::EnableUpstream { id: cmd_uuid.to_string(), upstream_address: UpstreamAddress::FQDN(upstream_address.clone()) };
                    match send_cmd.send(command) {
                        Ok(_) => log::debug!("Command sent"),
                        Err(e) => log::error!("Error sending command {}", e),
                    }
                }
            },
            Err(_) => {
                let upstream_was_disabled = poller.check_and_disable_upstream();
                if upstream_was_disabled {
                    log::warn!(
                        "Reached error count for upstream {:?}: disabling",
                        upstream_address,
                    );
                    // send disable upstream command to core
                    let cmd_uuid = Uuid::new_v4();
                    let command = Command::DisableUpstream { id: cmd_uuid.to_string(), upstream_address: UpstreamAddress::FQDN(upstream_address.clone()) };
                    match send_cmd.send(command) {
                        Ok(_) => log::debug!("Command sent"),
                        Err(e) => log::error!("Error sending command {}", e),
                    }
                }
            }
        }
    }
}