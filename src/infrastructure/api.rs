use std::str::FromStr;
use std::sync::{Arc, Mutex};

use hyper::{header, Body, Method, Request, Response};
use serde::Deserialize;
use serde::Serialize;

use crate::model::upstream::UpstreamAddress;
use crate::{Context, HapiError, Stats};
use crate::infrastructure::serializable_model::Route;

pub async fn process_request(
    context: Arc<Mutex<Context>>,
    stats: Arc<Mutex<Stats>>,
    request: Request<Body>,
) -> Result<Response<Body>, HapiError> {
    log::debug!("Received: {:?}", &request);

    let path = request.uri().path().to_owned();
    let path_parts: Vec<&str> = path.split("/").collect();

    let resource = ApiResource::from_str(path_parts[1]).unwrap();

    let response = match (resource, request.method()) {
        (ApiResource::Route, &Method::GET) => {
            if path_parts.len() > 2 {
                // a route ID was given
                match get_route_by_id_json(context, path_parts[2]) {
                    Some(json_route) => json_response(json_route),
                    None => not_found_response()
                }
            } else {
                let json = get_all_routes_json(context);
                json_response(json)
            }
        }
        (ApiResource::Route, &Method::DELETE) => {
            match delete_route(context, path_parts[2]) {
                Ok(_) => ok_response(),
                Err(_) => not_found_response()
            }
        }
        (ApiResource::Upstream, &Method::GET) => {
            let json = get_all_upstreams_json(context);
            json_response(json)
        }
        (ApiResource::Stats, &Method::GET) => {
            let json = get_all_stats_json(stats);
            json_response(json)
        }
        _ => not_found_response(),
    };

    log::debug!("Response: {:?}", &response);
    Ok(response)
}

fn get_all_upstreams_json(context: Arc<Mutex<Context>>) -> String {
    let upstreams = get_upstreams(context);
    let serializable_addresses: Vec<String> = upstreams.iter().map(|u| u.to_string()).collect();
    serde_json::to_string(&serializable_addresses).unwrap()
}

fn get_upstreams(context: Arc<Mutex<Context>>) -> Vec<UpstreamAddress> {
    let ctx = context.lock().unwrap();
    ctx.get_all_upstreams()
}

fn get_all_routes_json(context: Arc<Mutex<Context>>) -> String {
    let ctx = context.lock().unwrap();
    let routes = ctx.get_all_routes();

    let mut serializable_routes = Vec::new();
    for r in routes {
        serializable_routes.push(crate::infrastructure::serializable_model::Route::from(
            r.clone(),
        ));
    }

    serde_json::to_string(&serializable_routes).unwrap()
}

fn get_all_stats_json(stats: Arc<Mutex<Stats>>) -> String {
    let sts = get_all_stats(stats);

    let mut serializable_stats = Vec::new();
    for s in sts {
        serializable_stats.push(SerializableStats::from(s));
    }

    serde_json::to_string(&serializable_stats).unwrap()
}

fn get_all_stats(stats: Arc<Mutex<Stats>>) -> Vec<(String, String, String, String, u64)> {
    let sts = stats.lock().unwrap();
    sts.get_all()
}

fn get_route_by_id_json(context: Arc<Mutex<Context>>, route_id: &str) -> Option<String> {
    get_route_by_id(context, route_id)
        .map(|serializable_route| serde_json::to_string(&serializable_route).unwrap())
}

fn get_route_by_id(context: Arc<Mutex<Context>>, route_id: &str) -> Option<Route> {
    let ctx = context.lock().unwrap();
    ctx.get_route_by_id(route_id)
        .map(|route| Route::from(route.clone()))
}

fn delete_route(context: Arc<Mutex<Context>>, route_id: &str) -> Result<(), HapiError> {
    let mut ctx = context.lock().unwrap();
    ctx.remove_route(route_id)
}

fn json_response(json: String) -> Response<Body> {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .status(200)
        .body(Body::from(json))
        .unwrap()
}

fn not_found_response() -> Response<Body> {
    Response::builder().status(404).body(Body::empty()).unwrap()
}

fn ok_response() -> Response<Body> {
    Response::builder().status(201).body(Body::empty()).unwrap()
}

enum ApiResource {
    Route,
    Upstream,
    Stats,
    Unknown,
}

impl FromStr for ApiResource {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "routes" => Ok(ApiResource::Route),
            "upstreams" => Ok(ApiResource::Upstream),
            "stats" => Ok(ApiResource::Stats),
            _ => Ok(ApiResource::Unknown),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SerializableStats {
    client: String,
    method: String,
    path: String,
    upstream: String,
    count: u64,
}

impl From<(String, String, String, String, u64)> for SerializableStats {
    fn from(stat: (String, String, String, String, u64)) -> Self {
        SerializableStats {
            client: stat.0,
            method: stat.1,
            path: stat.2,
            upstream: stat.3,
            count: stat.4,
        }
    }
}
