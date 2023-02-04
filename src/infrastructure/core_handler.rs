use futures_util::future::err;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::broadcast::error::SendError;
use uuid::Uuid;

use crate::errors::HapiError;
use crate::events::commands::Command;
use crate::events::events::Event;
use crate::events::events::Event::{RoutesWereFound, RouteWasAdded, RouteWasFound, RouteWasNotAdded, RouteWasNotFound, RouteWasNotRemoved, RouteWasRemoved, StatsWereFound, StatWasCounted, UpstreamWasDisabled, UpstreamWasEnabled, UpstreamWasFound, UpstreamWasNotFound};
use crate::infrastructure::settings::HapiSettings;
use crate::modules::core::context::Context;
use crate::modules::core::route::Route;
use crate::modules::core::upstream::UpstreamAddress;
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
                    Ok(_) => Some(RouteWasAdded { cmd_id: id, route }),
                    Err(e) => Some(RouteWasNotAdded { cmd_id: id, route }),
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
            Command::LookupRoute { id, route_id } => {
                match context.get_route_by_id(route_id.as_str()) {
                    Some(route) => Some(RouteWasFound { cmd_id: id, route: route.clone() }),
                    None => Some(RouteWasNotFound { cmd_id: id, route_id }),
                }
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

pub(crate) struct CoreClient {
    send_cmd: Sender<Command>,
    recv_evt: Receiver<Event>,
}

impl CoreClient {
    pub fn build(send_cmd: Sender<Command>, recv_evt: Receiver<Event>) -> Self {
        Self {
            send_cmd,
            recv_evt,
        }
    }

    pub async fn get_routes(&mut self) -> Result<Vec<Route>, HapiError> {
        let cmd_uuid = Uuid::new_v4();
        let command = Command::LookupAllRoutes { id: cmd_uuid.to_string() };
        self.send_cmd.send(command)?;

        let mut result = Vec::new();
        while let Ok(event) = self.recv_evt.recv().await {
            log::debug!("Received event {:?}", event);
            match event {
                RoutesWereFound { cmd_id, routes } => {
                    if cmd_id == cmd_uuid.to_string() {
                        result = routes;
                        break
                    }
                },
                _ => {},
            }
        };
        Ok(result)
    }

    pub async fn get_route_by_id(&mut self, route_id: &str) -> Result<Option<Route>, HapiError> {
        let cmd_uuid = Uuid::new_v4();
        let command = Command::LookupRoute { id: cmd_uuid.to_string(), route_id: route_id.to_string() };
        self.send_cmd.send(command)?;

        let mut result = None;
        while let Ok(event) = self.recv_evt.recv().await {
            log::debug!("Received event {:?}", event);
            match event {
                RouteWasFound { cmd_id, route} => {
                    if cmd_id == cmd_uuid.to_string() {
                        result = Some(route);
                        break
                    }
                },
                RouteWasNotFound { cmd_id, route_id } => {
                    if cmd_id == cmd_uuid.to_string() {
                        break
                    }
                },
                _ => {},
            }
        };
        Ok(result)
    }

    pub async fn search_upstream(&mut self, client: &str, path: &str, method: &str) -> Result<Option<UpstreamAddress>, HapiError> {
        let cmd_uuid = Uuid::new_v4();
        let command = Command::LookupUpstream { id: cmd_uuid.to_string(), client: client.to_string(), path: path.to_string(), method: method.to_string() };
        self.send_cmd.send(command)?;

        let mut result = None;
        while let Ok(event) = self.recv_evt.recv().await {
            log::debug!("Received event {:?}", event);
            match event {
                UpstreamWasFound { cmd_id, upstream_address } => {
                    if cmd_id == cmd_uuid.to_string() {
                        result = Some(upstream_address.clone());
                        break
                    }
                },
                UpstreamWasNotFound { cmd_id } => {
                    if cmd_id == cmd_uuid.to_string() {
                        break
                    }
                },
                _ => {},
            }
        };
        Ok(result)
    }

    pub async fn add_route(&mut self, route: Route) -> Result<(), HapiError> {
        let cmd_uuid = Uuid::new_v4();
        let command = Command::AddRoute { id: cmd_uuid.to_string(), route };
        self.send_cmd.send(command)?;

        let mut result = Ok(());
        while let Ok(event) = self.recv_evt.recv().await {
            log::debug!("Received event {:?}", event);
            match event {
                RouteWasAdded { cmd_id, route } => {
                    if cmd_id == cmd_uuid.to_string() {
                        break
                    }
                },
                RouteWasNotAdded { cmd_id, route } => {
                    if cmd_id == cmd_uuid.to_string() {
                        result = Err(HapiError::RouteAlreadyExists);
                        break
                    }
                },
                _ => {},
            }
        };
        result
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

