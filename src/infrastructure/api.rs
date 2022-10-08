use std::sync::{Arc, Mutex};
use hyper::{Body, header, Method, Request, Response};
use regex::Regex;
use crate::{Context, HapiError};
use crate::model::route::Route;
use crate::model::upstream::UpstreamAddress;
use serde::{Serialize, Deserialize};

pub async fn process_request(
    context: Arc<Mutex<Context>>,
    request: Request<Body>,
) -> Result<Response<Body>, HapiError> {
    log::debug!("Received: {:?}", &request);

    let route_regex = Regex::new("^/routes$|^/routes/$|^/routes/(.*)$").unwrap();
    let upstream_regex = Regex::new("^/upstreams$|^/upstreams/$|^/upstreams/(.*)$").unwrap();

    let response = match (request.uri().path(), request.method()) {
        (path, &Method::GET) if route_regex.is_match(path) => {
            let json = get_all_routes_json(context);
            json_response(json)
        },
        (path, &Method::GET) if upstream_regex.is_match(path) => {
            let json = get_all_upstreams_json(context);
            json_response(json)
        },
        _ => Response::builder().status(404).body(Body::empty()).unwrap()
    };

    log::debug!("Response: {:?}", &response);
    Ok(response)
}

fn get_all_upstreams_json(context: Arc<Mutex<Context>>) -> String {
    let upstreams= get_upstreams(context);
    let serializable_addresses: Vec<String> = upstreams.iter()
        .map(|u| u.to_string())
        .collect();
    serde_json::to_string(&serializable_addresses).unwrap()
}

fn get_upstreams(context: Arc<Mutex<Context>>) -> Vec<UpstreamAddress> {
    let ctx = context.lock().unwrap();
    ctx.get_all_upstreams()
}

fn get_all_routes_json(context: Arc<Mutex<Context>>) -> String {
    let ctx = context.lock().unwrap();
    let routes = ctx.get_all_routes();

    let serializable_routes: Vec<SerializableRoute> = routes.iter()
        .map(|r| SerializableRoute::from(*r))
        .collect();
    serde_json::to_string(&serializable_routes).unwrap()
}

fn json_response(json: String) -> Response<Body> {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .status(200)
        .body(Body::from(json))
        .unwrap()
}

#[derive(Serialize, Deserialize)]
struct SerializableRoute {
    name: String,
    methods: Vec<String>,
    paths: Vec<String>,
    upstreams: Vec<String>,
    strategy: String,
}

impl SerializableRoute {
    fn from(route: &Route) -> Self {
        let upstreams: Vec<String> = route.upstreams.iter()
            .map(|u| u.address.to_string())
            .collect();

        SerializableRoute {
            name: route.name.clone(),
            methods: route.methods.clone(),
            paths: route.paths.clone(),
            upstreams,
            strategy: route.strategy.get_type_name(),
        }
    }
}