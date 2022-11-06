use std::str::FromStr;
use std::sync::{Arc, Mutex};

use hyper::header::HOST;
use hyper::{Body, Client, HeaderMap, Request, Response, Uri};

use crate::model::upstream::UpstreamAddress;
use crate::{Context, HapiError};

pub async fn process_request(
    context: Arc<Mutex<Context>>,
    request: Request<Body>,
) -> Result<(Response<Body>, Option<UpstreamAddress>), HapiError> {
    let method = request.method();
    let path = request.uri().path();

    let maybe_upstream = search_upstream(context, path, method.as_str());
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
            Ok((response, Some(upstream_address)))
        }
        None => {
            log::debug!("No routes found for {:?}", request);
            let response = Response::builder().status(404).body(Body::empty()).unwrap();
            Ok((response, None))
        }
    }
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
