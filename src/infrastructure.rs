use std::fmt::Debug;
use std::str::FromStr;
use std::sync::{Arc};
use hyper::{Body, Client, HeaderMap, Request, Response, Uri};
use hyper::header::HOST;
use tokio::sync::Mutex;
use crate::{Context, HapiError};
use crate::model::upstream::UpstreamStrategy;

// type Model<T> = Arc<Mutex<Context<T>>>;

#[derive(Clone)]
pub struct Infrastructure {
    // model: Model<T>
}

impl Infrastructure {
    pub fn build() -> Self {
        Infrastructure {
            // model,
        }
    }

    pub async fn process_request(self, request: Request<Body>) -> Result<Response<Body>, HapiError> {
        log::debug!("Received: {:?}", &request);
        // let method = request.method().as_str();
        // let path = request.uri().path();
        //
        // let upstream;
        // {
        //     let mut model = self.model.lock().await;
        //     upstream = model.get_upstream_for(method, path);
        // }
        //
        // let response = if let Some(upstream) = upstream {
        //     let upstream_uri = Uri::from_str(absolute_url_for(upstream.as_str(), path).as_str())?;
        //     let headers = headers_for(&request, upstream.as_str());
        //
        //     let mut upstream_request = Request::from(request);
        //     *upstream_request.uri_mut() = upstream_uri;
        //     *upstream_request.headers_mut() = headers;
        //     log::debug!("Generated: {:?}", &upstream_request);
        //
        //     let client = Client::new();
        //     client.request(upstream_request).await?
        // } else {
        //     log::debug!("No routes found for {:?}", request);
        //     Response::builder()
        //         .status(404)
        //         .body(Body::empty())
        //         .unwrap()
        // };

        let response = Response::builder()
                .status(404)
                .body(Body::empty())
                .unwrap();

        Ok(response)
    }
}

fn absolute_url_for(upstream: &str, original_path: &str) -> String {
    let mut absolute_url = String::from("http://");
    absolute_url.push_str(upstream);
    absolute_url.push_str(original_path);
    absolute_url
}

fn headers_for(request: &Request<Body>, upstream: &str) -> HeaderMap {
    let original_headers = request.headers();
    let mut headers = original_headers.clone();
    headers.insert(HOST, upstream.parse().unwrap());
    headers
}