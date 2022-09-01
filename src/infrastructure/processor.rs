use std::str::FromStr;
use std::sync::{Arc, Mutex};

use hyper::header::HOST;
use hyper::{Body, Client, HeaderMap, Request, Response, Uri};

use crate::infrastructure::stats;
use crate::infrastructure::stats::Stats;
use crate::model::upstream::UpstreamAddress;
use crate::{Context, HapiError};

pub async fn process_request(
    context: Arc<Mutex<Context>>,
    request: Request<Body>,
    stats: Arc<Mutex<Stats>>,
    client: String,
) -> Result<Response<Body>, HapiError> {
    log::debug!("Received: {:?}", &request);
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let upstream;
    {
        let mut ctx = context.lock().unwrap();
        upstream = ctx.upstream_lookup(path.as_str(), method.as_str());
    }

    let response = if let Some(ups) = upstream {
        let upstream_uri = Uri::from_str(absolute_url_for(&ups, path.as_str()).as_str())?;
        let headers = headers_for(&request, &ups);

        let mut upstream_request = Request::from(request);
        *upstream_request.uri_mut() = upstream_uri;
        *upstream_request.headers_mut() = headers;
        log::debug!("Generated: {:?}", &upstream_request);

        stats::count_request(
            stats,
            client.as_str(),
            method.as_str(),
            path.as_str(),
            ups.to_string().as_str(),
        )
        .await;

        let client = Client::new();
        client.request(upstream_request).await?
    } else {
        log::debug!("No routes found for {:?}", request);
        Response::builder().status(404).body(Body::empty()).unwrap()
    };

    log::debug!("Response: {:?}", &response);
    Ok(response)
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
