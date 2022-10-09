use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::sync::mpsc::Receiver;
use tokio::time::sleep;

use crate::Context;
use crate::infrastructure::serializable_model::Probe;
use crate::model::upstream::UpstreamAddress;

#[derive(Debug)]
pub enum Command {
    Probe { probe: Probe },
    StopProbe { upstream_address: String },
}

pub async fn upstream_probe_handler(mut rx: Receiver<Command>, context: Arc<Mutex<Context>>) {
    let mut probing_tasks = HashMap::new();
    while let Some(message) = rx.recv().await {
        log::debug!("Received message {:?}", message);
        match message {
            Command::Probe { probe } => {
                if !probing_tasks.contains_key(&probe.upstream_address) {
                    let ctx = context.clone();
                    let key = probe.upstream_address.clone();
                    let handle = tokio::spawn(async { probe_upstream(probe, ctx).await });
                    probing_tasks.insert(key, handle);
                }
            }
            Command::StopProbe { upstream_address } => {
                if let Some(handle) = probing_tasks.get(&upstream_address) {
                    log::info!("Shutting down upstream probe for {:?}", &upstream_address);
                    handle.abort();
                    probing_tasks.remove(&upstream_address);
                }
            }
        }
    }
}

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
    use crate::infrastructure::upstream_probe::Poller;

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
