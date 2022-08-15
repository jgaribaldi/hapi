use std::fmt::{Debug, Formatter};

#[derive(Clone, Debug)]
pub struct Upstream {
    pub address: String,
    pub enabled: bool,
}

impl Upstream {
    pub fn build(address: &str) -> Self {
        Upstream {
            address: address.to_string(),
            enabled: true,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn has_address(
        &self,
        address: &str
    ) -> bool {
        self.address == String::from(address)
    }
}

pub trait UpstreamStrategy {
    fn next(&mut self, upstreams: &[&Upstream]) -> usize;
    fn clone_box(&self) -> Box<dyn UpstreamStrategy>;
}

impl Debug for (dyn UpstreamStrategy + 'static) {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "UpstreamStrategy")
    }
}

impl Clone for Box<dyn UpstreamStrategy> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AlwaysFirstUpstreamStrategy {
}

impl UpstreamStrategy for AlwaysFirstUpstreamStrategy {
    fn next(&mut self, _: &[&Upstream]) -> usize {
        0
    }

    fn clone_box(&self) -> Box<dyn UpstreamStrategy> {
        Box::new(self.clone())
    }
}

impl AlwaysFirstUpstreamStrategy {
    pub fn build() -> Self {
        AlwaysFirstUpstreamStrategy {}
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RoundRobinUpstreamStrategy {
    index: usize,
}

impl UpstreamStrategy for RoundRobinUpstreamStrategy {
    fn next(&mut self, upstreams: &[&Upstream]) -> usize {
        let current_index = self.index;
        self.index = (self.index + 1) % upstreams.len();
        current_index
    }

    fn clone_box(&self) -> Box<dyn UpstreamStrategy> {
        Box::new(self.clone())
    }
}

impl RoundRobinUpstreamStrategy {
    pub fn build(index: usize) -> Self {
        RoundRobinUpstreamStrategy {
            index,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::upstream::{AlwaysFirstUpstreamStrategy, RoundRobinUpstreamStrategy, UpstreamStrategy};
    use crate::Upstream;

    #[test]
    fn should_return_always_first() {
        // given:
        let mut strategy = AlwaysFirstUpstreamStrategy::build();
        let upstream1 = Upstream::build("localhost:8080");
        let upstream2 = Upstream::build("localhost:8081");
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
        let mut strategy = RoundRobinUpstreamStrategy::build(0);
        let upstream1 = Upstream::build("localhost:8080");
        let upstream2 = Upstream::build("localhost:8081");
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

    #[test]
    fn should_have_address() {
        // given:
        let upstream = Upstream::build("upstream1:8080");

        // when:
        let result = upstream.has_address("upstream1:8080");

        // then:
        assert_eq!(true, result)
    }

    #[test]
    fn should_not_have_address() {
        // given:
        let upstream = Upstream::build("upstream1:8080");

        // when:
        let result = upstream.has_address("upstream1:8081");

        // then:
        assert_eq!(false, result)
    }
}