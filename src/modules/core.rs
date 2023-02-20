pub(crate) mod context {
    use crate::modules::core::route::Route;
    use crate::modules::core::upstream::{Upstream, UpstreamAddress};
    use regex::Regex;
    use std::collections::{HashMap, HashSet};

    #[derive(Clone, Debug)]
    pub(crate) struct Context {
        routes: Vec<Route>,
        routing_table: HashMap<(String, String), usize>, // (path, method) => route index
        route_index: HashMap<String, usize>, // route id => route index
    }

    impl Context {
        pub fn build_empty() -> Self {
            Context {
                routes: Vec::new(),
                routing_table: HashMap::new(),
                route_index: HashMap::new(),
            }
        }

        /// Given a path and a method, attempts to get a proper route and returns an upstream that
        /// is capable of handling the request.
        /// First, try to get the route by matching exactly by (path, method). If that fails, try
        /// to match by wrapping the given path and method using regular expressions
        pub fn upstream_lookup(
            &mut self,
            path: &str,
            method: &str
        ) -> Result<Option<&Upstream>, CoreError> {
            let result = self.find_route_index(path, method)?
                .and_then(move |route_index| self.routes.get_mut(route_index))
                .and_then(|route| route.strategy.next());

            Ok(result)
        }

        /// Disables the given upstream from all the routes that contain it, so new requests don't
        /// get routed to that upstream
        pub fn disable_upstream_for_all_routes(
            &mut self,
            upstream: &UpstreamAddress,
        ) -> Result<(), CoreError> {
            for route in self.routes.iter_mut() {
                route.strategy.disable_upstream(upstream)
            }
            Ok(())
        }

        /// Enables the given upstream in all the routes that contain it, so new requests can be
        /// routed to that upstream
        pub fn enable_upstream_for_all_routes(
            &mut self,
            upstream: &UpstreamAddress,
        ) -> Result<(), CoreError> {
            for route in self.routes.iter_mut() {
                route.strategy.enable_upstream(upstream)
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

        pub fn get_all_upstreams(&self) -> Result<Vec<&Upstream>, CoreError> {
            let mut temp = HashSet::new();

            for route in self.routes.iter() {
                let ups = route.strategy.get_upstreams();
                for u in ups {
                    temp.insert(u);
                }
            }

            let result = temp.into_iter().collect();
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
            let route = self
                .route_index
                .get(route_id)
                .and_then(|index| self.routes.get(*index));
            Ok(route)
        }

        fn find_route_index(
            &self,
            path: &str,
            method: &str
        ) -> Result<Option<usize>, CoreError> {
            let key = (path.to_string(), method.to_string());
            let route_index = self.routing_table
                .get(&key)
                .map(|value| *value)
                .or_else(|| { self.match_route_index(path, method).ok()? });

            Ok(route_index)
        }

        fn match_route_index(
            &self,
            path: &str,
            method: &str
        ) -> Result<Option<usize>, regex::Error> {
            let mut result = Ok(None);

            for (key, value) in self.routing_table.iter() {
                let k = key.clone();
                let path_regexp = Regex::new(regexp_for(k.0).as_str())?;
                let method_regexp = Regex::new(regexp_for(k.1).as_str())?;

                if path_regexp.is_match(path) && method_regexp.is_match(method) {
                    result = Ok(Some(*value));
                    break;
                }
            }
            result
        }

        fn do_add_route(&mut self, route: Route) {
            self.routes.push(route);

            self.rebuild_routing_table();
            self.rebuild_route_index();
        }

        fn do_remove_route(&mut self, route_index: usize) -> Route {
            let removed_route = self.routes.remove(route_index);

            self.rebuild_routing_table();
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
        CannotCreateRegexp,
    }

    fn regexp_for(string: String) -> String {
        let mut result = String::new();
        result.push_str("^");
        result.push_str(string.as_str());
        result.push_str("$");
        result
    }

    #[cfg(test)]
    mod tests {
        use crate::modules::core::context::Context;
        use crate::modules::core::route::Route;
        use crate::modules::core::upstream::{Upstream, UpstreamAddress};
        use crate::modules::core::upstream::UpstreamStrategy::{AlwaysFirst, RoundRobin};

        #[test]
        fn should_perform_upstream_lookup() {
            // given:
            let mut context = Context::build_empty();
            context.add_route(sample_route_1_af()).unwrap();
            context.add_route(sample_route_2_rr()).unwrap();

            // when:
            let upstream = context.upstream_lookup("uri1", "GET").unwrap().unwrap();

            // then:
            assert_eq!("upstream1", upstream.address.to_string().as_str());
        }

        #[test]
        fn should_match_route_by_path_regexp() {
            // given:
            let mut context = Context::build_empty();
            context.add_route(sample_route_2_af()).unwrap();
            context.add_route(sample_route_3_af()).unwrap();

            // when:
            let upstream = context.upstream_lookup("uri10", "GET").unwrap().unwrap();

            // then:
            assert_eq!(
                "upstream20".to_string(),
                upstream.address.to_string().as_str()
            );
        }

        #[test]
        fn should_match_route_by_method_regexp() {
            // given:
            let mut context = Context::build_empty();
            context.add_route(sample_route_2_af()).unwrap();
            context.add_route(sample_route_4_af()).unwrap();

            // when:
            let upstream = context.upstream_lookup("uri4", "PATCH").unwrap().unwrap();

            // then:
            assert_eq!(
                "upstream10".to_string(),
                upstream.address.to_string().as_str()
            );
        }

        #[test]
        fn should_not_find_route_for_non_exact_match() {
            // given:
            let mut context = Context::build_empty();
            context.add_route(sample_route_5_af()).unwrap();

            // when:
            let upstream = context.upstream_lookup("uri5", "GET").unwrap();

            // then:
            assert_eq!(upstream, None)
        }

        #[test]
        fn should_not_find_route_if_all_upstreams_are_disabled() {
            // given:
            let mut route = sample_route_1_rr();
            let addresses: Vec<UpstreamAddress> = route.strategy.get_upstreams().iter().map(|u| u.address.clone()).collect();
            for a in addresses.iter() {
                route.strategy.disable_upstream(a);
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
            context.add_route(sample_route_5_af()).unwrap();
            context.add_route(sample_route_6_af()).unwrap();
            let ups_addr = UpstreamAddress::FQDN(String::from("upstream21"));

            // when:
            context.disable_upstream_for_all_routes(&ups_addr).unwrap();

            // then:
            for route in context.routes.iter() {
                for u in route.strategy.get_upstreams().iter() {
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
            context.add_route(sample_route_7_af()).unwrap();
            context.add_route(sample_route_8_af()).unwrap();
            let ups_addr = UpstreamAddress::FQDN(String::from("upstream21"));

            // when:
            context.enable_upstream_for_all_routes(&ups_addr).unwrap();

            // then:
            for route in context.routes.iter() {
                for u in route.strategy.get_upstreams().iter() {
                    if u.address == ups_addr {
                        assert_eq!(true, u.enabled);
                    }
                }
            }
        }

        #[test]
        fn should_add_route() {
            // given:
            let route1 = sample_route_1_af();
            let route2 = sample_route_2_af();
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
            let route1 = sample_route_1_af();
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
            let route1 = sample_route_1_af();
            let route2 = sample_route_2_af();
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
            let route1 = sample_route_1_af();
            let route2 = sample_route_2_af();
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
            let route1 = sample_route_1_af();
            let route2 = sample_route_2_af();
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

        fn sample_route_1_af() -> Route {
            let upstreams = vec![
                Upstream::build_from_fqdn("upstream1"),
                Upstream::build_from_fqdn("upstream2"),
            ];
            let strategy = AlwaysFirst { upstreams };
            Route::build(
                String::from("id1"),
                String::from("route1"),
                vec![String::from("GET")],
                vec![String::from("uri1"), String::from("uri2")],
                strategy,
            )
        }

        fn sample_route_1_rr() -> Route {
            let upstreams = vec![
                Upstream::build_from_fqdn("upstream1"),
                Upstream::build_from_fqdn("upstream2"),
            ];
            let strategy = RoundRobin { upstreams, next_index: 0 };
            Route::build(
                String::from("id1"),
                String::from("route1"),
                vec![String::from("GET")],
                vec![String::from("uri1"), String::from("uri2")],
                strategy,
            )
        }

        fn sample_route_2_af() -> Route {
            let upstreams = vec![
                Upstream::build_from_fqdn("upstream3"),
                Upstream::build_from_fqdn("upstream4"),
            ];
            let strategy = AlwaysFirst { upstreams };
            Route::build(
                String::from("id2"),
                String::from("route2"),
                vec![String::from("GET")],
                vec![String::from("uri2"), String::from("uri3")],
                strategy,
            )
        }

        fn sample_route_2_rr() -> Route {
            let upstreams = vec![
                Upstream::build_from_fqdn("upstream3"),
                Upstream::build_from_fqdn("upstream4"),
            ];
            let strategy = RoundRobin { upstreams, next_index: 0 };
            Route::build(
                String::from("id2"),
                String::from("route2"),
                vec![String::from("GET")],
                vec![String::from("uri2"), String::from("uri3")],
                strategy,
            )
        }

        fn sample_route_3_af() -> Route {
            let upstreams = vec![
                Upstream::build_from_fqdn("upstream20"),
                Upstream::build_from_fqdn("upstream21"),
            ];
            let strategy = AlwaysFirst { upstreams };
            Route::build(
                String::from("id3"),
                String::from("route3"),
                vec![String::from("GET")],
                vec![String::from("^uri.*$")],
                strategy,
            )
        }

        fn sample_route_4_af() -> Route {
            let upstreams = vec![
                Upstream::build_from_fqdn("upstream10"),
                Upstream::build_from_fqdn("upstream11"),
            ];
            let strategy = AlwaysFirst { upstreams };
            Route::build(
                String::from("id4"),
                String::from("route4"),
                vec![String::from("^.+$")],
                vec![String::from("uri4")],
                strategy,
            )
        }

        fn sample_route_5_af() -> Route {
            let upstreams = vec![
                Upstream::build_from_fqdn("upstream20"),
                Upstream::build_from_fqdn("upstream21"),
            ];
            let strategy = AlwaysFirst { upstreams };
            Route::build(
                String::from("id5"),
                String::from("route5"),
                vec![String::from("GET")],
                vec![String::from("uri")],
                strategy,
            )
        }

        fn sample_route_6_af() -> Route {
            let upstreams = vec![
                Upstream::build_from_fqdn("upstream21"),
                Upstream::build_from_fqdn("upstream22"),
            ];
            let strategy = AlwaysFirst { upstreams };
            Route::build(
                String::from("id6"),
                String::from("route6"),
                vec![String::from("GET")],
                vec![String::from("uri2")],
                strategy,
            )
        }

        fn sample_route_7_af() -> Route {
            let upstream1 = Upstream::build_from_fqdn("upstream20");
            let mut upstream2 = Upstream::build_from_fqdn("upstream21");
            upstream2.enabled = false;
            let upstreams = vec![upstream1, upstream2];
            let strategy = AlwaysFirst { upstreams };
            Route::build(
                String::from("id7"),
                String::from("route7"),
                vec![String::from("GET")],
                vec![String::from("uri")],
                strategy,
            )
        }

        fn sample_route_8_af() -> Route {
            let mut upstream1 = Upstream::build_from_fqdn("upstream21");
            upstream1.enabled = false;
            let upstream2 = Upstream::build_from_fqdn("upstream22");
            let upstreams = vec![upstream1, upstream2];
            let strategy = AlwaysFirst { upstreams };
            Route::build(
                String::from("id8"),
                String::from("route8"),
                vec![String::from("GET")],
                vec![String::from("uri2")],
                strategy,
            )
        }
    }
}

pub(crate) mod route {
    use crate::modules::core::upstream::UpstreamStrategy;

    #[derive(Clone, Debug, PartialEq)]
    pub struct Route {
        pub id: String,
        pub name: String,
        pub methods: Vec<String>,
        pub paths: Vec<String>,
        pub strategy: UpstreamStrategy,
    }

    impl Route {
        pub fn build(
            id: String,
            name: String,
            methods: Vec<String>,
            paths: Vec<String>,
            strategy: UpstreamStrategy,
        ) -> Self {
            Route {
                id,
                name,
                methods,
                paths,
                strategy,
            }
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
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum UpstreamStrategy {
        AlwaysFirst {
            upstreams: Vec<Upstream>,
        },
        RoundRobin {
            upstreams: Vec<Upstream>,
            next_index: usize,
        },
    }

    impl UpstreamStrategy {
        pub fn next(&mut self) -> Option<&Upstream> {
            match self {
                UpstreamStrategy::AlwaysFirst { upstreams } => {
                    let mut result = None;
                    for upstream in upstreams.iter() {
                        if upstream.enabled {
                            result = Some(upstream);
                            break;
                        }
                    }
                    result
                },
                UpstreamStrategy::RoundRobin { upstreams, next_index } => {
                    let mut result = None;
                    let mut iter_counter = 0;

                    loop {
                        if iter_counter == upstreams.len() {
                            break;
                        }

                        match upstreams.get(*next_index) {
                            Some(ups) => {
                                if ups.enabled {
                                    *next_index = (*next_index + 1) % upstreams.len();
                                    result = Some(ups);
                                    break;
                                }
                            },
                            None => {},
                        }

                        iter_counter = iter_counter + 1;
                    }

                    result
                },
            }
        }

        pub fn get_upstreams(&self) -> Vec<&Upstream> {
            match self {
                UpstreamStrategy::AlwaysFirst { upstreams } => {
                    upstreams.iter().collect()
                },
                UpstreamStrategy::RoundRobin { upstreams, .. } => {
                    upstreams.iter().collect()
                },
            }
        }

        pub fn enable_upstream(&mut self, upstream_address: &UpstreamAddress) {
            match self {
                UpstreamStrategy::AlwaysFirst { upstreams } => {
                    for u in upstreams {
                        if u.address == *upstream_address {
                            u.enabled = true;
                        }
                    }
                },
                UpstreamStrategy::RoundRobin { upstreams, .. } => {
                    for u in upstreams {
                        if u.address == *upstream_address {
                            u.enabled = true;
                        }
                    }
                },
            }
        }

        pub fn disable_upstream(&mut self, upstream_address: &UpstreamAddress) {
            match self {
                UpstreamStrategy::AlwaysFirst { upstreams } => {
                    for u in upstreams {
                        if u.address == *upstream_address {
                            u.enabled = false;
                        }
                    }
                },
                UpstreamStrategy::RoundRobin { upstreams, .. } => {
                    for u in upstreams {
                        if u.address == *upstream_address {
                            u.enabled = false;
                        }
                    }
                },
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::modules::core::upstream::{Upstream, UpstreamStrategy};

        #[test]
        fn should_return_always_first() {
            // given:
            let upstream1 = Upstream::build_from_fqdn("localhost:8080");
            let upstream2 = Upstream::build_from_fqdn("localhost:8081");
            let upstreams = vec![upstream1.clone(), upstream2.clone()];
            let mut strategy = UpstreamStrategy::AlwaysFirst { upstreams };

            // when:
            let first_result = strategy.next().unwrap().clone();
            let second_result = strategy.next().unwrap().clone();

            // then:
            assert_eq!(first_result, upstream1);
            assert_eq!(second_result, upstream1);
        }

        #[test]
        fn should_return_round_robin() {
            // given:
            let upstream1 = Upstream::build_from_fqdn("localhost:8080");
            let upstream2 = Upstream::build_from_fqdn("localhost:8081");
            let upstreams = vec![upstream1.clone(), upstream2.clone()];
            let mut strategy = UpstreamStrategy::RoundRobin {
                upstreams,
                next_index: 0,
            };

            // when:
            let first_result = strategy.next().unwrap().clone();
            let second_result = strategy.next().unwrap().clone();
            let third_result = strategy.next().unwrap().clone();
            let fourth_result = strategy.next().unwrap().clone();

            // then:
            assert_eq!(first_result, upstream1);
            assert_eq!(second_result, upstream2);
            assert_eq!(third_result, upstream1);
            assert_eq!(fourth_result, upstream2);
        }

        #[test]
        fn should_return_second_if_first_disabled_af() {
            // given:
            let mut upstream1 = Upstream::build_from_fqdn("localhost:8080");
            let upstream2 = Upstream::build_from_fqdn("localhost:8081");
            upstream1.enabled = false;
            let upstreams = vec![upstream1.clone(), upstream2.clone()];
            let mut strategy = UpstreamStrategy::AlwaysFirst { upstreams };

            // when:
            let result = strategy.next().unwrap().clone();

            // then:
            assert_eq!(result, upstream2);
        }

        #[test]
        fn should_return_none_if_upstreams_disabled_af() {
            // given:
            let mut upstream1 = Upstream::build_from_fqdn("localhost:8080");
            let mut upstream2 = Upstream::build_from_fqdn("localhost:8081");
            upstream1.enabled = false;
            upstream2.enabled = false;
            let upstreams = vec![upstream1, upstream2];
            let mut strategy = UpstreamStrategy::AlwaysFirst { upstreams };

            // when:
            let result = strategy.next();

            // then:
            assert_eq!(result, None);
        }

        #[test]
        fn should_return_none_if_upstreams_disabled_rr() {
            // given:
            let mut upstream1 = Upstream::build_from_fqdn("localhost:8080");
            let mut upstream2 = Upstream::build_from_fqdn("localhost:8081");
            upstream1.enabled = false;
            upstream2.enabled = false;
            let upstreams = vec![upstream1, upstream2];
            let mut strategy = UpstreamStrategy::RoundRobin {
                upstreams,
                next_index: 0,
            };

            // when:
            let result = strategy.next();

            // then:
            assert_eq!(result, None);
        }
    }
}
