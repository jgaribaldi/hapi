use hyper::{Body, Request, Response};
use crate::HapiError;

pub async fn process_request(request: Request<Body>) -> Result<Response<Body>, HapiError> {
    // API not implemented yet
    Ok(Response::builder().status(418).body(Body::empty()).unwrap())
}