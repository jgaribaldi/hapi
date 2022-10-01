use std::mem::size_of;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Server};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

use crate::errors::HapiError;
use crate::infrastructure::processor;
use crate::infrastructure::settings::HapiSettings;
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

    let settings = HapiSettings::load_from_file("src/settings.json")?;
    log::info!("Settings {:?}", settings);

    let context = build_context_from_test_routes();
    log::info!("{:?}", context);
    let upstreams_to_probe = context.get_all_upstreams();

    let thread_safe_context = Arc::new(Mutex::new(context));
    let gqh_thread_safe_context = thread_safe_context.clone();
    let uph_thread_safe_context = thread_safe_context.clone();

    let thread_safe_stats = Arc::new(Mutex::new(Stats::build()));
    let (main_cmd_tx, probe_handler_cmd_rx) = mpsc::channel(1024 * size_of::<Command>());

    // spawn upstream probe handler thread
    tokio::spawn(async move {
        upstream_probe_handler(probe_handler_cmd_rx, uph_thread_safe_context).await;
    });

    // send commands to probe current upstreams
    for ups in upstreams_to_probe.iter() {
        let upc = UpstreamProbeConfiguration::build_default(ups);
        match main_cmd_tx.send(Command::Probe { upc }).await {
            Ok(_) => log::debug!("Sent Probe command to probe handler for address {:?}", ups),
            Err(error) => log::error!("Error sending message to probe handler {:?}", error),
        }
    }

    let make_service = make_service_fn(move |conn: &AddrStream| {
        let context = thread_safe_context.clone();
        let stats = thread_safe_stats.clone();
        let remote_addr = conn.remote_addr();

        let service = service_fn(move |request| {
            let client = identify_client(&remote_addr, &request);
            processor::process_request(context.clone(), request, stats.clone(), client)
        });
        async move { Ok::<_, HapiError>(service) }
    });

    let addr = settings.server_socket_address()?;
    let server = Server::bind(&addr)
        .serve(make_service)
        .with_graceful_shutdown(graceful_quit_handler(
            main_cmd_tx.clone(),
            gqh_thread_safe_context,
        ));

    if let Err(e) = server.await {
        log::error!("server error: {}", e);
    }
    Ok(())
}

fn identify_client(remote_addr: &SocketAddr, _request: &Request<Body>) -> String {
    remote_addr.ip().to_string()
}

async fn graceful_quit_handler(
    gqh_cmd_tx: Sender<Command>,
    gqh_thread_safe_context: Arc<Mutex<Context>>,
) {
    tokio::signal::ctrl_c()
        .await
        .expect("Could not install graceful quit signal handler");

    let mut upstream_addresses = Vec::new();
    {
        let ctx = gqh_thread_safe_context.lock().unwrap();
        upstream_addresses.extend_from_slice(&ctx.get_all_upstreams().as_slice());
    }

    for ups in upstream_addresses.iter() {
        match gqh_cmd_tx
            .send(Command::StopProbe { ups: ups.clone() })
            .await
        {
            Ok(_) => log::debug!("Sent Probe command to probe handler for address {:?}", ups),
            Err(error) => log::error!("Error sending message to probe handler {:?}", error),
        }
    }
    log::info!("Shutting down Hapi. Bye :-)")
}

fn build_context_from_test_routes() -> Context {
    let mut context = Context::build_empty();
    context.add_route(test_route_1());
    context.add_route(test_route_2());
    context
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
