use std::collections::HashMap;
use crate::Route;

pub trait UpstreamStrategy {
    fn next_for(&mut self, route: &Route) -> Option<String>;
}

#[derive(Clone, Debug)]
pub struct AlwaysFirstUpstreamStrategy {
}

impl AlwaysFirstUpstreamStrategy {
    pub fn build() -> Self {
        AlwaysFirstUpstreamStrategy {}
    }
}

impl UpstreamStrategy for AlwaysFirstUpstreamStrategy {
    fn next_for(&mut self, route: &Route) -> Option<String> {
        route.upstreams.first()
            .map(|upstream| String::from(upstream))
    }
}

#[derive(Clone, Debug)]
struct RoundRobinUpstreamStrategyStatus {
    next_index: usize,
}

impl RoundRobinUpstreamStrategyStatus {
    fn build() -> Self {
        RoundRobinUpstreamStrategyStatus { next_index: 0 }
    }
}

#[derive(Clone, Debug)]
pub struct RoundRobinUpstreamStrategy {
    status: HashMap<String, RoundRobinUpstreamStrategyStatus>,
}

impl RoundRobinUpstreamStrategy {
    pub fn build() -> Self {
        RoundRobinUpstreamStrategy {
            status: HashMap::new(),
        }
    }
}

impl UpstreamStrategy for RoundRobinUpstreamStrategy {
    fn next_for(&mut self, route: &Route) -> Option<String> {
        let strategy_status = self.status.entry(route.name.clone())
            .or_insert_with(RoundRobinUpstreamStrategyStatus::build);

        let result = route.upstreams.get(strategy_status.next_index)
            .map(|upstream| upstream.to_string());

        strategy_status.next_index = (strategy_status.next_index + 1) % route.upstreams.len();
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::Route;
    use crate::model::upstream_strategy::{AlwaysFirstUpstreamStrategy, RoundRobinUpstreamStrategy, UpstreamStrategy};

    #[test]
    fn should_return_first_in_two_calls() {
        let route = sample_route();
        let mut strategy = AlwaysFirstUpstreamStrategy::build();

        let first_result = strategy.next_for(&route);
        let second_result = strategy.next_for(&route);

        assert_eq!(Some(String::from("upstream1")), first_result);
        assert_eq!(Some(String::from("upstream1")), second_result);
    }

    #[test]
    fn should_return_upstreams_in_round_robin() {
        let route = sample_route();
        let mut strategy = RoundRobinUpstreamStrategy::build();

        let first_result = strategy.next_for(&route);
        let second_result = strategy.next_for(&route);
        let third_result = strategy.next_for(&route);
        let fourth_result = strategy.next_for(&route);

        assert_eq!(Some(String::from("upstream1")), first_result);
        assert_eq!(Some(String::from("upstream2")), second_result);
        assert_eq!(Some(String::from("upstream1")), third_result);
        assert_eq!(Some(String::from("upstream2")), fourth_result);
    }

    fn sample_route() -> Route {
        Route::build(
            "route1",
            &["GET"],
            &["uri1", "uri2"],
            &["upstream1", "upstream2"],
        )
    }
}