use crate::Route;

pub trait UpstreamStrategy {
    fn next_for(&self, route: &Route) -> Option<String>;
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
    fn next_for(&self, route: &Route) -> Option<String> {
        route.upstreams.first()
            .map(|upstream| String::from(upstream))
    }
}

#[cfg(test)]
mod tests {
    use crate::{AlwaysFirstUpstreamStrategy, Route};
    use crate::model::upstream_strategy::UpstreamStrategy;

    #[test]
    fn should_return_first_in_two_calls() {
        let route = Route::build(
            "route1",
            &["GET"],
            &["uri1", "uri2"],
            &["upstream1", "upstream2"],
        );
        let strategy = AlwaysFirstUpstreamStrategy::build();

        let first_result = strategy.next_for(&route);
        let second_result = strategy.next_for(&route);

        assert_eq!(Some(String::from("upstream1")), first_result);
        assert_eq!(Some(String::from("upstream1")), second_result);
    }
}