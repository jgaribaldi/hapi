extern crate core;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Server};

use crate::errors::HapiError;
use crate::infrastructure::processor;
use crate::infrastructure::stats::Stats;
use crate::infrastructure::upstream_probe::{probe_upstream, UpstreamProbeConfiguration};
use crate::model::context::Context;
use crate::model::route::Route;
use crate::model::upstream::{AlwaysFirstUpstreamStrategy, RoundRobinUpstreamStrategy, Upstream};

mod errors;
mod infrastructure;
mod model;

#[tokio::main]
async fn main() -> Result<(), HapiError> {
    simple_logger::init_with_env()?;

    log::info!("This is Hapi, the Happy API");
    let context = Arc::new(Mutex::new(initialize_context()));
    let stats = Arc::new(Mutex::new(Stats::build()));
    let upstream_probe_config = create_upstream_probe_configuration();

    for upc in upstream_probe_config {
        let ctx = context.clone();
        tokio::spawn(async move {
            probe_upstream(upc, ctx).await;
        });
    }

    let make_service = make_service_fn(move |conn: &AddrStream| {
        let context = context.clone();
        let stats = stats.clone();
        let remote_addr = conn.remote_addr();

        let service = service_fn(move |request| {
            let client = identify_client(&remote_addr, &request);
            processor::process_request(context.clone(), request, stats.clone(), client)
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
        vec![String::from("GET")],
        vec![String::from("/test")],
        vec![
            Upstream::build("localhost:8001"),
            Upstream::build("localhost:8002"),
        ],
        Box::new(RoundRobinUpstreamStrategy::build(0)),
    );
    let route2 = Route::build(
        String::from("Test 2"),
        vec![String::from("GET")],
        vec![String::from("/test2")],
        vec![
            Upstream::build("localhost:8001"),
            Upstream::build("localhost:8002"),
        ],
        Box::new(AlwaysFirstUpstreamStrategy::build()),
    );
    let context = Context::build_from_routes(vec![route1, route2]);
    log::info!("{:?}", context);
    context
}

fn create_upstream_probe_configuration() -> Vec<UpstreamProbeConfiguration> {
    vec![
        UpstreamProbeConfiguration::build("localhost:8001", 2000, 5, 5),
        UpstreamProbeConfiguration::build("localhost:8002", 4000, 2, 2),
    ]
}

async fn graceful_quit() {
    tokio::signal::ctrl_c()
        .await
        .expect("Could not install graceful quit signal handler");
    log::info!("Shutting down Hapi. Bye :-)")
}

fn identify_client(remote_addr: &SocketAddr, _request: &Request<Body>) -> String {
    remote_addr.ip().to_string()
}