use std::str::FromStr;

use hyper::header::HOST;
use hyper::{Body, Client, HeaderMap, Request, Response, Uri};
use tokio::sync::broadcast::{Receiver, Sender};

use crate::HapiError;
use crate::events::commands::Command;
use crate::events::events::Event;
use crate::infrastructure::core_handler::CoreClient;
use crate::modules::core::upstream::UpstreamAddress;

pub(crate) async fn process_request(
    request: Request<Body>,
    client: String,
    send_cmd: Sender<Command>,
    recv_evt: Receiver<Event>,
) -> Result<Response<Body>, HapiError> {
    let method = request.method();
    let path = request.uri().path();

    let mut core_client = CoreClient::build(send_cmd, recv_evt);
    // TODO: remove the following unwrap
    let maybe_upstream = core_client.search_upstream(client.as_str(), path, method.as_str()).await.unwrap();
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
