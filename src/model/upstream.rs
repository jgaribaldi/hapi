#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum UpstreamAddress {
    FQDN(String),
    IPv4((u8, u8, u8, u8, u16)),
}

impl UpstreamAddress {
    pub fn to_string(&self) -> String {
        match self {
            UpstreamAddress::FQDN(fqdn) => fqdn.clone(),
            UpstreamAddress::IPv4(ipv4) => {
                format!("{}.{}.{}.{}:{}", ipv4.0, ipv4.1, ipv4.2, ipv4.3, ipv4.4)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Upstream {
    pub address: UpstreamAddress,
    pub enabled: bool,
}

impl Upstream {
    pub fn build_from_fqdn(fqdn: &str) -> Self {
        Upstream {
            address: UpstreamAddress::FQDN(fqdn.to_string()),
            enabled: true,
        }
    }

    pub fn build_from_ipv4(ipv4: (u8, u8, u8, u8, u16)) -> Self {
        Upstream {
            address: UpstreamAddress::IPv4(ipv4),
            enabled: true,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum UpstreamStrategy {
    AlwaysFirst,
    RoundRobin { index: usize },
}

impl UpstreamStrategy {
    pub fn next(&mut self, upstreams: &[&Upstream]) -> usize {
        match self {
            UpstreamStrategy::AlwaysFirst => 0,
            UpstreamStrategy::RoundRobin {
                index: current_index_value,
            } => {
                let current_index = *current_index_value;
                *current_index_value = (*current_index_value + 1) % upstreams.len();

                // this check if for cases in which the upstream array changes in runtime:
                // the array will shrink in size if the upstream falls and the current index could be
                // equal to the available upstreams array length
                if current_index < upstreams.len() {
                    current_index
                } else {
                    upstreams.len() - 1
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::upstream::{Upstream, UpstreamStrategy};

    #[test]
    fn should_return_always_first() {
        // given:
        let mut strategy = UpstreamStrategy::AlwaysFirst;
        let upstream1 = Upstream::build_from_fqdn("localhost:8080");
        let upstream2 = Upstream::build_from_fqdn("localhost:8081");
        let upstreams = vec![&upstream1, &upstream2];

        // when:
        let first_result = strategy.next(upstreams.as_slice());
        let second_result = strategy.next(upstreams.as_slice());

        // then:
        assert_eq!(first_result, 0);
        assert_eq!(second_result, 0);
    }

    #[test]
    fn should_return_round_robin() {
        // given:
        let mut strategy = UpstreamStrategy::RoundRobin { index: 0 };
        let upstream1 = Upstream::build_from_fqdn("localhost:8080");
        let upstream2 = Upstream::build_from_fqdn("localhost:8081");
        let upstreams = vec![&upstream1, &upstream2];

        // when:
        let first_result = strategy.next(upstreams.as_slice());
        let second_result = strategy.next(upstreams.as_slice());
        let third_result = strategy.next(upstreams.as_slice());
        let fourth_result = strategy.next(upstreams.as_slice());

        // then:
        assert_eq!(first_result, 0);
        assert_eq!(second_result, 1);
        assert_eq!(third_result, 0);
        assert_eq!(fourth_result, 1);
    }
}
