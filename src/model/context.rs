use std::collections::HashMap;
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
        name: String,
        methods: Vec<String>,
        uris: Vec<String>,
        upstreams: Vec<String>,
    ) -> Self {
        Route { name, methods, uris, upstreams }
    }
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

    pub fn build_from_routes(routes: Vec<Route>, upstream_strategy: T) -> Self {
        let mut routes_map: HashMap<String, Vec<Route>> = HashMap::new();

        for route in routes {
            for uri in &route.uris {
                routes_map
                    .entry(String::from(uri))
                    .or_insert_with(Vec::new)
                    .push(route.clone())
            }
        }

        Context {
            routes: routes_map,
            upstream_strategy,
        }
    }

    pub fn register_route(&mut self, route: Route) {
        for uri in &route.uris {
            self.routes.entry(String::from(uri))
                .or_insert_with(Vec::new)
                .push(route.clone())
        }
    }


    pub fn get_upstream_for(&mut self, method: &str, path: &str) -> Option<String> {
        let best_matching_route = self.get_best_matching_route(method, path);
        if let Some(route) = best_matching_route {
            let r = route.clone();
            self.upstream_strategy.next_for(&r)
        } else {
            None
        }
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
    use crate::{Context, Route};
    use crate::model::upstream_strategy::AlwaysFirstUpstreamStrategy;

    #[test]
    fn should_create_context_from_routes() {
        let routes = vec!(sample_route_1(), sample_route_2());

        let upstream_strategy = AlwaysFirstUpstreamStrategy::build();
        let context = Context::build_from_routes(routes, upstream_strategy);
        assert_eq!(context.routes.len(), 3);
    }

    #[test]
    fn should_register_route() {
        let upstream_strategy = AlwaysFirstUpstreamStrategy::build();
        let mut context = Context::build(upstream_strategy);
        let route = sample_route_1();
        let uris = route.uris.clone();

        context.register_route(route);

        for uri in uris {
            assert_eq!(context.routes.contains_key(&*uri), true)
        }
    }

    #[test]
    fn should_find_upstream_for_get() {
        let routes = vec!(sample_route_1(), sample_route_2());
        let upstream_strategy = AlwaysFirstUpstreamStrategy::build();
        let mut context = Context::build_from_routes(routes, upstream_strategy);

        let upstream = context.get_upstream_for("GET", "uri1");

        assert_eq!(Some(String::from("upstream1")), upstream)
    }

    #[test]
    fn should_not_find_upstream_for_post() {
        let routes = vec!(sample_route_1(), sample_route_2());
        let upstream_strategy = AlwaysFirstUpstreamStrategy::build();
        let mut context = Context::build_from_routes(routes, upstream_strategy);

        let upstream = context.get_upstream_for("POST", "uri1");

        assert_eq!(None, upstream)
    }

    #[test]
    fn should_not_find_upstream_for_unknown_uri() {
        let routes = vec!(sample_route_1(), sample_route_2());
        let upstream_strategy = AlwaysFirstUpstreamStrategy::build();
        let mut context = Context::build_from_routes(routes, upstream_strategy);

        let upstream = context.get_upstream_for("GET", "uri4");

        assert_eq!(None, upstream)
    }

    fn sample_route_1() -> Route {
        Route::build(
            String::from("route1"),
            vec!(String::from("GET")),
            vec!(String::from("uri1"), String::from("uri2")),
            vec!(String::from("upstream1")),
        )
    }

    fn sample_route_2() -> Route {
        Route::build(
            String::from("route2"),
            vec!(String::from("GET")),
            vec!(String::from("uri2"), String::from("uri3")),
            vec!(String::from("upstream2")),
        )
    }
}