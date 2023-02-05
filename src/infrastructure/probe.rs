use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tokio::time::sleep;

use crate::infrastructure::serializable_model::Probe;
use crate::Context;
use crate::modules::core::upstream::UpstreamAddress;
use crate::modules::probe::Poller;

#[derive(Debug)]
pub enum Command {
    RebuildProbes,
    StopProbes,
}

/// Task that manages the upstream probes thread: listens to commands in the channel and acts
/// accordingly
pub async fn probe_handler(
    mut rx: Receiver<Command>,
    context: Arc<Mutex<Context>>,
    probe_settings: Option<Vec<Probe>>,
) {
    let mut upstream_probe_controller =
        ProbeController::build(context.clone(), probe_settings);

    while let Some(message) = rx.recv().await {
        log::debug!("Received message {:?}", message);
        match message {
            Command::RebuildProbes => upstream_probe_controller.rebuild_probes(),
            Command::StopProbes => upstream_probe_controller.shutdown_all_probes(),
        }
    }
}

struct ProbeController {
    probes_status: HashMap<UpstreamAddress, JoinHandle<()>>,
    context: Arc<Mutex<Context>>,
    probe_settings: HashMap<String, Probe>,
}

impl ProbeController {
    pub fn build(context: Arc<Mutex<Context>>, probe_settings: Option<Vec<Probe>>) -> Self {
        let probe_settings = match probe_settings {
            Some(probes) => {
                let mut ps = HashMap::new();
                for p in probes.iter() {
                    ps.insert(p.upstream_address.clone(), p.clone());
                }
                ps
            }
            None => HashMap::new(),
        };

        ProbeController {
            probes_status: HashMap::new(),
            context,
            probe_settings,
        }
    }

    pub fn rebuild_probes(&mut self) {
        let upstreams = self.get_current_upstreams();
        let being_probed = self.get_upstreams_being_probed();

        let to_add = probes_to_add(&upstreams, &being_probed);
        let to_remove = probes_to_remove(&upstreams, &being_probed);

        for u in to_add {
            self.do_add_probe(u);
        }

        for u in to_remove {
            self.do_remove_probe(u);
        }
    }

    pub fn shutdown_all_probes(&mut self) {
        let current_upstreams = HashSet::new();
        let being_probed = self.get_upstreams_being_probed();
        let to_remove = probes_to_remove(&current_upstreams, &being_probed);

        for u in to_remove {
            self.do_remove_probe(u);
        }
    }

    fn get_current_upstreams(&self) -> HashSet<UpstreamAddress> {
        let ctx = self.context.lock().unwrap();
        let mut result = HashSet::new();
        for u in ctx.get_all_upstreams().unwrap().iter() {
            result.insert(u.clone());
        }
        result
    }

    fn get_upstreams_being_probed(&self) -> HashSet<UpstreamAddress> {
        let mut result = HashSet::new();
        for t in self.probes_status.keys() {
            result.insert(t.clone());
        }
        result
    }

    /// Spawn a new probing task for the given upstream and add it to the probe handler state
    fn do_add_probe(&mut self, to_add: &UpstreamAddress) {
        log::debug!("Spawning upstream probe for {:?}", to_add);
        let handle = self.create_probe_and_spawn_task(to_add.to_string().as_str());
        match self.probes_status.insert(to_add.clone(), handle) {
            None => {}
            Some(old_handle) => old_handle.abort(),
        }
    }

    fn create_probe_and_spawn_task(&self, upstream_to_add: &str) -> JoinHandle<()> {
        let probe = match self.probe_settings.get(upstream_to_add) {
            Some(existing_setting) => existing_setting.clone(),
            None => Probe::default(upstream_to_add),
        };

        let context = self.context.clone();
        tokio::spawn(async { probe_upstream(probe, context).await })
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

/// Calculates the difference between the current upstreams and the upstreams being probed,
/// indicating that "new" upstreams are present in the current context and we need to start probing
/// them
fn probes_to_add<'a>(
    current_upstreams: &'a HashSet<UpstreamAddress>,
    upstreams_being_probed: &'a HashSet<UpstreamAddress>,
) -> Vec<&'a UpstreamAddress> {
    let mut result = Vec::new();
    for u in current_upstreams.difference(upstreams_being_probed) {
        result.push(u);
    }
    result
}

/// Calculates the difference between the upstreams currently being probed and the upstreams present
/// in the current context, indicating that "old" upstreams are being probed (upstreams that no
/// longer exist in the current context) and that we should stop probing them
fn probes_to_remove<'a>(
    current_upstreams: &'a HashSet<UpstreamAddress>,
    upstreams_being_probed: &'a HashSet<UpstreamAddress>,
) -> Vec<&'a UpstreamAddress> {
    let mut result = Vec::new();
    for u in upstreams_being_probed.difference(current_upstreams) {
        result.push(u);
    }
    result
}

/// Task that probes an upstream according to the given configuration (probe): if it detects that
/// the upstream is down, it disables it in the current context. If it detects that the upstream is
/// back up, it enables it in the current context.
/// The test to see if a given upstream is "up" is done establishing a TCP connection to the
/// upstream address.
async fn probe_upstream(configuration: Probe, context: Arc<Mutex<Context>>) {
    let mut poller = Poller::build(configuration.error_count, configuration.success_count);
    log::info!("Probing upstream with configuration {:?}", configuration);

    loop {
        sleep(Duration::from_millis(configuration.poll_interval_ms)).await;
        let poll_result = TcpStream::connect(&configuration.upstream_address).await;

        match poll_result {
            Ok(_) => {
                let upstream_was_enabled = poller.check_and_enable_upstream();
                if upstream_was_enabled {
                    log::info!(
                        "Reached success count for upstream {:?}: re-enabling",
                        configuration.upstream_address,
                    );
                    let addr = UpstreamAddress::FQDN(configuration.upstream_address.clone());
                    let mut ctx = context.lock().unwrap();
                    ctx.enable_upstream_for_all_routes(&addr);
                }
            }
            Err(_) => {
                let upstream_was_disabled = poller.check_and_disable_upstream();
                if upstream_was_disabled {
                    log::warn!(
                        "Reached error count for upstream {:?}: disabling",
                        configuration.upstream_address,
                    );
                    let addr = UpstreamAddress::FQDN(configuration.upstream_address.clone());
                    let mut ctx = context.lock().unwrap();
                    ctx.disable_upstream_for_all_routes(&addr);
                }
            }
        }
    }
}