pub(crate) mod context {
    use std::collections::{HashMap, HashSet};
    use regex::Regex;
    use crate::errors::HapiError;
    use crate::modules::core::route::Route;
    use crate::modules::core::upstream::{Upstream, UpstreamAddress};

    #[derive(Clone, Debug)]
    pub struct Context {
        routes: Vec<Route>,
        routing_table: HashMap<(String, String), usize>, // (path, method) => route index
        upstreams: HashSet<Upstream>,
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

        pub fn upstream_lookup(&mut self, path: &str, method: &str) -> Result<Option<UpstreamAddress>, CoreError> {
            let upstream_address = self.find_routing_table_index(path, method)
                .and_then(|index| self.routes.get_mut(index))
                .and_then(|route| route.next_available_upstream())
                .map(|upstream| upstream.address.clone());
            Ok(upstream_address)
        }

        pub fn disable_upstream_for_all_routes(&mut self, upstream: &UpstreamAddress) -> Result<(), CoreError> {
            for route in self.routes.iter_mut() {
                route.disable_upstream(upstream)
            }
            Ok(())
        }

        pub fn enable_upstream_for_all_routes(&mut self, upstream: &UpstreamAddress) -> Result<(), CoreError> {
            for route in self.routes.iter_mut() {
                route.enable_upstream(upstream)
            }
            Ok(())
        }

        /// Adds the given route to this context
        /// Returns an error if the given route already exists in the context
        pub fn add_route(&mut self, route: Route) -> Result<(), CoreError> {
            if !self.route_index.contains_key(&route.id) {
                self.do_add_route(route);
                Ok(())
            } else {
                Err(CoreError::RouteAlreadyExists)
            }
        }

        /// Removes the given route from this context
        /// Returns an error if the route id doesn't exist in the context
        pub fn remove_route(&mut self, route_id: &str) -> Result<Route, CoreError> {
            match self.route_index.get(route_id) {
                Some(route_index) => {
                    let removed_route = self.do_remove_route(*route_index);
                    Ok(removed_route)
                }
                None => Err(CoreError::RouteNotExists),
            }
        }

        pub fn get_all_upstreams(&self) -> Result<Vec<UpstreamAddress>, CoreError> {
            let mut result = Vec::new();
            for ups in self.upstreams.iter() {
                result.push(ups.address.clone());
            }
            Ok(result)
        }

        pub fn get_all_routes(&self) -> Result<Vec<&Route>, CoreError> {
            let mut result = Vec::new();
            for r in self.routes.iter() {
                result.push(r);
            }
            Ok(result)
        }

        pub fn get_route_by_id(&self, route_id: &str) -> Result<Option<&Route>, CoreError> {
            let route = self.route_index
                .get(route_id)
                .and_then(|index| self.routes.get(*index));
            Ok(route)
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
            self.routes.push(route);

            self.rebuild_routing_table();
            self.rebuild_upstreams();
            self.rebuild_route_index();
        }

        fn do_remove_route(&mut self, route_index: usize) -> Route {
            let removed_route = self.routes.remove(route_index);

            self.rebuild_routing_table();
            self.rebuild_upstreams();
            self.rebuild_route_index();
            removed_route
        }

        fn rebuild_routing_table(&mut self) {
            self.routing_table.clear();

            for (index, route) in self.routes.iter().enumerate() {
                for path in route.paths.iter() {
                    for method in route.methods.iter() {
                        self.routing_table
                            .insert((path.clone(), method.clone()), index);
                    }
                }
            }
        }

        fn rebuild_upstreams(&mut self) {
            self.upstreams.clear();

            for route in self.routes.iter() {
                for upstream in route.upstreams.iter() {
                    self.upstreams.insert(upstream.clone());
                }
            }
        }

        fn rebuild_route_index(&mut self) {
            self.route_index.clear();

            for (idx, route) in self.routes.iter().enumerate() {
                self.route_index.insert(route.id.clone(), idx);
            }
        }
    }

    #[derive(Clone, Debug)]
    pub(crate) enum CoreError {
        RouteAlreadyExists,
        RouteNotExists,
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
        use crate::modules::core::context::Context;
        use crate::modules::core::route::Route;
        use crate::modules::core::upstream::{Upstream, UpstreamAddress, UpstreamStrategy};

        #[test]
        fn should_perform_upstream_lookup() {
            // given:
            let mut context = Context::build_empty();
            context
                .add_route(sample_route_1(UpstreamStrategy::AlwaysFirst))
                .unwrap();
            context
                .add_route(sample_route_2(UpstreamStrategy::RoundRobin { index: 0 }))
                .unwrap();

            // when:
            let upstream = context.upstream_lookup("uri1", "GET").unwrap();

            // then:
            assert_eq!("upstream1", upstream.unwrap().to_string().as_str());
        }

        #[test]
        fn should_match_route_by_path_regexp() {
            // given:
            let mut context = Context::build_empty();
            context
                .add_route(sample_route_2(UpstreamStrategy::AlwaysFirst))
                .unwrap();
            context
                .add_route(sample_route_3(UpstreamStrategy::AlwaysFirst))
                .unwrap();

            // when:
            let upstream = context.upstream_lookup("uri10", "GET").unwrap();

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
            context
                .add_route(sample_route_2(UpstreamStrategy::AlwaysFirst))
                .unwrap();
            context
                .add_route(sample_route_4(UpstreamStrategy::AlwaysFirst))
                .unwrap();

            // when:
            let upstream = context.upstream_lookup("uri4", "PATCH").unwrap();

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
            context
                .add_route(sample_route_5(UpstreamStrategy::AlwaysFirst))
                .unwrap();

            // when:
            let upstream = context.upstream_lookup("uri5", "GET").unwrap();

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
            let upstream = context.upstream_lookup("uri1", "GET").unwrap();

            // then:
            assert_eq!(None, upstream)
        }

        #[test]
        fn should_disable_upstream() {
            // given:
            let mut context = Context::build_empty();
            context
                .add_route(sample_route_5(UpstreamStrategy::AlwaysFirst))
                .unwrap();
            context
                .add_route(sample_route_6(UpstreamStrategy::AlwaysFirst))
                .unwrap();
            let ups_addr = UpstreamAddress::FQDN(String::from("upstream21"));

            // when:
            context.disable_upstream_for_all_routes(&ups_addr).unwrap();

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
            context
                .add_route(sample_route_7(UpstreamStrategy::AlwaysFirst))
                .unwrap();
            context
                .add_route(sample_route_8(UpstreamStrategy::AlwaysFirst))
                .unwrap();
            let ups_addr = UpstreamAddress::FQDN(String::from("upstream21"));

            // when:
            context.enable_upstream_for_all_routes(&ups_addr).unwrap();

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

        #[test]
        fn should_remove_routes_in_reverse_order() {
            // given:
            let route1 = sample_route_1(UpstreamStrategy::AlwaysFirst);
            let route2 = sample_route_2(UpstreamStrategy::AlwaysFirst);
            let route_id1_to_remove = route1.id.clone();
            let route_id2_to_remove = route2.id.clone();
            let mut context = Context::build_empty();
            context.add_route(route1).unwrap();
            context.add_route(route2).unwrap();

            // when:
            let remove_result1 = context.remove_route(route_id1_to_remove.as_str());
            let remove_result2 = context.remove_route(route_id2_to_remove.as_str());

            // then:
            assert_eq!(true, remove_result1.is_ok());
            assert_eq!(true, remove_result2.is_ok());
            assert_eq!(0, context.routes.len());
            assert_eq!(0, context.route_index.len());
            assert_eq!(0, context.routing_table.len());
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
}

pub(crate) mod route {
    use crate::modules::core::upstream::{Upstream, UpstreamAddress, UpstreamStrategy};

    #[derive(Clone, Debug, PartialEq)]
    pub struct Route {
        pub id: String,
        pub name: String,
        pub methods: Vec<String>,
        pub paths: Vec<String>,
        pub upstreams: Vec<Upstream>,
        pub strategy: UpstreamStrategy,
    }

    impl Route {
        pub fn build(
            id: String,
            name: String,
            methods: Vec<String>,
            paths: Vec<String>,
            upstreams: Vec<Upstream>,
            strategy: UpstreamStrategy,
        ) -> Self {
            Route {
                id,
                name,
                methods,
                paths,
                upstreams,
                strategy,
            }
        }

        pub fn enable_upstream(&mut self, upstream: &UpstreamAddress) {
            for u in self.upstreams.iter_mut() {
                if u.address == *upstream && !u.enabled {
                    u.enable()
                }
            }
        }

        pub fn disable_upstream(&mut self, upstream: &UpstreamAddress) {
            for u in self.upstreams.iter_mut() {
                if u.address == *upstream && u.enabled {
                    u.disable()
                }
            }
        }

        pub fn next_available_upstream(&mut self) -> Option<&Upstream> {
            let enabled_upstreams: Vec<&Upstream> =
                self.upstreams.iter().filter(|u| u.enabled).collect();

            if enabled_upstreams.len() > 0 {
                let next_upstream_index = self.strategy.next(enabled_upstreams.as_slice());
                Some(enabled_upstreams[next_upstream_index])
            } else {
                None
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::modules::core::route::Route;
        use crate::modules::core::upstream::{Upstream, UpstreamAddress, UpstreamStrategy};

        #[test]
        fn should_enable_upstream() {
            // given:
            let mut route = sample_route();
            let ups_addr = UpstreamAddress::FQDN(String::from("upstream2"));

            // when:
            route.enable_upstream(&ups_addr);

            // then:
            let u = get_upstream_by_address(&route, &ups_addr).unwrap();
            assert_eq!(true, u.enabled)
        }

        #[test]
        fn should_disable_upstream() {
            // given:
            let mut route = sample_route();
            let ups_addr = UpstreamAddress::FQDN(String::from("upstream1"));

            // when:
            route.disable_upstream(&ups_addr);

            // then:
            let u = get_upstream_by_address(&route, &ups_addr).unwrap();
            assert_eq!(false, u.enabled)
        }

        #[test]
        fn should_return_next_available_upstream() {
            // given:
            let mut route = sample_route();

            // when:
            let upstream1 = route.next_available_upstream().unwrap().clone();
            let upstream2 = route.next_available_upstream().unwrap().clone();
            let upstream3 = route.next_available_upstream().unwrap().clone();
            let upstream4 = route.next_available_upstream().unwrap().clone();

            // then:
            assert_eq!(String::from("upstream1"), upstream1.address.to_string());
            assert_eq!(String::from("upstream3"), upstream2.address.to_string());
            assert_eq!(String::from("upstream1"), upstream3.address.to_string());
            assert_eq!(String::from("upstream3"), upstream4.address.to_string());
        }

        fn sample_route() -> Route {
            let strategy = UpstreamStrategy::RoundRobin { index: 0 };
            let upstream1 = Upstream::build_from_fqdn("upstream1");
            let mut upstream2 = Upstream::build_from_fqdn("upstream2");
            upstream2.disable();
            let upstream3 = Upstream::build_from_fqdn("upstream3");

            Route::build(
                String::from("id1"),
                String::from("route1"),
                vec![String::from("GET")],
                vec![String::from("uri1"), String::from("uri2")],
                vec![upstream1, upstream2, upstream3],
                strategy,
            )
        }

        fn get_upstream_by_address(route: &Route, address: &UpstreamAddress) -> Option<Upstream> {
            for u in route.upstreams.iter() {
                if u.address == *address {
                    return Some(u.clone());
                }
            }
            return None;
        }
    }
}

pub(crate) mod upstream {
    use std::fmt::{Display, Formatter};

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

    impl Display for UpstreamAddress {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
        use crate::modules::core::upstream::{Upstream, UpstreamStrategy};

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
}