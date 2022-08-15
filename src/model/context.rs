use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use regex::Regex;

use crate::model::route::Route;

#[derive(Clone,Debug)]
pub struct Context {
    routes: Vec<Route>,
    routing_table: HashMap<(String, String), usize>, // (path, method) => route index
}

impl Context {
    pub fn build_from_routes(
        routes: Vec<Route>,
    ) -> Self {
        let mut table: HashMap<(String, String), usize> = HashMap::new();

        for (index, route) in routes.iter().enumerate() {
            for path in route.paths.iter() {
                for method in route.methods.iter() {
                    table.insert((path.to_string(), method.to_string()), index);
                }
            }
        }

        Context {
            routes,
            routing_table: table,
        }
    }

    pub fn upstream_lookup(
        &mut self,
        path: &str,
        method: &str,
    ) -> Option<String> {
        self.find_routing_table_index(path, method)
            .and_then(|index| self.routes.get_mut(index))
            .and_then(|route| {
                route.next_available_upstream()
            })
            .map(|upstream| {
                upstream.clone().address
            })
    }

    pub fn get_upstreams(&self) -> Vec<String> {
        let mut upstream_set = HashSet::new();

        for route in self.routes.iter() {
            for upstream in route.upstreams.iter() {
                upstream_set.insert(upstream.address.clone());
            }
        }

        upstream_set.into_iter().collect()
    }

    pub fn disable_upstream_for_all_routes(
        &mut self,
        upstream: &str,
    ) {
        for route in self.routes.iter_mut() {
            route.disable_upstream(upstream)
        }
    }

    fn find_routing_table_index(
        &self,
        path: &str,
        method: &str,
    ) -> Option<usize> {
        // attempt exact match by (path, method) key
        let exact_key = (path.to_string(), method.to_string());

        self.routing_table.get(&exact_key)
            .map(|value| {
                *value
            })
            .or_else(|| {
                let mut result = None;
                // attempt matching by regexp
                for (key, value) in self.routing_table.iter() {
                    let path_regexp = Regex::new(
                        wrap_string_in_regexp(key.0.as_str()).as_str()
                    ).unwrap();
                    let method_regexp = Regex::new(
                        wrap_string_in_regexp(key.1.as_str()).as_str()
                    ).unwrap();

                    if path_regexp.is_match(path) && method_regexp.is_match(method) {
                        result = Some(value.clone());
                        break
                    }
                }
                result
            })
    }
}

fn wrap_string_in_regexp(string: &str) -> String {
    let mut result = String::new();
    result.push_str("^");
    result.push_str(string);
    result.push_str("$");
    result
}

// TODO: this is awful, remove it
unsafe impl Send for Context {
}

#[cfg(test)]
mod tests {
    use crate::Context;
    use crate::model::route::Route;
    use crate::model::upstream::{AlwaysFirstUpstreamStrategy, RoundRobinUpstreamStrategy, Upstream, UpstreamStrategy};

    #[test]
    fn should_create_context_from_routes() {
        // given:
        let routes = vec![
            sample_route_1(Box::new(AlwaysFirstUpstreamStrategy::build())),
            sample_route_2(Box::new(RoundRobinUpstreamStrategy::build(0))),
        ];

        // when:
        let context = Context::build_from_routes(routes);

        // then:
        assert_eq!(context.routing_table.len(), 3);
    }

    #[test]
    fn should_perform_upstream_lookup() {
        // given:
        let routes = vec![
            sample_route_1(Box::new(AlwaysFirstUpstreamStrategy::build())),
            sample_route_2(Box::new(RoundRobinUpstreamStrategy::build(0))),
        ];
        let mut context = Context::build_from_routes(routes);

        // when:
        let upstream = context.upstream_lookup("uri1", "GET");

        // then:
        assert_eq!("upstream1", upstream.unwrap());
    }

    #[test]
    fn should_match_route_by_path_regexp() {
        // given:
        let routes = vec![
            sample_route_2(Box::new(AlwaysFirstUpstreamStrategy::build())),
            sample_route_3(Box::new(AlwaysFirstUpstreamStrategy::build())),
        ];
        let mut context = Context::build_from_routes(routes);

        // when:
        let upstream = context.upstream_lookup("uri10", "GET");

        // then:
        assert_eq!("upstream20".to_string(), upstream.unwrap());
    }

    #[test]
    fn should_match_route_by_method_regexp() {
        // given:
        let routes = vec![
            sample_route_2(Box::new(AlwaysFirstUpstreamStrategy::build())),
            sample_route_4(Box::new(AlwaysFirstUpstreamStrategy::build())),
        ];
        let mut context = Context::build_from_routes(routes);

        // when:
        let upstream = context.upstream_lookup("uri4", "PATCH");

        // then:
        assert_eq!("upstream10".to_string(), upstream.unwrap());
    }

    #[test]
    fn should_not_find_route_for_non_exact_match() {
        // given:
        let routes = vec![
            sample_route_5(Box::new(AlwaysFirstUpstreamStrategy::build())),
        ];
        let mut context = Context::build_from_routes(routes);

        // when:
        let upstream = context.upstream_lookup("uri5", "GET");

        // then:
        assert_eq!(upstream, None)
    }

    #[test]
    fn should_not_find_route_if_all_upstreams_are_disabled() {
        // given:
        let mut route = sample_route_1(Box::new(RoundRobinUpstreamStrategy::build(0)));
        for upstream in route.upstreams.iter_mut() {
            upstream.disable()
        }
        println!("{:?}", route);
        let routes = vec![route];
        let mut context = Context::build_from_routes(routes);

        // when:
        let upstream = context.upstream_lookup("uri1", "GET");

        // then:
        assert_eq!(None, upstream)
    }

    #[test]
    fn should_get_all_upstreams() {
        // given:
        let routes = vec![
            sample_route_1(Box::new(AlwaysFirstUpstreamStrategy::build())),
            sample_route_2(Box::new(AlwaysFirstUpstreamStrategy::build())),
        ];
        let context = Context::build_from_routes(routes);

        // when:
        let upstreams = context.get_upstreams();

        // then:
        assert_eq!(true, upstreams.contains(&String::from("upstream1")));
        assert_eq!(true, upstreams.contains(&String::from("upstream2")));
        assert_eq!(true, upstreams.contains(&String::from("upstream3")));
        assert_eq!(true, upstreams.contains(&String::from("upstream4")));
    }

    #[test]
    fn should_disable_upstream() {
        // given:
        let routes = vec![
            sample_route_5(Box::new(AlwaysFirstUpstreamStrategy::build())),
            sample_route_6(Box::new(AlwaysFirstUpstreamStrategy::build())),
        ];
        let mut context = Context::build_from_routes(routes);

        // when:
        context.disable_upstream_for_all_routes("upstream21");

        // then:
        for route in context.routes.iter() {
            for u in route.upstreams.iter() {
                if u.has_address("upstream21") {
                    assert_eq!(false, u.enabled);
                }
            }
        }
    }

    fn sample_route_1(strategy: Box<dyn UpstreamStrategy>) -> Route {
        Route::build(
            String::from("route1"),
            vec!(String::from("GET")),
            vec!(String::from("uri1"), String::from("uri2")),
            vec!(Upstream::build("upstream1"), Upstream::build("upstream2")),
            strategy,
        )
    }

    fn sample_route_2(strategy: Box<dyn UpstreamStrategy>) -> Route {
        Route::build(
            String::from("route2"),
            vec!(String::from("GET")),
            vec!(String::from("uri2"), String::from("uri3")),
            vec!(Upstream::build("upstream3"), Upstream::build("upstream4")),
            strategy,
        )
    }

    fn sample_route_3(strategy: Box<dyn UpstreamStrategy>) -> Route {
        Route::build(
            String::from("route3"),
            vec!(String::from("GET")),
            vec!(String::from("^uri.*$")),
            vec!(Upstream::build("upstream20"), Upstream::build("upstream21")),
            strategy,
        )
    }

    fn sample_route_4(strategy: Box<dyn UpstreamStrategy>) -> Route {
        Route::build(
            String::from("route4"),
            vec!(String::from("^.+$")),
            vec!(String::from("uri4")),
            vec!(Upstream::build("upstream10"), Upstream::build("upstream11")),
            strategy,
        )
    }

    fn sample_route_5(strategy: Box<dyn UpstreamStrategy>) -> Route {
        Route::build(
            String::from("route5"),
            vec!(String::from("GET")),
            vec!(String::from("uri")),
            vec!(Upstream::build("upstream20"), Upstream::build("upstream21")),
            strategy,
        )
    }

    fn sample_route_6(strategy: Box<dyn UpstreamStrategy>) -> Route {
        Route::build(
            String::from("route6"),
            vec!(String::from("GET")),
            vec!(String::from("uri2")),
            vec!(Upstream::build("upstream21"), Upstream::build("upstream22")),
            strategy,
        )
    }
}