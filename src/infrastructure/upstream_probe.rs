use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::sleep;

use crate::Context;

pub struct UpstreamProbeConfiguration {
    pub upstream: String,
    pub poll_interval_ms: u64, // how often to poll upstream, in milliseconds
    pub error_count: u64,      // times in a row for TCP connect error to disable upstream
    pub success_count: u64,    // times in a row for TCP connect success to re-enable upstream
}

impl UpstreamProbeConfiguration {
    pub fn build(
        upstream: &str,
        poll_interval_ms: u64,
        error_count: u64,
        success_count: u64,
    ) -> Self {
        UpstreamProbeConfiguration {
            upstream: upstream.to_string(),
            poll_interval_ms,
            error_count,
            success_count,
        }
    }
}

pub async fn probe_upstream(
    configuration: UpstreamProbeConfiguration,
    context: Arc<Mutex<Context>>,
) {
    let mut poller = Poller::build(configuration.error_count, configuration.success_count);

    loop {
        sleep(Duration::from_millis(configuration.poll_interval_ms)).await;
        let poll_result = TcpStream::connect(&configuration.upstream).await;

        match poll_result {
            Ok(_) => {
                let upstream_was_enabled = poller.check_and_enable_upstream();
                if upstream_was_enabled {
                    log::info!(
                        "Reached success count for upstream {}: re-enabling",
                        configuration.upstream,
                    );
                    let mut ctx = context.lock().unwrap();
                    ctx.enable_upstream_for_all_routes(&configuration.upstream.as_str());
                }
            }
            Err(_) => {
                let upstream_was_disabled = poller.check_and_disable_upstream();
                if upstream_was_disabled {
                    log::warn!(
                        "Reached error count for upstream {}: disabling",
                        configuration.upstream,
                    );
                    let mut ctx = context.lock().unwrap();
                    ctx.disable_upstream_for_all_routes(&configuration.upstream.as_str());
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
