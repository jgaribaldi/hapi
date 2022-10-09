use std::str::FromStr;
use std::sync::{Arc, Mutex};

use hyper::{Body, header, Method, Request, Response};

use crate::{Context, HapiError};
use crate::model::upstream::UpstreamAddress;

pub async fn process_request(
    context: Arc<Mutex<Context>>,
    request: Request<Body>,
) -> Result<Response<Body>, HapiError> {
    log::debug!("Received: {:?}", &request);

    let path = request.uri().path().to_owned();
    let path_parts: Vec<&str> = path.split("/").collect();

    let resource = ApiResource::from_str(path_parts[1]).unwrap();

    let response = match (resource, request.method()) {
        (ApiResource::Route, &Method::GET) => {
            let json = get_all_routes_json(context);
            json_response(json)
        }
        (ApiResource::Upstream, &Method::GET) => {
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
