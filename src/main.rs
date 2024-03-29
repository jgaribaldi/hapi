use std::mem::size_of;
use std::net::SocketAddr;

use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Server};
use tokio::sync::broadcast;

use crate::errors::HapiError;
use crate::events::commands::Command;
use crate::events::events::Event;
use crate::infrastructure::core_handler::handle_core;
use crate::infrastructure::probe_handler::handle_probes;
use crate::infrastructure::processor::process_request;
use crate::infrastructure::settings::HapiSettings;
use crate::infrastructure::stats_handler::handle_stats;
use crate::interfaces::api::handle_api;

mod errors;
mod events;
mod infrastructure;
mod interfaces;
mod modules;
mod repositories;

#[tokio::main]
async fn main() -> Result<(), HapiError> {
    simple_logger::init_with_env()?;
    log::info!("This is Hapi, the Happy API");

    // commands channel
    let (send_cmd, _recv_cmd) = broadcast::channel(1024 * size_of::<Command>());
    // events channel
    let (send_evt, _recv_evt) = broadcast::channel(1024 * size_of::<Event>());

    // core handler
    let send_evt1 = send_evt.clone();
    let recv_cmd1 = send_cmd.subscribe();
    tokio::spawn(async move {
        handle_core(recv_cmd1, send_evt1).await;
    });

    // stats handler
    let send_evt2 = send_evt.clone();
    let recv_cmd2 = send_cmd.subscribe();
    let recv_evt2 = send_evt.subscribe();
    tokio::spawn(async move {
        handle_stats(recv_cmd2, send_evt2, recv_evt2).await;
    });

    // probes handler
    let recv_evt3 = send_evt.subscribe();
    let send_cmd3 = send_cmd.clone();
    tokio::spawn(async move {
        handle_probes(recv_evt3, send_cmd3).await;
    });

    let send_cmd4 = send_cmd.clone();
    let send_evt4 = send_evt.clone();
    let make_service = make_service_fn(move |conn: &AddrStream| {
        let remote_addr = conn.remote_addr();
        let send_cmd4 = send_cmd4.clone();
        let send_evt4 = send_evt4.clone();

        let service = service_fn(move |request| {
            let client = identify_client(&remote_addr, &request);
            let send_cmd4 = send_cmd4.clone();
            let send_evt4 = send_evt4.clone();
            let recv_evt4 = send_evt4.subscribe();
            process_request(request, client, send_cmd4, recv_evt4)
        });
        async move { Ok::<_, HapiError>(service) }
    });

    let settings = HapiSettings::load_from_file("settings.json")?;
    let addr = settings.server_socket_address()?;
    let server = Server::bind(&addr)
        .serve(make_service)
        .with_graceful_shutdown(graceful_quit_handler());

    let make_api_service = make_service_fn(move |_conn| {
        let send_cmd5 = send_cmd.clone();
        let send_evt5 = send_evt.clone();
        let service = service_fn(move |request| {
            let send_cmd5 = send_cmd5.clone();
            let recv_evt5 = send_evt5.subscribe();
            handle_api(request, send_cmd5, recv_evt5)
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

async fn graceful_quit_handler() {
    tokio::signal::ctrl_c()
        .await
        .expect("Could not install graceful quit signal handler");

    log::info!("Shutting down Hapi. Bye :-)")
}

async fn api_graceful_quit_handler() {
    tokio::signal::ctrl_c()
        .await
        .expect("Could not install graceful quit signal handler");

    log::info!("Shutting down API server. Bye :-)")
}
