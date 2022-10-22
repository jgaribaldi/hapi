use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use regex::Regex;

use crate::model::route::Route;
use crate::model::upstream::UpstreamAddress;

#[derive(Clone, Debug)]
pub struct Context {
    routes: Vec<Route>,
    routing_table: HashMap<(String, String), usize>, // (path, method) => route index
    upstreams: HashSet<UpstreamAddress>,
}

impl Context {
    pub fn build_from_routes(routes: Vec<Route>) -> Self {
        let mut routing_table: HashMap<(String, String), usize> = HashMap::new();
        let mut upstreams = HashSet::new();

        for (index, route) in routes.iter().enumerate() {
            for path in route.paths.iter() {
                for method in route.methods.iter() {
                    routing_table.insert((path.to_string(), method.to_string()), index);
                }
            }
            for upstream in route.upstreams.iter() {
                upstreams.insert(upstream.address.clone());
            }
        }

        Context {
            routes,
            routing_table,
            upstreams,
        }
    }

    pub fn build_empty() -> Self {
        Context {
            routes: Vec::new(),
            routing_table: HashMap::new(),
            upstreams: HashSet::new(),
        }
    }

    pub fn upstream_lookup(&mut self, path: &str, method: &str) -> Option<UpstreamAddress> {
        self.find_routing_table_index(path, method)
            .and_then(|index| self.routes.get_mut(index))
            .and_then(|route| route.next_available_upstream())
            .map(|upstream| upstream.address.clone())
    }

    pub fn disable_upstream_for_all_routes(&mut self, upstream: &UpstreamAddress) {
        for route in self.routes.iter_mut() {
            route.disable_upstream(upstream)
        }
    }

    pub fn enable_upstream_for_all_routes(&mut self, upstream: &UpstreamAddress) {
        for route in self.routes.iter_mut() {
            route.enable_upstream(upstream)
        }
    }

    /// Adds the given route to this context. Returns an optional array of upstream addresses
    /// indicating which upstream addresses were added to this context because they didn't exist
    /// before.
    pub fn add_route(&mut self, route: Route) -> Option<Vec<UpstreamAddress>> {
        let mut added_upstreams = Vec::new();
        for path in route.paths.iter() {
            for method in route.methods.iter() {
                self.routing_table
                    .insert((path.clone(), method.clone()), self.routes.len());
            }
        }
        for upstream in route.upstreams.iter() {
            if self.upstreams.insert(upstream.address.clone()) == true {
                added_upstreams.push(upstream.address.clone());
            }
        }
        self.routes.push(route);

        if added_upstreams.len() > 0 {
            Some(added_upstreams)
        } else {
            None
        }
    }

    /// Removes the given route from this context. Returns an optional array of upstream addresses
    /// indicating which upstream addresses were removed from this context, as no other route
    /// included such addresses.
    pub fn remove_route(&mut self, route: Route) -> Option<Vec<UpstreamAddress>> {
        let mut deleted_upstreams = Vec::new();
        let mut index_to_remove = self.routes.len() + 1;
        for (idx, r) in self.routes.iter().enumerate() {
            if *r == route {
                index_to_remove = idx;
                break;
            }
        }

        let mut keys_to_remove = Vec::new();
        for (key, value) in self.routing_table.iter() {
            if *value == index_to_remove {
                keys_to_remove.push(key.clone());
            }
        }

        for key_to_remove in keys_to_remove {
            self.routing_table.remove(&key_to_remove);
        }

        self.routes.remove(index_to_remove);

        for ups in route.upstreams {
            if self.upstreams.remove(&ups.address) == true {
                deleted_upstreams.push(ups.address.clone());
            }
        }

        if deleted_upstreams.len() > 0 {
            Some(deleted_upstreams)
        } else {
            None
        }
    }

    pub fn get_all_upstreams(&self) -> Vec<UpstreamAddress> {
        let mut result = Vec::new();
        for ups in self.upstreams.iter() {
            result.push(ups.clone());
        }
        result
    }

    pub fn get_all_routes(&self) -> Vec<&Route> {
        let mut result = Vec::new();
        for r in self.routes.iter() {
            result.push(r);
        }
        result
    }

    fn find_routing_table_index(&self, path: &str, method: &str) -> Option<usize> {
        // attempt exact match by (path, method) key
        let exact_key = (path.to_string(), method.to_string());

        self.routing_table
            .get(&exact_key)
            .map(|value| *value)
            .or_else(|| {
                let mut result = None;
                // attempt matching by regexp
                for (key, value) in self.routing_table.iter() {
                    let path_regexp =
                        Regex::new(wrap_string_in_regexp(key.0.as_str()).as_str()).unwrap();
                    let method_regexp =
                        Regex::new(wrap_string_in_regexp(key.1.as_str()).as_str()).unwrap();

                    if path_regexp.is_match(path) && method_regexp.is_match(method) {
                        result = Some(value.clone());
                        break;
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

#[cfg(test)]
mod tests {
    use crate::model::route::Route;
    use crate::model::upstream::{Upstream, UpstreamAddress, UpstreamStrategy};
    use crate::Context;

    #[test]
    fn should_create_context_from_routes() {
        // given:
        let routes = vec![
            sample_route_1(UpstreamStrategy::AlwaysFirst),
            sample_route_2(UpstreamStrategy::RoundRobin { index: 0 }),
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
            sample_route_1(UpstreamStrategy::AlwaysFirst),
            sample_route_2(UpstreamStrategy::RoundRobin { index: 0 }),
        ];
        let mut context = Context::build_from_routes(routes);

        // when:
        let upstream = context.upstream_lookup("uri1", "GET");

        // then:
        assert_eq!("upstream1", upstream.unwrap().to_string().as_str());
    }

    #[test]
    fn should_match_route_by_path_regexp() {
        // given:
        let routes = vec![
            sample_route_2(UpstreamStrategy::AlwaysFirst),
            sample_route_3(UpstreamStrategy::AlwaysFirst),
        ];
        let mut context = Context::build_from_routes(routes);

        // when:
        let upstream = context.upstream_lookup("uri10", "GET");

        // then:
        assert_eq!(
            "upstream20".to_string(),
            upstream.unwrap().to_string().as_str()
        );
    }

    #[test]
    fn should_match_route_by_method_regexp() {
        // given:
        let routes = vec![
            sample_route_2(UpstreamStrategy::AlwaysFirst),
            sample_route_4(UpstreamStrategy::AlwaysFirst),
        ];
        let mut context = Context::build_from_routes(routes);

        // when:
        let upstream = context.upstream_lookup("uri4", "PATCH");

        // then:
        assert_eq!(
            "upstream10".to_string(),
            upstream.unwrap().to_string().as_str()
        );
    }

    #[test]
    fn should_not_find_route_for_non_exact_match() {
        // given:
        let routes = vec![sample_route_5(UpstreamStrategy::AlwaysFirst)];
        let mut context = Context::build_from_routes(routes);

        // when:
        let upstream = context.upstream_lookup("uri5", "GET");

        // then:
        assert_eq!(upstream, None)
    }

    #[test]
    fn should_not_find_route_if_all_upstreams_are_disabled() {
        // given:
        let mut route = sample_route_1(UpstreamStrategy::RoundRobin { index: 0 });
        for upstream in route.upstreams.iter_mut() {
            upstream.disable()
        }
        let routes = vec![route];
        let mut context = Context::build_from_routes(routes);

        // when:
        let upstream = context.upstream_lookup("uri1", "GET");

        // then:
        assert_eq!(None, upstream)
    }

    #[test]
    fn should_disable_upstream() {
        // given:
        let routes = vec![
            sample_route_5(UpstreamStrategy::AlwaysFirst),
            sample_route_6(UpstreamStrategy::AlwaysFirst),
        ];
        let mut context = Context::build_from_routes(routes);
        let ups_addr = UpstreamAddress::FQDN(String::from("upstream21"));

        // when:
        context.disable_upstream_for_all_routes(&ups_addr);

        // then:
        for route in context.routes.iter() {
            for u in route.upstreams.iter() {
                if u.address == ups_addr {
                    assert_eq!(false, u.enabled);
                }
            }
        }
    }

    #[test]
    fn should_enable_upstream() {
        // given:
        let routes = vec![
            sample_route_7(UpstreamStrategy::AlwaysFirst),
            sample_route_8(UpstreamStrategy::AlwaysFirst),
        ];
        let mut context = Context::build_from_routes(routes);
        let ups_addr = UpstreamAddress::FQDN(String::from("upstream21"));

        // when:
        context.enable_upstream_for_all_routes(&ups_addr);

        // then:
        for route in context.routes.iter() {
            for u in route.upstreams.iter() {
                if u.address == ups_addr {
                    assert_eq!(true, u.enabled);
                }
            }
        }
    }

    #[test]
    fn should_add_route() {
        // given:
        let route1 = sample_route_1(UpstreamStrategy::AlwaysFirst);
        let route2 = sample_route_2(UpstreamStrategy::AlwaysFirst);
        let mut context = Context::build_from_routes(vec![route1]);

        // when:
        let added_routes = context.add_route(route2).unwrap();

        // then:
        assert_eq!(2, context.routes.len());
        assert_eq!(3, context.routing_table.len());
        assert_eq!(
            UpstreamAddress::FQDN("upstream3".to_string()),
            added_routes[0]
        );
        assert_eq!(
            UpstreamAddress::FQDN("upstream4".to_string()),
            added_routes[1]
        );
    }

    #[test]
    fn should_remove_route() {
        // given:
        let route1 = sample_route_1(UpstreamStrategy::AlwaysFirst);
        let route2 = sample_route_2(UpstreamStrategy::AlwaysFirst);
        let route3 = sample_route_1(UpstreamStrategy::AlwaysFirst);
        let mut context = Context::build_from_routes(vec![route1, route2]);

        // when:
        let removed_routes = context.remove_route(route3).unwrap();

        // then:
        assert_eq!(1, context.routes.len());
        assert_eq!(2, context.routing_table.len());
        assert_eq!(
            UpstreamAddress::FQDN("upstream1".to_string()),
            removed_routes[0]
        );
        assert_eq!(
            UpstreamAddress::FQDN("upstream2".to_string()),
            removed_routes[1]
        );
    }

    fn sample_route_1(strategy: UpstreamStrategy) -> Route {
        Route::build(
            String::from("route1"),
            vec![String::from("GET")],
            vec![String::from("uri1"), String::from("uri2")],
            vec![
                Upstream::build_from_fqdn("upstream1"),
                Upstream::build_from_fqdn("upstream2"),
            ],
            strategy,
        )
    }

    fn sample_route_2(strategy: UpstreamStrategy) -> Route {
        Route::build(
            String::from("route2"),
            vec![String::from("GET")],
            vec![String::from("uri2"), String::from("uri3")],
            vec![
                Upstream::build_from_fqdn("upstream3"),
                Upstream::build_from_fqdn("upstream4"),
            ],
            strategy,
        )
    }

    fn sample_route_3(strategy: UpstreamStrategy) -> Route {
        Route::build(
            String::from("route3"),
            vec![String::from("GET")],
            vec![String::from("^uri.*$")],
            vec![
                Upstream::build_from_fqdn("upstream20"),
                Upstream::build_from_fqdn("upstream21"),
            ],
            strategy,
        )
    }

    fn sample_route_4(strategy: UpstreamStrategy) -> Route {
        Route::build(
            String::from("route4"),
            vec![String::from("^.+$")],
            vec![String::from("uri4")],
            vec![
                Upstream::build_from_fqdn("upstream10"),
                Upstream::build_from_fqdn("upstream11"),
            ],
            strategy,
        )
    }

    fn sample_route_5(strategy: UpstreamStrategy) -> Route {
        Route::build(
            String::from("route5"),
            vec![String::from("GET")],
            vec![String::from("uri")],
            vec![
                Upstream::build_from_fqdn("upstream20"),
                Upstream::build_from_fqdn("upstream21"),
            ],
            strategy,
        )
    }

    fn sample_route_6(strategy: UpstreamStrategy) -> Route {
        Route::build(
            String::from("route6"),
            vec![String::from("GET")],
            vec![String::from("uri2")],
            vec![
                Upstream::build_from_fqdn("upstream21"),
                Upstream::build_from_fqdn("upstream22"),
            ],
            strategy,
        )
    }

    fn sample_route_7(strategy: UpstreamStrategy) -> Route {
        let upstream1 = Upstream::build_from_fqdn("upstream20");
        let mut upstream2 = Upstream::build_from_fqdn("upstream21");
        upstream2.enabled = false;
        Route::build(
            String::from("route7"),
            vec![String::from("GET")],
            vec![String::from("uri")],
            vec![upstream1, upstream2],
            strategy,
        )
    }

    fn sample_route_8(strategy: UpstreamStrategy) -> Route {
        let mut upstream1 = Upstream::build_from_fqdn("upstream21");
        upstream1.enabled = false;
        let upstream2 = Upstream::build_from_fqdn("upstream22");
        Route::build(
            String::from("route8"),
            vec![String::from("GET")],
            vec![String::from("uri2")],
            vec![upstream1, upstream2],
            strategy,
        )
    }
}
