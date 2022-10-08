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
            // unimplemented
            Response::builder().status(418).body(Body::empty()).unwrap()
        },
        (path, &Method::GET) if upstream_regex.is_match(path) => {
            let json = get_all_upstreams_json(context);
            Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .status(200)
                .body(Body::from(json))
                .unwrap()
        },
        _ => Response::builder().status(404).body(Body::empty()).unwrap()
    };

    log::debug!("Response: {:?}", &response);
    Ok(response)
}

fn get_all_upstreams_json(context: Arc<Mutex<Context>>) -> String {
    let upstreams= get_upstreams(context);
    let upstream_addresses: Vec<String> = upstreams.iter()
        .map(|u| u.to_string())
        .collect();
    serde_json::to_string(&upstream_addresses).unwrap()
}

fn get_upstreams(context: Arc<Mutex<Context>>) -> Vec<UpstreamAddress> {
    let ctx = context.lock().unwrap();
    ctx.get_all_upstreams()
}