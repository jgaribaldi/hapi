use std::str::FromStr;
use futures_util::TryFutureExt;
use hyper::{Body, header, Method, Request, Response};
use tokio::sync::broadcast::error::SendError;
use tokio::sync::broadcast::{Receiver, Sender};
use uuid::Uuid;
use crate::errors::HapiError;
use crate::events::commands::Command;
use crate::events::events::Event;
use crate::infrastructure::core_handler::CoreClient;
use crate::modules::core::route::Route;

pub(crate) async fn handle_api(
    request: Request<Body>,
    send_cmd: Sender<Command>,
    mut recv_evt: Receiver<Event>,
) -> Result<Response<Body>, HapiError> {
    log::debug!("Received: {:?}", &request);

    let path = request.uri().path().to_owned();
    let path_parts: Vec<&str> = path.split("/").collect();

    let resource = ApiResource::from_str(path_parts[1]).unwrap();
    let resource_id = path_parts.get(2);

    let response = match (resource, request.method(), resource_id) {
        (ApiResource::Route, &Method::GET, None) => {
            let routes = get_routes(send_cmd, recv_evt).await;
            let content = serde_json::to_string(&routes).unwrap();
            json(content)
        },
        (ApiResource::Route, &Method::GET, Some(r_id)) => {
            if let Some(r) = get_route(*r_id, send_cmd, recv_evt).await {
                let content = serde_json::to_string(&r).unwrap(); // TODO: remove unwrap
                json(content)
            } else {
                not_found()
            }
        },
        (ApiResource::Route, &Method::POST, None) => {
            not_found() // TODO: remove
        },
        (ApiResource::Route, &Method::DELETE, Some(r_id)) => {
            not_found() // TODO: remove
        },
        _ => {
            not_found() // TODO: remove
        }
    };

    log::debug!("Response: {:?}", &response);
    Ok(response)
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

async fn get_routes(
    send_cmd: Sender<Command>,
    mut recv_evt: Receiver<Event>,
) -> Vec<crate::infrastructure::serializable_model::Route> {
    let mut core_client = CoreClient::build(send_cmd, recv_evt);
    let found_routes = core_client.get_routes().await.unwrap(); // TODO: remove unwrap

    let mut result = Vec::new();
    for r in found_routes {
        result.push(crate::infrastructure::serializable_model::Route::from(r))
    }
    result
}

async fn get_route(
    route_id: &str,
    send_cmd: Sender<Command>,
    mut recv_evt: Receiver<Event>,
) -> Option<crate::infrastructure::serializable_model::Route> {
    let mut core_client = CoreClient::build(send_cmd, recv_evt);
    core_client.get_route_by_id(route_id).await.unwrap() // TODO: remove unwrap
        .map(|r| crate::infrastructure::serializable_model::Route::from(r))
}

fn ok() -> Response<Body> {
    Response::builder().status(200).body(Body::empty()).unwrap()
}

fn created() -> Response<Body> {
    Response::builder().status(201).body(Body::empty()).unwrap()
}

fn not_found() -> Response<Body> {
    Response::builder().status(404).body(Body::empty()).unwrap()
}

fn bad_request(e: HapiError) -> Response<Body> {
    let body = Body::from(e.to_string());
    Response::builder().status(400).body(body).unwrap()
}

fn json(json: String) -> Response<Body> {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .status(200)
        .body(Body::from(json))
        .unwrap()
}