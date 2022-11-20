use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tokio::time::sleep;

use crate::infrastructure::serializable_model::Probe;
use crate::model::upstream::UpstreamAddress;
use crate::Context;

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

        for u in to_add.iter() {
            self.do_add_probe(u);
        }

        for u in to_remove.iter() {
            self.do_remove_probe(u);
        }
    }

    pub fn shutdown_all_probes(&mut self) {
        let current_upstreams = HashSet::new();
        let being_probed = self.get_upstreams_being_probed();
        let to_remove = probes_to_remove(&current_upstreams, &being_probed);

        for u in to_remove.iter() {
            self.do_remove_probe(u);
        }
    }

    fn get_current_upstreams(&self) -> HashSet<UpstreamAddress> {
        let ctx = self.context.lock().unwrap();
        let mut result = HashSet::new();
        for u in ctx.get_all_upstreams().iter() {
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
fn probes_to_add(
    current_upstreams: &HashSet<UpstreamAddress>,
    upstreams_being_probed: &HashSet<UpstreamAddress>,
) -> Vec<UpstreamAddress> {
    let mut result = Vec::new();
    for u in current_upstreams.difference(upstreams_being_probed) {
        result.push(u.clone());
    }
    result
}

/// Calculates the difference between the upstreams currently being probed and the upstreams present
/// in the current context, indicating that "old" upstreams are being probed (upstreams that no
/// longer exist in the current context) and that we should stop probing them
fn probes_to_remove(
    current_upstreams: &HashSet<UpstreamAddress>,
    upstreams_being_probed: &HashSet<UpstreamAddress>,
) -> Vec<UpstreamAddress> {
    let mut result = Vec::new();
    for u in upstreams_being_probed.difference(current_upstreams) {
        result.push(u.clone());
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

struct Poller {
    error_count: u64,
    success_count: u64,
    current_error_count: u64,
    current_success_count: u64,
    upstream_enabled: bool,
}

impl Poller {
    fn build(error_count: u64, success_count: u64) -> Self {
        Poller {
            error_count,
            success_count,
            current_error_count: 0,
            current_success_count: 0,
            upstream_enabled: true,
        }
    }

    /// Returns `true` if the upstream was enabled
    fn check_and_enable_upstream(&mut self) -> bool {
        if !self.upstream_enabled {
            // start counting successes only if upstream is disabled
            self.current_success_count += 1;

            if self.current_success_count == self.success_count {
                // reached maximum success count => enable upstream and reset current count
                self.upstream_enabled = true;
                self.current_success_count = 0;
                return true;
            }
        }
        return false;
    }

    /// Returns `true` if the upstream was disabled
    fn check_and_disable_upstream(&mut self) -> bool {
        if self.upstream_enabled {
            // start counting errors only if upstream is enabled
            self.current_error_count += 1;

            if self.current_error_count == self.error_count {
                // reached maximum error count => disable upstream and reset current count
                self.upstream_enabled = false;
                self.current_error_count = 0;
                return true;
            }
        }
        return false;
    }
}

#[cfg(test)]
mod tests {
    use crate::infrastructure::probe::Poller;

    #[test]
    fn should_enable_upstream_if_reached_success_count() {
        // given:
        let mut poller = Poller::build(3, 3);
        poller.upstream_enabled = false; // start with a disabled upstream
        poller.current_success_count = 2;

        // when:
        let result = poller.check_and_enable_upstream();

        // then:
        assert_eq!(true, result);
        assert_eq!(true, poller.upstream_enabled);
        assert_eq!(0, poller.current_error_count);
    }

    #[test]
    fn should_disable_upstream_if_reached_error_count() {
        // given:
        let mut poller = Poller::build(3, 3);
        poller.current_error_count = 2;

        // when:
        let result = poller.check_and_disable_upstream();

        // then:
        assert_eq!(true, result);
        assert_eq!(false, poller.upstream_enabled);
        assert_eq!(0, poller.current_error_count);
    }

    #[test]
    fn should_not_enable_upstream_if_success_count_not_reached() {
        // given:
        let mut poller = Poller::build(3, 3);
        poller.upstream_enabled = false; // start with a disabled upstream

        // when:
        poller.check_and_enable_upstream();
        let result = poller.check_and_enable_upstream();

        // then:
        assert_eq!(false, result);
        assert_eq!(false, poller.upstream_enabled);
        assert_eq!(2, poller.current_success_count);
    }

    #[test]
    fn should_not_disable_upstream_if_error_count_not_reached() {
        // given:
        let mut poller = Poller::build(3, 3);

        // when:
        poller.check_and_disable_upstream();
        let result = poller.check_and_disable_upstream();

        // then:
        assert_eq!(false, result);
        assert_eq!(true, poller.upstream_enabled);
        assert_eq!(2, poller.current_error_count);
    }
}
