use std::str::FromStr;
use std::sync::{Arc, Mutex};

use hyper::{Body, Client, HeaderMap, Request, Response, Uri};
use hyper::header::HOST;

use crate::{Context, HapiError};
use crate::infrastructure::stats;
use crate::infrastructure::stats::Stats;
use crate::model::upstream::UpstreamAddress;

pub async fn process_request(
    context: Arc<Mutex<Context>>,
    request: Request<Body>,
    stats: Arc<Mutex<Stats>>,
    client: String,
) -> Result<Response<Body>, HapiError> {
    log::debug!("Received: {:?}", &request);
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let maybe_upstream = search_upstream(context, path.as_str(), method.as_str());
    let response = match maybe_upstream {
        Some(upstream_address) => {
            let upstream_uri =
                Uri::from_str(absolute_url_for(&upstream_address, path.as_str()).as_str())?;
            let headers = headers_for(&request, &upstream_address);

            let mut upstream_request = Request::from(request);
            *upstream_request.uri_mut() = upstream_uri;
            *upstream_request.headers_mut() = headers;
            log::debug!("Generated: {:?}", &upstream_request);

            stats::count_request(
                stats,
                client.as_str(),
                method.as_str(),
                path.as_str(),
                upstream_address.to_string().as_str(),
            )
            .await;

            let client = Client::new();
            client.request(upstream_request).await?
        }
        None => {
            log::debug!("No routes found for {:?}", request);
            Response::builder().status(404).body(Body::empty()).unwrap()
        }
    };

    log::debug!("Response: {:?}", &response);
    Ok(response)
}

fn search_upstream(
    context: Arc<Mutex<Context>>,
    path: &str,
    method: &str,
) -> Option<UpstreamAddress> {
    let mut ctx = context.lock().unwrap();
    ctx.upstream_lookup(path, method)
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
