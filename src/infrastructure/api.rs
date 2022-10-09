use std::sync::{Arc, Mutex};

use hyper::{Body, header, Method, Request, Response};
use regex::Regex;

use crate::{Context, HapiError};
use crate::model::upstream::UpstreamAddress;

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
        }
        (path, &Method::GET) if upstream_regex.is_match(path) => {
            let json = get_all_upstreams_json(context);
            json_response(json)
        }
        _ => Response::builder().status(404).body(Body::empty()).unwrap(),
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

fn json_response(json: String) -> Response<Body> {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .status(200)
        .body(Body::from(json))
        .unwrap()
}
