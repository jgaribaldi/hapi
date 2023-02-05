use crate::errors::HapiError;
use crate::infrastructure::serializable_model::Route;
use crate::infrastructure::probe::Command;
use crate::infrastructure::probe::Command::RebuildProbes;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use crate::modules::core::context::Context;

// pub async fn resolve_hapi_request(
//     request: Request<Body>,
//     client: String,
// ) -> Result<Response<Body>, HapiError> {
//     log::debug!("Received: {:?}", &request);
//     let method = request.method().to_string();
//     let path = request.uri().path().to_string();
//
//     let (response, maybe_upstream) = process_request(request).await?;
//     if let Some(upstream) = maybe_upstream {
//         stats::count_request(
//             stats,
//             client.as_str(),
//             method.as_str(),
//             path.as_str(),
//             upstream.to_string().as_str(),
//         )
//         .await;
//     }
//     Ok(response)
// }

pub async fn add_route(
    context: Arc<Mutex<Context>>,
    route: Route,
    cmd_tx: Sender<Command>,
) -> Result<(), HapiError> {
    // do_add_route(context, route)?;
    rebuild_probes(&cmd_tx).await;
    Ok(())
}

pub async fn delete_route(
    context: Arc<Mutex<Context>>,
    route_id: &str,
    cmd_tx: Sender<Command>,
) -> Result<(), HapiError> {
    // do_delete_route(context, route_id)?;
    rebuild_probes(&cmd_tx).await;
    Ok(())
}

// fn do_add_route(context: Arc<Mutex<Context>>, route_to_add: Route) -> Result<(), HapiError> {
//     let mut ctx = context.lock().unwrap();
//     let r = crate::modules::core::route::Route::from(route_to_add);
//     ctx.add_route(r)
// }
//
// fn do_delete_route(context: Arc<Mutex<Context>>, route_id: &str) -> Result<crate::modules::core::route::Route, HapiError> {
//     let mut ctx = context.lock().unwrap();
//     ctx.remove_route(route_id)
// }

async fn rebuild_probes(cmd_tx: &Sender<Command>) {
    match cmd_tx.send(RebuildProbes).await {
        Ok(_) => log::debug!("Sent RebuildProbes command to probe handler"),
        Err(e) => log::error!(
            "Error sending RebuildProbes command to probe handler {:?}",
            e
        ),
    }
}
