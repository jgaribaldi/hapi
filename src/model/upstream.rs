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
}

pub trait UpstreamStrategy {
    fn next(&mut self) -> usize;
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
    fn next(&mut self) -> usize {
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
    size: usize,
}

impl UpstreamStrategy for RoundRobinUpstreamStrategy {
    fn next(&mut self) -> usize {
        let current_index = self.index;
        self.index = (self.index + 1) % self.size;
        current_index
    }

    fn clone_box(&self) -> Box<dyn UpstreamStrategy> {
        Box::new(self.clone())
    }
}

impl RoundRobinUpstreamStrategy {
    pub fn build(index: usize, size: usize) -> Self {
        RoundRobinUpstreamStrategy {
            index,
            size
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::upstream::{AlwaysFirstUpstreamStrategy, RoundRobinUpstreamStrategy, UpstreamStrategy};

    #[test]
    fn should_return_always_first() {
        // given:
        let mut strategy = AlwaysFirstUpstreamStrategy::build();

        // when:
        let first_result = strategy.next();
        let second_result = strategy.next();

        // then:
        assert_eq!(first_result, 0);
        assert_eq!(second_result, 0);
    }

    #[test]
    fn should_return_round_robin() {
        // given:
        let mut strategy = RoundRobinUpstreamStrategy::build(0, 2);

        // when:
        let first_result = strategy.next();
        let second_result = strategy.next();
        let third_result = strategy.next();
        let fourth_result = strategy.next();

        // then:
        assert_eq!(first_result, 0);
        assert_eq!(second_result, 1);
        assert_eq!(third_result, 0);
        assert_eq!(fourth_result, 1);
    }

}