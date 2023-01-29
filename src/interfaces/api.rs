use std::str::FromStr;
use hyper::{Body, header, Method, Request, Response};
use tokio::sync::broadcast::error::SendError;
use tokio::sync::broadcast::{Receiver, Sender};
use uuid::Uuid;
use crate::errors::HapiError;
use crate::events::commands::Command;
use crate::events::events::Event;
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

    let response = match (resource, request.method()) {
        (ApiResource::Route, &Method::GET) => {
            match resource_id {
                None => {
                    let routes = get_routes(send_cmd, recv_evt).await;
                    let content = serde_json::to_string(&routes).unwrap();
                    json(content)
                },
                Some(r_id) => not_found(), // TODO: remove
            }
        },
        (ApiResource::Route, &Method::POST) => {
            not_found() // TODO: remove
        },
        (ApiResource::Route, &Method::DELETE) => {
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
    let found_routes = get_routes_from_core(send_cmd, recv_evt).await;
    let mut result = Vec::new();
    for r in found_routes {
        result.push(crate::infrastructure::serializable_model::Route::from(r))
    }
    result
}

async fn get_routes_from_core(
    send_cmd: Sender<Command>,
    mut recv_evt: Receiver<Event>,
) -> Vec<Route> {
    let cmd_uuid = Uuid::new_v4();
    let command = Command::LookupAllRoutes { id: cmd_uuid.to_string() };
    match send_cmd.send(command) {
        Ok(_) => log::debug!("Command sent"),
        Err(e) => log::error!("Error sending command {}", e),
    }

    let mut result = Vec::new();
    while let Ok(event) = recv_evt.recv().await {
        log::debug!("Received event {:?}", event);
        match event {
            Event::RoutesWereFound { cmd_id, routes } => {
                if cmd_id == cmd_uuid.to_string() {
                    result = routes;
                    break
                }
            },
            _ => {},
        }
    }
    result
}

fn get_route() {

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