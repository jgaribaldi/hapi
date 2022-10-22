use serde::Deserialize;
use serde::Serialize;

use crate::model::upstream::{Upstream, UpstreamStrategy};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Route {
    pub id: String,
    pub name: String,
    pub methods: Vec<String>,
    pub paths: Vec<String>,
    pub strategy: Strategy,
    pub upstreams: Vec<String>,
}

impl From<crate::model::route::Route> for Route {
    fn from(route: crate::model::route::Route) -> Self {
        let upstreams: Vec<String> = route
            .upstreams
            .iter()
            .map(|u| u.address.to_string())
            .collect();

        Route {
            id: route.id.clone(),
            name: route.name.clone(),
            methods: route.methods.clone(),
            paths: route.paths.clone(),
            upstreams,
            strategy: Strategy::from(route.strategy),
        }
    }
}

impl From<Route> for crate::model::route::Route {
    fn from(serializable_route: Route) -> Self {
        let mut upstreams = Vec::new();
        for u in serializable_route.upstreams {
            upstreams.push(Upstream::build_from_fqdn(u.as_str()));
        }

        match serializable_route.strategy {
            Strategy::AlwaysFirst => crate::model::route::Route::build(
                serializable_route.id.clone(),
                serializable_route.name.clone(),
                serializable_route.methods.clone(),
                serializable_route.paths.clone(),
                upstreams,
                UpstreamStrategy::AlwaysFirst,
            ),
            Strategy::RoundRobin => crate::model::route::Route::build(
                serializable_route.id.clone(),
                serializable_route.name.clone(),
                serializable_route.methods.clone(),
                serializable_route.paths.clone(),
                upstreams,
                UpstreamStrategy::RoundRobin { index: 0 },
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Probe {
    pub upstream_address: String,
    pub poll_interval_ms: u64,
    pub error_count: u64,
    pub success_count: u64,
}

impl Probe {
    pub fn default(upstream_address: &str) -> Self {
        Probe {
            upstream_address: upstream_address.to_string(),
            poll_interval_ms: 1000,
            error_count: 5,
            success_count: 5,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Strategy {
    AlwaysFirst,
    RoundRobin,
}

impl From<UpstreamStrategy> for Strategy {
    fn from(upstream_strategy: UpstreamStrategy) -> Self {
        match upstream_strategy {
            UpstreamStrategy::AlwaysFirst => Strategy::AlwaysFirst,
            UpstreamStrategy::RoundRobin { index: _ } => Strategy::RoundRobin,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::infrastructure::serializable_model::{Route, Strategy};
    use crate::model::upstream::{Upstream, UpstreamStrategy};

    #[test]
    fn should_convert_route_to_serializable_route() {
        // given:
        let route = sample_route();

        // when:
        let serializable_route = Route::from(route);

        // then:
        assert_eq!(serializable_route, sample_serializable_route())
    }

    #[test]
    fn should_convert_serializable_route_to_route() {
        // given:
        let serializable_route = sample_serializable_route();

        // when:
        let route: crate::model::route::Route = serializable_route.into();

        // then:
        assert_eq!(route, sample_route())
    }

    fn sample_route() -> crate::model::route::Route {
        crate::model::route::Route::build(
            String::from("id1"),
            String::from("route1"),
            vec![String::from("GET")],
            vec![String::from("uri1"), String::from("uri2")],
            vec![
                Upstream::build_from_fqdn("upstream1"),
                Upstream::build_from_fqdn("upstream2"),
            ],
            UpstreamStrategy::AlwaysFirst,
        )
    }

    fn sample_serializable_route() -> Route {
        Route {
            id: String::from("id1"),
            name: String::from("route1"),
            methods: vec![String::from("GET")],
            paths: vec![String::from("uri1"), String::from("uri2")],
            upstreams: vec![String::from("upstream1"), String::from("upstream2")],
            strategy: Strategy::AlwaysFirst,
        }
    }
}
