use std::collections::HashMap;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use crate::errors::HapiError;
use crate::events::commands::Command;
use crate::events::events::Event;
use crate::events::events::Event::{ProbeWasStarted, ProbeWasStopped, RouteWasAdded, RouteWasRemoved, StatsWereFound, StatWasCounted, UpstreamWasDisabled, UpstreamWasEnabled, UpstreamWasFound, UpstreamWasNotFound};
use crate::infrastructure::settings::HapiSettings;
use crate::modules::core::context::Context;
use crate::modules::core::upstream::UpstreamAddress;
use crate::modules::probe::Poller;
use crate::modules::stats::Stats;

pub(crate) async fn handle_core(mut recv_cmd: Receiver<Command>, send_evt: Sender<Event>) {
    // TODO: remove .unwrap()
    let settings = HapiSettings::load_from_file("settings.json").unwrap();
    log::info!("Settings {:?}", settings);

    // TODO: remove .unwrap()
    let mut context = build_context_from_settings(&settings).unwrap();

    while let Ok(command) = recv_cmd.recv().await {
        log::debug!("Received command {:?}", command);
        let maybe_event = match command {
            Command::LookupUpstream { id, client, path, method } => {
                context.upstream_lookup(path.as_str(), method.as_str())
                    .map(|upstream| {
                        UpstreamWasFound { cmd_id: id.clone(), upstream_address: upstream }
                    })
                    .or(Some(UpstreamWasNotFound { cmd_id: id.clone() }))
            },
            // TODO fix
            Command::EnableUpstream { id } => Some(UpstreamWasEnabled { cmd_id: id }),
            Command::DisableUpstream { id } => Some(UpstreamWasDisabled { cmd_id: id }),
            Command::AddRoute { id } => Some(RouteWasAdded { cmd_id: id }),
            Command::RemoveRoute { id } => Some(RouteWasRemoved { cmd_id: id }),
            _ => None,
        };

        if let Some(event) = maybe_event {
            match send_evt.send(event) {
                Ok(_) => log::debug!("Event sent"),
                Err(e) => log::error!("Error sending event {}", e),
            }
        }
    }
}

fn build_context_from_settings(settings: &HapiSettings) -> Result<Context, HapiError> {
    let mut context = Context::build_empty();
    for r in settings.routes() {
        context.add_route(r)?;
    }
    Ok(context)
}

pub(crate) async fn handle_stats(mut recv_cmd: Receiver<Command>, send_evt: Sender<Event>) {
    let mut stats = Stats::build();

    while let Ok(command) = recv_cmd.recv().await {
        let maybe_event = match command {
            Command::CountStat { id } => {
                // TODO: fix
                stats.count_request("client", "method", "path", "upstream");
                Some(StatWasCounted { cmd_id: id })
            },
            Command::LookupStats { id } => Some(StatsWereFound { cmd_id: id }),
            _ => None,
        };

        if let Some(event) = maybe_event {
            match send_evt.send(event) {
                Ok(_) => log::debug!("Event sent"),
                Err(e) => log::error!("Error sending event {}", e),
            }
        }
    }
}

pub(crate) async fn handle_probes(
    mut recv_cmd: Receiver<Command>,
    send_evt: Sender<Event>,
    send_cmd: Sender<Command>,
) {
    let mut probe_controller = ProbeController::build(send_cmd);

    while let Ok(command) = recv_cmd.recv().await {
        let maybe_event = match command {
            Command::StartProbe { id } => {
                // TODO: fix
                let upstream_address = UpstreamAddress::FQDN(String::from("127.0.0.1"));
                probe_controller.do_add_probe(&upstream_address);
                Some(ProbeWasStarted { cmd_id: id })
            },
            Command::StopProbe { id } => {
                // TODO: fix
                let upstream_address = UpstreamAddress::FQDN(String::from("127.0.0.1"));
                probe_controller.do_remove_probe(&upstream_address);
                Some(ProbeWasStopped { cmd_id: id })
            },
            _ => None,
        };

        if let Some(event) = maybe_event {
            match send_evt.send(event) {
                Ok(_) => log::debug!("Event sent"),
                Err(e) => log::error!("Error sending event {}", e),
            }
        }
    }
}

struct ProbeController {
    probes_status: HashMap<UpstreamAddress, JoinHandle<()>>,
    send_cmd: Sender<Command>,
}

impl ProbeController {
    fn build(send_cmd: Sender<Command>) -> Self {
        ProbeController {
            probes_status: HashMap::new(),
            send_cmd,
        }
    }

    /// Spawn a new probing task for the given upstream and add it to the probe handler state
    fn do_add_probe(&mut self, to_add: &UpstreamAddress) {
        log::debug!("Spawning upstream probe for {:?}", to_add);
        let upstream_address = to_add.to_string();
        let send_cmd = self.send_cmd.clone();
        let handle = tokio::spawn(async {
            probe_upstream(upstream_address, send_cmd).await
        });
        match self.probes_status.insert(to_add.clone(), handle) {
            None => {}
            Some(old_handle) => old_handle.abort(),
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
async fn probe_upstream(upstream_address: String, send_cmd: Sender<Command>) {
    let mut poller = Poller::build(5, 5);

    loop {
        sleep(Duration::from_millis(2000)).await;
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
                    // TODO: fix
                    let command = Command::EnableUpstream { id: String::from("12324") };
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
                    // TODO: fix
                    let command = Command::DisableUpstream { id: String::from("12324") };
                    match send_cmd.send(command) {
                        Ok(_) => log::debug!("Command sent"),
                        Err(e) => log::error!("Error sending command {}", e),
                    }
                }
            }
        }
    }
}