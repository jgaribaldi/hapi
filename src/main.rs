use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use hyper::Server;
use hyper::service::{make_service_fn, service_fn};
use crate::infrastructure::Infrastructure;
use crate::model::{Context, Route};

mod model;
mod infrastructure;

#[tokio::main]
async fn main() {
    // TODO: remove unwrap()
    simple_logger::init_with_env().unwrap();

    log::info!("This is Hapi, the Happy API");
    let context = initialize_context();

    let model = Arc::new(context);
    let infrastructure = Infrastructure::build(model);

    let make_service = make_service_fn(move |_conn| {
        let infrastructure = infrastructure.clone();

        let service = service_fn(move |request| {
            let infrastructure = infrastructure.clone();
            infrastructure.process_request(request)
        });
        async move { Ok::<_, Infallible>(service) }
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let server = Server::bind(&addr)
        .serve(make_service)
        .with_graceful_shutdown(graceful_quit());

    if let Err(e) = server.await {
        log::error!("server error: {}", e);
    }
}

fn initialize_context() -> Context {
    let mut context = Context::build();
    let route = Route::build("Test", &["GET"], &["/test"], &["localhost:8001"]);
    context.register_route(&route);
    log::info!("{:?}", context);
    context
}

async fn graceful_quit() {
    tokio::signal::ctrl_c()
        .await
        .expect("Could not install graceful quit signal handler");
    log::info!("Shutting down Hapi. Bye :-)")
}