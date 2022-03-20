use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use crate::model::upstream_strategy::UpstreamStrategy;

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub struct Route {
    pub name: String,
    pub methods: Vec<String>,
    pub uris: Vec<String>,
    pub upstreams: Vec<String>,
}

impl Route {
    pub fn build(
        name: &str,
        methods: &[&str],
        uris: &[&str],
        upstreams: &[&str]
    ) -> Self {
        let deduped_methods = deduplicate(methods);
        let deduped_uris = deduplicate(uris);
        let deduped_upstreams = deduplicate(upstreams);

        Route {
            name: String::from(name),
            methods: deduped_methods,
            uris: deduped_uris,
            upstreams: deduped_upstreams,
        }
    }
}

fn deduplicate(list: &[&str]) -> Vec<String> {
    let set: HashSet<String> = list.iter()
        .map(|item| { String::from(*item) })
        .collect();
    set.into_iter().collect()
}

#[derive(Debug)]
pub struct Context<T>
    where T: UpstreamStrategy + Debug + Clone
{
    routes: HashMap<String, Vec<Route>>,
    upstream_strategy: T, // for now we use the same upstream strategy for all routes
}

impl<T> Context<T>
    where T: UpstreamStrategy + Debug + Clone
{
    pub fn build(upstream_strategy: T) -> Self {
        Context {
            routes: HashMap::new(),
            upstream_strategy
        }
    }

    pub fn build_from_routes(routes: &HashSet<Route>, upstream_strategy: T) -> Self {
        let mut routes_map: HashMap<String, Vec<Route>> = HashMap::new();

        for route in routes {
            for uri in &route.uris {
                routes_map
                    .entry(String::from(uri))
                    .or_insert_with(Vec::new)
                    .push((*route).clone())
            }
        }

        Context {
            routes: routes_map,
            upstream_strategy,
        }
    }

    pub fn register_route(&mut self, route: &Route) {
        for uri in &route.uris {
            self.routes.entry(String::from(uri))
                .or_insert_with(Vec::new)
                .push(route.clone())
        }
    }


    pub fn get_upstream_for(&self, method: &str, path: &str) -> Option<String> {
        self.get_best_matching_route(method, path)
            .and_then(|route| {
                self.upstream_strategy.next_for(route)
            })
    }

    fn get_best_matching_route(&self, method: &str, path: &str) -> Option<&Route> {
        // TODO: context policy for determining the best matching route
        let relevant_routes = self.get_relevant_routes(path);

        let mut result = None;
        for route in relevant_routes {
            if route.methods.contains(&method.to_string()) {
                // return first relevant route that contains given method
                result = Some(route);
                break
            }
        }
        result
    }

    fn get_relevant_routes(&self, uri: &str) -> &[Route] {
        match self.routes.get(uri) {
            Some(routes) => {
                routes.as_slice()
            },
            None => &[]
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::iter::FromIterator;
    use crate::{Context, Route};
    use crate::model::upstream_strategy::AlwaysFirstUpstreamStrategy;

    #[test]
    fn should_create_context_from_routes() {
        let routes_vec = vec!(sample_route_1(), sample_route_2());
        let routes = HashSet::from_iter(routes_vec);

        let upstream_strategy = AlwaysFirstUpstreamStrategy::build();
        let context = Context::build_from_routes(&routes, upstream_strategy);
        assert_eq!(context.routes.len(), 3);
    }

    #[test]
    fn should_register_route() {
        let upstream_strategy = AlwaysFirstUpstreamStrategy::build();
        let mut context = Context::build(upstream_strategy);
        let route = sample_route_1();

        context.register_route(&route);

        for uri in &route.uris {
            assert_eq!(context.routes.contains_key(uri), true)
        }
    }

    #[test]
    fn should_find_upstream_for_get() {
        let routes_vec = vec!(sample_route_1(), sample_route_2());
        let routes = HashSet::from_iter(routes_vec);
        let upstream_strategy = AlwaysFirstUpstreamStrategy::build();
        let context = Context::build_from_routes(&routes, upstream_strategy);

        let upstream = context.get_upstream_for("GET", "uri1");

        assert_eq!(Some(String::from("upstream1")), upstream)
    }

    #[test]
    fn should_not_find_upstream_for_post() {
        let routes_vec = vec!(sample_route_1(), sample_route_2());
        let routes = HashSet::from_iter(routes_vec);
        let upstream_strategy = AlwaysFirstUpstreamStrategy::build();
        let context = Context::build_from_routes(&routes, upstream_strategy);

        let upstream = context.get_upstream_for("POST", "uri1");

        assert_eq!(None, upstream)
    }

    #[test]
    fn should_not_find_upstream_for_unknown_uri() {
        let routes_vec = vec!(sample_route_1(), sample_route_2());
        let routes = HashSet::from_iter(routes_vec);
        let upstream_strategy = AlwaysFirstUpstreamStrategy::build();
        let context = Context::build_from_routes(&routes, upstream_strategy);

        let upstream = context.get_upstream_for("GET", "uri4");

        assert_eq!(None, upstream)
    }

    fn sample_route_1() -> Route {
        Route::build(
            "route1",
            &["GET"],
            &["uri1", "uri2"],
            &["upstream1"],
        )
    }

    fn sample_route_2() -> Route {
        Route::build(
            "route2",
            &["GET"],
            &["uri2", "uri3"],
            &["upstream2"],
        )
    }

    fn sample_context_with_routes() -> Context<AlwaysFirstUpstreamStrategy> {
        let routes_vec = vec!(sample_route_1(), sample_route_2());
        let routes = HashSet::from_iter(routes_vec);
        let upstream_strategy = AlwaysFirstUpstreamStrategy::build();
        Context::build_from_routes(&routes, upstream_strategy)
    }
}