use std::str::FromStr;

use hyper::header::HOST;
use hyper::{Body, Client, HeaderMap, Request, Response, Uri};
use tokio::sync::broadcast::{Receiver, Sender};

use crate::HapiError;
use crate::events::commands::Command;
use crate::events::events::Event;
use crate::modules::core::upstream::UpstreamAddress;

pub(crate) async fn process_request(
    request: Request<Body>,
    client: String,
    send_cmd: Sender<Command>,
    recv_evt: Receiver<Event>,
) -> Result<Response<Body>, HapiError> {
    let method = request.method();
    let path = request.uri().path();

    let maybe_upstream = search_upstream(path, method.as_str(), send_cmd, recv_evt).await;
    match maybe_upstream {
        Some(upstream_address) => {
            let upstream_uri = Uri::from_str(absolute_url_for(&upstream_address, path).as_str())?;
            let headers = headers_for(&request, &upstream_address);

            let mut upstream_request = Request::from(request);
            *upstream_request.uri_mut() = upstream_uri;
            *upstream_request.headers_mut() = headers;
            log::debug!("Generated: {:?}", &upstream_request);

            let client = Client::new();
            let response = client.request(upstream_request).await?;

            log::debug!("Response: {:?}", &response);
            Ok(response)
        }
        None => {
            log::debug!("No routes found for {:?}", request);
            let response = Response::builder().status(404).body(Body::empty()).unwrap();
            Ok(response)
        }
    }
}

async fn search_upstream(
    path: &str,
    method: &str,
    send_cmd: Sender<Command>,
    mut recv_evt: Receiver<Event>,
) -> Option<UpstreamAddress> {
    // TODO: fix
    let command = Command::LookupUpstream { id: String::from("1234") };
    match send_cmd.send(command) {
        Ok(_) => log::debug!("Command sent"),
        Err(e) => log::error!("Error sending command {}", e),
    };

    let mut result = None;
    while let Ok(event) = recv_evt.recv().await {
        match event {
            // TODO: fix
            Event::UpstreamWasFound { cmd_id } => {
                if cmd_id == String::from("1234") {
                    result = Some(UpstreamAddress::FQDN("127.0.0.1".to_string()));
                    break
                }
            },
            Event::UpstreamWasNotFound { cmd_id } => {
                // TODO: fix
                if cmd_id == String::from("1234") {
                    result = None
                }
            },
            _ => {},
        }
    };
    result
}

fn absolute_url_for(upstream: &UpstreamAddress, original_path: &str) -> String {
    let mut absolute_url = String::from("http://");
    absolute_url.push_str(upstream.to_string().as_str());
    absolute_url.push_str(original_path);
    absolute_url
}

fn headers_for(request: &Request<Body>, upstream: &UpstreamAddress) -> HeaderMap {
    let original_headers = request.headers();
    let mut headers = original_headers.clone();
    headers.insert(HOST, upstream.to_string().parse().unwrap());
    headers
}
