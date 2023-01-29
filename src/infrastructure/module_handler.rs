use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::broadcast::error::SendError;
use uuid::Uuid;

use crate::errors::HapiError;
use crate::events::commands::Command;
use crate::events::events::Event;
use crate::events::events::Event::{RoutesWereFound, RouteWasAdded, RouteWasNotAdded, RouteWasNotRemoved, RouteWasRemoved, StatsWereFound, StatWasCounted, UpstreamWasDisabled, UpstreamWasEnabled, UpstreamWasFound, UpstreamWasNotFound};
use crate::infrastructure::settings::HapiSettings;
use crate::modules::core::context::Context;
use crate::modules::stats::Stats;

pub(crate) async fn handle_core(
    mut recv_cmd: Receiver<Command>,
    send_evt: Sender<Event>,
    send_cmd: Sender<Command>,
) {
    // TODO: remove .unwrap()
    let settings = HapiSettings::load_from_file("settings.json").unwrap();
    log::info!("Settings {:?}", settings);

    // TODO: remove .unwrap()
    let mut context = build_context_from_settings(&settings, send_cmd).unwrap();

    while let Ok(command) = recv_cmd.recv().await {
        log::debug!("Received command {:?}", command);
        let maybe_event = match command {
            Command::LookupUpstream { id, client, path, method } => {
                context.upstream_lookup(path.as_str(), method.as_str())
                    .map(|upstream| {
                        UpstreamWasFound { cmd_id: id.clone(), upstream_address: upstream }
                    })
                    .or(Some(UpstreamWasNotFound { cmd_id: id.clone() }))
            },
            Command::AddRoute { id, route } => {
                match context.add_route(route.clone()) {
                    Ok(_) => Some(RouteWasAdded { cmd_id: id, route}),
                    Err(_e) => Some(RouteWasNotAdded { cmd_id: id, route }),
                }
            },
            Command::RemoveRoute { id, route_id } => {
                match context.remove_route(route_id.as_str()) {
                    Ok(removed_route) => Some(RouteWasRemoved { cmd_id: id, route: removed_route }),
                    Err(_e) => Some(RouteWasNotRemoved { cmd_id: id, route_id }),
                }

            },
            Command::EnableUpstream { id, upstream_address } => {
                context.enable_upstream_for_all_routes(&upstream_address);
                Some(UpstreamWasEnabled { cmd_id: id, upstream_address })
            },
            Command::DisableUpstream { id, upstream_address } => {
                context.disable_upstream_for_all_routes(&upstream_address);
                Some(UpstreamWasDisabled { cmd_id: id, upstream_address })
            },
            Command::LookupAllRoutes { id} => {
                let mut all_routes = Vec::new();
                for r in context.get_all_routes() {
                    all_routes.push(r.clone());
                }
                Some(RoutesWereFound { cmd_id: id, routes: all_routes })
            },
            _ => None,
        };

        if let Some(event) = maybe_event {
            match send_evt.send(event) {
                Ok(_) => log::debug!("Event sent"),
                Err(e) => log::error!("Error sending event {}", e),
            }
        }
    }
}

fn build_context_from_settings(settings: &HapiSettings, send_cmd: Sender<Command>) -> Result<Context, HapiError> {
    let mut context = Context::build_empty();
    for r in settings.routes() {
        let command = Command::AddRoute { id: Uuid::new_v4().to_string(), route: r.clone() };
        match send_cmd.send(command) {
            Ok(_) => log::debug!("Command sent"),
            Err(e) => log::error!("Error sending command {}", e),
        }
    }
    Ok(context)
}

pub(crate) async fn handle_stats(mut recv_cmd: Receiver<Command>, send_evt: Sender<Event>) {
    let mut stats = Stats::build();

    while let Ok(command) = recv_cmd.recv().await {
        let maybe_event = match command {
            Command::CountStat { id } => {
                // TODO: fix
                stats.count_request("client", "method", "path", "upstream");
                Some(StatWasCounted { cmd_id: id })
            },
            Command::LookupStats { id } => Some(StatsWereFound { cmd_id: id }),
            _ => None,
        };

        if let Some(event) = maybe_event {
            match send_evt.send(event) {
                Ok(_) => log::debug!("Event sent"),
                Err(e) => log::error!("Error sending event {}", e),
            }
        }
    }
}
