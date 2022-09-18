use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Server};
use tokio::sync::mpsc;

use crate::errors::HapiError;
use crate::infrastructure::processor;
use crate::infrastructure::stats::Stats;
use crate::infrastructure::upstream_probe::{
    upstream_probe_handler, Command, UpstreamProbeConfiguration,
};
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

    // build an empty context and add some routes so we can get the upstream addresses for the probe
    // configurations
    let mut upstreams = Vec::new();
    let mut ctx = Context::build_empty();

    if let Some(added_routes) = ctx.add_route(test_route_1()) {
        upstreams.extend_from_slice(&added_routes.as_slice());
    }

    if let Some(added_routes) = ctx.add_route(test_route_2()) {
        upstreams.extend_from_slice(&added_routes.as_slice());
    }
    log::info!("{:?}", ctx);

    let context = Arc::new(Mutex::new(ctx));
    let stats = Arc::new(Mutex::new(Stats::build()));

    let (tx, rx) = mpsc::channel(32);
    let tx2 = tx.clone();
    let ctx = context.clone();

    tokio::spawn(async move {
        upstream_probe_handler(rx, ctx).await;
    });

    for ups_addr in upstreams.iter() {
        let upc = UpstreamProbeConfiguration::build_default(ups_addr);
        match tx2.send(Command::Probe { upc }).await {
            Ok(_) => log::debug!("Sent Probe command to probe handler for address {:?}", ups_addr),
            Err(error) => log::error!("Error sending message to probe handler {:?}", error),
        }
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

async fn graceful_quit() {
    tokio::signal::ctrl_c()
        .await
        .expect("Could not install graceful quit signal handler");
    log::info!("Shutting down Hapi. Bye :-)")
}

fn identify_client(remote_addr: &SocketAddr, _request: &Request<Body>) -> String {
    remote_addr.ip().to_string()
}

fn test_route_1() -> Route {
    Route::build(
        String::from("Test 1"),
        vec![String::from("GET")],
        vec![String::from("/test")],
        vec![
            Upstream::build_from_fqdn("localhost:8001"),
            Upstream::build_from_fqdn("localhost:8002"),
        ],
        Box::new(RoundRobinUpstreamStrategy::build(0)),
    )
}

fn test_route_2() -> Route {
    Route::build(
        String::from("Test 2"),
        vec![String::from("GET")],
        vec![String::from("/test2")],
        vec![
            Upstream::build_from_fqdn("localhost:8001"),
            Upstream::build_from_fqdn("localhost:8002"),
        ],
        Box::new(AlwaysFirstUpstreamStrategy::build()),
    )
}
