extern crate core;

use std::net::SocketAddr;
use std::sync::{Arc};
use hyper::Server;
use hyper::service::{make_service_fn, service_fn};
use tokio::sync::Mutex;
use crate::errors::HapiError;
use crate::model::context::{Context, Route};
use crate::model::upstream::{AlwaysFirstUpstreamStrategy, RoundRobinUpstreamStrategy, Upstream};

mod infrastructure;
mod errors;
mod model;

// TODO: this is awful, remove it
unsafe impl Send for Context {
}

#[tokio::main]
async fn main() -> Result<(), HapiError> {
    simple_logger::init_with_env()?;

    log::info!("This is Hapi, the Happy API");
    let context = Arc::new(Mutex::new(initialize_context()));

    let make_service = make_service_fn(move |_conn| {
        let context = context.clone();
        let service = service_fn(move |request| {
            infrastructure::process_request(context.clone(), request)
        });
        async move { Ok::<_, HapiError>(service) }
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let server = Server::bind(&addr)
        .serve(make_service)
        .with_graceful_shutdown(graceful_quit());

    if let Err(e) = server.await {
        log::error!("server error: {}", e);
    }
    Ok(())
}

fn initialize_context() -> Context {
    let route1 = Route::build(
        String::from("Test 1"),
        vec!(String::from("GET")),
        vec!(String::from("/test")),
        vec!(
            Upstream::build("localhost:8001"),
            Upstream::build("localhost:8002"),
        ),
        Box::new(RoundRobinUpstreamStrategy::build(0, 2)),
    );
    let route2 = Route::build(
        String::from("Test 2"),
        vec!(String::from("GET")),
        vec!(String::from("/test2")),
        vec!(
            Upstream::build("localhost:8001"),
            Upstream::build("localhost:8002"),
        ),
        Box::new(AlwaysFirstUpstreamStrategy::build()),
    );
    let context = Context::build_from_routes(vec!(route1, route2));
    log::info!("{:?}", context);
    context
}

async fn graceful_quit() {
    tokio::signal::ctrl_c()
        .await
        .expect("Could not install graceful quit signal handler");
    log::info!("Shutting down Hapi. Bye :-)")
}