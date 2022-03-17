use std::str::FromStr;
use std::sync::Arc;
use hyper::{Body, Client, HeaderMap, Request, Response, Uri};
use hyper::header::HOST;
use crate::{Context, HapiError};

type Model = Arc<Context>;

#[derive(Clone)]
pub struct Infrastructure {
    model: Model
}

impl Infrastructure {
    pub fn build(model: Model) -> Self {

        Infrastructure {
            model,
        }
    }

    pub async fn process_request(self, request: Request<Body>) -> Result<Response<Body>, HapiError> {
        log::debug!("Received: {:?}", &request);
        let method = request.method().as_str();
        let path = request.uri().path();

        let response = match self.model.get_upstream_for(method, path) {
            Some(upstream) => {
                let upstream_uri = Uri::from_str(absolute_url_for(upstream, path).as_str())?;
                let headers = headers_for(&request, upstream);

                let mut upstream_request = Request::from(request);
                *upstream_request.uri_mut() = upstream_uri;
                *upstream_request.headers_mut() = headers;
                log::debug!("Generated: {:?}", &upstream_request);

                let client = Client::new();
                client.request(upstream_request).await?
            }
            None => {
                log::debug!("No routes found for {:?}", request);
                Response::builder()
                    .status(404)
                    .body(Body::empty())
                    .unwrap()
            }
        };

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