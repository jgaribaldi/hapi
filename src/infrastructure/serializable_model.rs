use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::str::FromStr;
use crate::modules::core::upstream::{Upstream, UpstreamStrategy};

const IPV4_REGEX: &str = "^(\\d|[1-9]\\d|1\\d\\d|2[0-4]\\d|25[0-5])\\.(\\d|[1-9]\\d|1\\d\\d|2[0-4]\\d|25[0-5])\\.(\\d|[1-9]\\d|1\\d\\d|2[0-4]\\d|25[0-5])\\.(\\d|[1-9]\\d|1\\d\\d|2[0-4]\\d|25[0-5])(:(0|[1-9][0-9]{0,3}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5]))*$";

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Route {
    pub id: String,
    pub name: String,
    pub methods: Vec<String>,
    pub paths: Vec<String>,
    pub strategy: Strategy,
    pub upstreams: Vec<String>,
}

impl From<crate::modules::core::route::Route> for Route {
    fn from(route: crate::modules::core::route::Route) -> Self {
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

impl From<Route> for crate::modules::core::route::Route {
    fn from(serializable_route: Route) -> Self {
        let mut upstreams = Vec::new();
        let regex = Regex::new(IPV4_REGEX).unwrap();

        for u in serializable_route.upstreams {
            let upstream = if regex.is_match(u.as_str()) {
                let tuple = upstream_str_to_tuple(&regex, u.as_str());
                Upstream::build_from_ipv4(tuple)
            } else {
                Upstream::build_from_fqdn(u.as_str())
            };
            upstreams.push(upstream)
        }

        match serializable_route.strategy {
            Strategy::AlwaysFirst => crate::modules::core::route::Route::build(
                serializable_route.id.clone(),
                serializable_route.name.clone(),
                serializable_route.methods.clone(),
                serializable_route.paths.clone(),
                upstreams,
                UpstreamStrategy::AlwaysFirst,
            ),
            Strategy::RoundRobin => crate::modules::core::route::Route::build(
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

fn upstream_str_to_tuple(regex: &Regex, upstream: &str) -> (u8, u8, u8, u8, u16) {
    let parts = regex.captures(upstream).unwrap();

    let octet1 = u8::from_str(&parts[1]).unwrap();
    let octet2 = u8::from_str(&parts[2]).unwrap();
    let octet3 = u8::from_str(&parts[3]).unwrap();
    let octet4 = u8::from_str(&parts[4]).unwrap();

    let mut group_counter = 0;
    for p in parts.iter() {
        if p.is_some() {
            group_counter += 1;
        }
    }

    if group_counter == 5 {
        (octet1, octet2, octet3, octet4, 80)
    } else {
        let port = u16::from_str(&parts[6]).unwrap();
        (octet1, octet2, octet3, octet4, port)
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
    use crate::infrastructure::serializable_model::{
        upstream_str_to_tuple, Route, Strategy, IPV4_REGEX,
    };
    use regex::Regex;
    use crate::modules::core::upstream::{Upstream, UpstreamStrategy};

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
        let route: crate::modules::core::route::Route = serializable_route.into();

        // then:
        assert_eq!(route, sample_route())
    }

    #[test]
    fn should_convert_serializable_ipv4_route_to_route() {
        // given:
        let serializable_route = sample_serializable_route_ipv4();

        // when:
        let route: crate::modules::core::route::Route = serializable_route.into();

        // then:
        assert_eq!(route, sample_route_ipv4())
    }

    #[test]
    fn should_convert_upstream_str_to_tuple_ip_only() {
        // given:
        let regex = Regex::new(IPV4_REGEX).unwrap();
        let upstream = "192.168.0.100";

        // when:
        let result = upstream_str_to_tuple(&regex, upstream);

        // then:
        assert_eq!(result, (192, 168, 0, 100, 80))
    }

    #[test]
    fn should_convert_upstream_str_to_tuple_ip_and_port() {
        // given:
        let regex = Regex::new(IPV4_REGEX).unwrap();
        let upstream = "192.168.0.100:8080";

        // when:
        let result = upstream_str_to_tuple(&regex, upstream);

        // then:
        assert_eq!(result, (192, 168, 0, 100, 8080))
    }

    fn sample_route() -> crate::modules::core::route::Route {
        crate::modules::core::route::Route::build(
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

    fn sample_route_ipv4() -> crate::modules::core::route::Route {
        crate::modules::core::route::Route::build(
            String::from("id1"),
            String::from("route1"),
            vec![String::from("GET")],
            vec![String::from("uri1"), String::from("uri2")],
            vec![
                Upstream::build_from_ipv4((192, 168, 0, 100, 80)),
                Upstream::build_from_ipv4((192, 168, 0, 101, 8080)),
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

    fn sample_serializable_route_ipv4() -> Route {
        Route {
            id: String::from("id1"),
            name: String::from("route1"),
            methods: vec![String::from("GET")],
            paths: vec![String::from("uri1"), String::from("uri2")],
            upstreams: vec![
                String::from("192.168.0.100"),
                String::from("192.168.0.101:8080"),
            ],
            strategy: Strategy::AlwaysFirst,
        }
    }
}
