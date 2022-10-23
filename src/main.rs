use std::mem::size_of;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Server};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

use crate::errors::HapiError;
use crate::infrastructure::settings::HapiSettings;
use crate::infrastructure::stats::Stats;
use crate::infrastructure::upstream_probe::{upstream_probe_handler, Command};
use crate::infrastructure::{api, processor};
use crate::model::context::Context;

mod errors;
mod infrastructure;
mod model;

#[tokio::main]
async fn main() -> Result<(), HapiError> {
    simple_logger::init_with_env()?;
    log::info!("This is Hapi, the Happy API");

    let settings = HapiSettings::load_from_file("settings.json")?;
    log::info!("Settings {:?}", settings);

    let context = build_context_from_settings(&settings);

    let thread_safe_context = Arc::new(Mutex::new(context));
    let gqh_thread_safe_context = thread_safe_context.clone();
    let uph_thread_safe_context = thread_safe_context.clone();
    let api_thread_safe_context = thread_safe_context.clone();

    let thread_safe_stats = Arc::new(Mutex::new(Stats::build()));
    let api_thread_safe_stats = thread_safe_stats.clone();
    let (main_cmd_tx, probe_handler_cmd_rx) = mpsc::channel(1024 * size_of::<Command>());

    // spawn upstream probe handler thread
    tokio::spawn(async move {
        upstream_probe_handler(probe_handler_cmd_rx, uph_thread_safe_context).await;
    });

    // send commands to probe current upstreams
    for probe in settings.probes().iter() {
        match main_cmd_tx
            .send(Command::Probe {
                probe: probe.clone(),
            })
            .await
        {
            Ok(_) => log::debug!(
                "Sent Probe command to probe handler for address {:?}",
                probe.upstream_address
            ),
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

    let make_api_service = make_service_fn(move |_conn| {
        let context = api_thread_safe_context.clone();
        let stats = api_thread_safe_stats.clone();
        let service = service_fn(move |request| {
            let context = context.clone();
            let stats = stats.clone();
            api::process_request(context, stats, request)
        });
        async move { Ok::<_, HapiError>(service) }
    });

    let api_addr = settings.api_socket_address()?;
    let api_server = Server::bind(&api_addr)
        .serve(make_api_service)
        .with_graceful_shutdown(api_graceful_quit_handler());

    let _ret = futures_util::future::join(server, api_server).await;
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
            .send(Command::StopProbe {
                upstream_address: ups.to_string(),
            })
            .await
        {
            Ok(_) => log::debug!("Sent Probe command to probe handler for address {:?}", ups),
            Err(error) => log::error!("Error sending message to probe handler {:?}", error),
        }
    }
    log::info!("Shutting down Hapi. Bye :-)")
}

async fn api_graceful_quit_handler() {
    tokio::signal::ctrl_c()
        .await
        .expect("Could not install graceful quit signal handler");
}

fn build_context_from_settings(settings: &HapiSettings) -> Context {
    let mut context = Context::build_empty();
    for r in settings.routes() {
        context.add_route(r).expect("Error adding route");
    }
    context
}
