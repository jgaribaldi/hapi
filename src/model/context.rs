use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use crate::HapiError;
use regex::Regex;

use crate::model::route::Route;
use crate::model::upstream::UpstreamAddress;

#[derive(Clone, Debug)]
pub struct Context {
    routes: Vec<Route>,
    routing_table: HashMap<(String, String), usize>, // (path, method) => route index
    upstreams: HashSet<UpstreamAddress>,
    route_index: HashMap<String, usize>, // route id => route index
}

impl Context {

    pub fn build_empty() -> Self {
        Context {
            routes: Vec::new(),
            routing_table: HashMap::new(),
            upstreams: HashSet::new(),
            route_index: HashMap::new(),
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
    /// before, or error if the given route already exists in the context
    pub fn add_route(&mut self, route: Route) -> Result<(), HapiError> {
        if !self.route_index.contains_key(&route.id) {
            self.do_add_route(route);
            Ok(())
        } else {
            Err(HapiError::RouteAlreadyExists)
        }
    }

    /// Removes the given route from this context. Returns an optional array of upstream addresses
    /// indicating which upstream addresses were removed from this context, as no other route
    /// included such addresses, or error if the route id doesn't exist in the context
    pub fn remove_route(&mut self, route_id: &str) -> Result<(), HapiError> {
        if self.route_index.contains_key(route_id) {
            self.do_remove_route(route_id);
            Ok(())
        } else {
            Err(HapiError::RouteNotExists)
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

    pub fn get_route_by_id(&self, route_id: &str) -> Option<&Route> {
        self.route_index.get(route_id)
            .and_then(|index| self.routes.get(*index))
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

    fn do_add_route(&mut self, route: Route) {
        self.route_index.insert(route.id.clone(), self.routes.len());
        self.routes.push(route);

        self.rebuild_routing_table();
        self.rebuild_upstreams()
    }

    fn do_remove_route(&mut self, route_id: &str) {
        let index_to_remove = self.route_index.remove(route_id).unwrap();
        self.routes.remove(index_to_remove);

        self.rebuild_routing_table();
        self.rebuild_upstreams()
    }

    fn rebuild_routing_table(&mut self) {
        self.routing_table.clear();

        for (index, route) in self.routes.iter().enumerate() {
            for path in route.paths.iter() {
                for method in route.methods.iter() {
                    self.routing_table.insert((path.clone(), method.clone()), index);
                }
            }
        }
    }

    fn rebuild_upstreams(&mut self) {
        self.upstreams.clear();

        for route in self.routes.iter() {
            for upstream in route.upstreams.iter() {
                self.upstreams.insert(upstream.address.clone());
            }
        }
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
    fn should_perform_upstream_lookup() {
        // given:
        let mut context = Context::build_empty();
        context.add_route(sample_route_1(UpstreamStrategy::AlwaysFirst)).unwrap();
        context.add_route(sample_route_2(UpstreamStrategy::RoundRobin { index: 0 })).unwrap();

        // when:
        let upstream = context.upstream_lookup("uri1", "GET");

        // then:
        assert_eq!("upstream1", upstream.unwrap().to_string().as_str());
    }

    #[test]
    fn should_match_route_by_path_regexp() {
        // given:
        let mut context = Context::build_empty();
        context.add_route(sample_route_2(UpstreamStrategy::AlwaysFirst)).unwrap();
        context.add_route(sample_route_3(UpstreamStrategy::AlwaysFirst)).unwrap();

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
        let mut context = Context::build_empty();
        context.add_route(sample_route_2(UpstreamStrategy::AlwaysFirst)).unwrap();
        context.add_route(sample_route_4(UpstreamStrategy::AlwaysFirst)).unwrap();

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
        let mut context = Context::build_empty();
        context.add_route(sample_route_5(UpstreamStrategy::AlwaysFirst)).unwrap();

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
        let mut context = Context::build_empty();
        context.add_route(route).unwrap();

        // when:
        let upstream = context.upstream_lookup("uri1", "GET");

        // then:
        assert_eq!(None, upstream)
    }

    #[test]
    fn should_disable_upstream() {
        // given:
        let mut context = Context::build_empty();
        context.add_route(sample_route_5(UpstreamStrategy::AlwaysFirst)).unwrap();
        context.add_route(sample_route_6(UpstreamStrategy::AlwaysFirst)).unwrap();
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
        let mut context = Context::build_empty();
        context.add_route(sample_route_7(UpstreamStrategy::AlwaysFirst)).unwrap();
        context.add_route(sample_route_8(UpstreamStrategy::AlwaysFirst)).unwrap();
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
        let mut context = Context::build_empty();

        // when:
        let add_route_result_1 = context.add_route(route1);
        let add_route_result_2 = context.add_route(route2);

        // then:
        assert_eq!(true, add_route_result_1.is_ok());
        assert_eq!(true, add_route_result_2.is_ok());
        assert_eq!(2, context.routes.len());
        assert_eq!(3, context.routing_table.len());
        assert_eq!(2, context.route_index.len());
    }

    #[test]
    fn should_not_add_route_if_it_exists() {
        // given:
        let route1 = sample_route_1(UpstreamStrategy::AlwaysFirst);
        let route2 = route1.clone();
        let mut context = Context::build_empty();
        context.add_route(route1).unwrap();

        // when:
        let add_result = context.add_route(route2);

        // then:
        assert_eq!(true, add_result.is_err());
        assert_eq!(1, context.routes.len());
        assert_eq!(2, context.routing_table.len());
        assert_eq!(1, context.route_index.len());
    }

    #[test]
    fn should_remove_route() {
        // given:
        let route1 = sample_route_1(UpstreamStrategy::AlwaysFirst);
        let route2 = sample_route_2(UpstreamStrategy::AlwaysFirst);
        let route_id_to_remove = route1.id.clone();
        let mut context = Context::build_empty();
        context.add_route(route1).unwrap();
        context.add_route(route2).unwrap();

        // when:
        let remove_result = context.remove_route(route_id_to_remove.as_str());

        // then:
        assert_eq!(true, remove_result.is_ok());
        assert_eq!(1, context.routes.len());
        assert_eq!(2, context.routing_table.len());
        assert_eq!(1, context.route_index.len());
    }

    #[test]
    fn should_not_remove_route_if_not_exists() {
        // given:
        let route1 = sample_route_1(UpstreamStrategy::AlwaysFirst);
        let route2 = sample_route_2(UpstreamStrategy::AlwaysFirst);
        let mut context = Context::build_empty();
        context.add_route(route1).unwrap();

        // when:
        let remove_route_result = context.remove_route(route2.id.as_str());

        // then:
        assert_eq!(true, remove_route_result.is_err());
        assert_eq!(1, context.routes.len());
        assert_eq!(1, context.route_index.len());
    }

    fn sample_route_1(strategy: UpstreamStrategy) -> Route {
        Route::build(
            String::from("id1"),
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
            String::from("id2"),
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
            String::from("id3"),
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
            String::from("id4"),
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
            String::from("id5"),
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
            String::from("id6"),
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
            String::from("id7"),
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
            String::from("id8"),
            String::from("route8"),
            vec![String::from("GET")],
            vec![String::from("uri2")],
            vec![upstream1, upstream2],
            strategy,
        )
    }
}
