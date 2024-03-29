use tokio::sync::broadcast::{Receiver, Sender};
use uuid::Uuid;

use crate::errors::HapiError;
use crate::events::commands::Command;
use crate::events::commands::Command::{
    AddRoute, DisableUpstream, EnableUpstream, LookupAllRoutes, LookupAllUpstreams, LookupRoute,
    LookupUpstream, RemoveRoute,
};
use crate::events::events::Event;
use crate::events::events::Event::{
    RouteWasAdded, RouteWasFound, RouteWasNotAdded, RouteWasNotFound, RouteWasNotRemoved,
    RouteWasRemoved, RoutesWereFound, UpstreamWasDisabled, UpstreamWasEnabled, UpstreamWasFound,
    UpstreamWasNotFound, UpstreamsWereFound,
};
use crate::modules::core::context::Context;
use crate::modules::core::route::Route;
use crate::modules::core::upstream::UpstreamAddress;
use crate::repositories::jsonfile::JsonFile;

pub(crate) async fn handle_core(mut recv_cmd: Receiver<Command>, send_evt: Sender<Event>) {
    let db = JsonFile::build("db.json")
        .expect("Could not find 'db.json' file");
    let mut context = load_json_file_db(db, send_evt.clone())
        .expect("Could not create context from 'db.json' file");

    while let Ok(command) = recv_cmd.recv().await {
        log::debug!("Received command {:?}", command);
        let maybe_event = match command {
            LookupUpstream {
                id,
                client,
                path,
                method,
            } => {
                match context.upstream_lookup(path.as_str(), method.as_str()) {
                    Ok(maybe_upstream) => match maybe_upstream {
                        Some(upstream) => Some(UpstreamWasFound {
                            cmd_id: id.clone(),
                            upstream_address: upstream.address.clone(),
                            client,
                            path,
                            method,
                        }),
                        None => Some(UpstreamWasNotFound { cmd_id: id.clone() }),
                    },
                    Err(_error) => None, // TODO: map error to proper event
                }
            }
            AddRoute { id, route } => match context.add_route(route.clone()) {
                Ok(_) => Some(RouteWasAdded { cmd_id: id, route }),
                Err(error) => Some(RouteWasNotAdded {
                    cmd_id: id,
                    route,
                    error,
                }),
            },
            RemoveRoute { id, route_id } => match context.remove_route(route_id.as_str()) {
                Ok(removed_route) => Some(RouteWasRemoved {
                    cmd_id: id,
                    route: removed_route,
                }),
                Err(error) => Some(RouteWasNotRemoved {
                    cmd_id: id,
                    route_id,
                    error,
                }),
            },
            EnableUpstream {
                id,
                upstream_address,
            } => {
                match context.enable_upstream_for_all_routes(&upstream_address) {
                    Ok(_) => Some(UpstreamWasEnabled {
                        cmd_id: id,
                        upstream_address,
                    }),
                    Err(_error) => None, // TODO: map error to proper event
                }
            }
            DisableUpstream {
                id,
                upstream_address,
            } => {
                match context.disable_upstream_for_all_routes(&upstream_address) {
                    Ok(_) => Some(UpstreamWasDisabled {
                        cmd_id: id,
                        upstream_address,
                    }),
                    Err(_error) => None, // TODO: map error to proper event
                }
            }
            LookupAllRoutes { id } => {
                match context.get_all_routes() {
                    Ok(found_routes) => {
                        let mut all_routes = Vec::new();
                        for r in found_routes {
                            all_routes.push(r.clone());
                        }
                        Some(RoutesWereFound {
                            cmd_id: id,
                            routes: all_routes,
                        })
                    }
                    Err(_error) => None, // TODO: map error to proper event
                }
            }
            LookupRoute { id, route_id } => {
                match context.get_route_by_id(route_id.as_str()) {
                    Ok(maybe_route) => match maybe_route {
                        Some(route) => Some(RouteWasFound {
                            cmd_id: id,
                            route: route.clone(),
                        }),
                        None => Some(RouteWasNotFound {
                            cmd_id: id,
                            route_id,
                        }),
                    },
                    Err(_error) => None, // TODO: map error to proper event
                }
            }
            LookupAllUpstreams { id } => {
                match context.get_all_upstreams() {
                    Ok(upstreams) => {
                        let found: Vec<UpstreamAddress> = upstreams.iter().map(|u| u.address.clone()).collect();
                        Some(UpstreamsWereFound {
                            cmd_id: id,
                            upstreams: found,
                        })
                    },
                    Err(_error) => None, // TODO: map error to proper event
                }
            }
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
        Self { send_cmd, recv_evt }
    }

    pub async fn get_routes(&mut self) -> Result<Vec<Route>, HapiError> {
        let cmd_uuid = Uuid::new_v4();
        let command = LookupAllRoutes {
            id: cmd_uuid.to_string(),
        };
        self.send_cmd.send(command)?;

        loop {
            match self.recv_evt.recv().await {
                Ok(event) => {
                    log::debug!("Received event {:?}", event);
                    match event {
                        RoutesWereFound { cmd_id, routes } => {
                            if cmd_id == cmd_uuid.to_string() {
                                break Ok(routes);
                            }
                        }
                        _ => {}
                    }
                }
                Err(error) => {
                    log::warn!("Error receiving message {:?}", error);
                    break Err(HapiError::MessageReceiveError(error));
                }
            }
        }
    }

    pub async fn get_route_by_id(&mut self, route_id: &str) -> Result<Option<Route>, HapiError> {
        let cmd_uuid = Uuid::new_v4();
        let command = LookupRoute {
            id: cmd_uuid.to_string(),
            route_id: route_id.to_string(),
        };
        self.send_cmd.send(command)?;

        loop {
            match self.recv_evt.recv().await {
                Ok(event) => {
                    log::debug!("Received event {:?}", event);
                    match event {
                        RouteWasFound { cmd_id, route } => {
                            if cmd_id == cmd_uuid.to_string() {
                                break Ok(Some(route));
                            }
                        }
                        RouteWasNotFound { cmd_id, .. } => {
                            if cmd_id == cmd_uuid.to_string() {
                                break Ok(None);
                            }
                        }
                        _ => {}
                    }
                }
                Err(error) => {
                    log::warn!("Error receiving message {:?}", error);
                    break Err(HapiError::MessageReceiveError(error));
                }
            }
        }
    }

    pub async fn search_upstream(
        &mut self,
        client: &str,
        path: &str,
        method: &str,
    ) -> Result<Option<UpstreamAddress>, HapiError> {
        let cmd_uuid = Uuid::new_v4();
        let command = LookupUpstream {
            id: cmd_uuid.to_string(),
            client: client.to_string(),
            path: path.to_string(),
            method: method.to_string(),
        };
        self.send_cmd.send(command)?;

        loop {
            match self.recv_evt.recv().await {
                Ok(event) => {
                    log::debug!("Received event {:?}", event);
                    match event {
                        UpstreamWasFound {
                            cmd_id,
                            upstream_address,
                            ..
                        } => {
                            if cmd_id == cmd_uuid.to_string() {
                                break Ok(Some(upstream_address.clone()));
                            }
                        }
                        UpstreamWasNotFound { cmd_id } => {
                            if cmd_id == cmd_uuid.to_string() {
                                break Ok(None);
                            }
                        }
                        _ => {}
                    }
                }
                Err(error) => {
                    log::warn!("Error receiving message {:?}", error);
                    break Err(HapiError::MessageReceiveError(error));
                }
            }
        }
    }

    pub async fn add_route(&mut self, route: Route) -> Result<(), HapiError> {
        // TODO: change return type to Result<Route, HapiError>
        let cmd_uuid = Uuid::new_v4();
        let command = AddRoute {
            id: cmd_uuid.to_string(),
            route,
        };
        self.send_cmd.send(command)?;

        loop {
            match self.recv_evt.recv().await {
                Ok(event) => {
                    log::debug!("Received event {:?}", event);
                    match event {
                        RouteWasAdded { cmd_id, .. } => {
                            if cmd_id == cmd_uuid.to_string() {
                                break Ok(());
                            }
                        }
                        RouteWasNotAdded { cmd_id, error, .. } => {
                            if cmd_id == cmd_uuid.to_string() {
                                break Err(HapiError::CoreError(error));
                            }
                        }
                        _ => {}
                    }
                }
                Err(error) => {
                    log::warn!("Error receiving message {:?}", error);
                    break Err(HapiError::MessageReceiveError(error));
                }
            }
        }
    }

    pub async fn remove_route(&mut self, route_id: &str) -> Result<Route, HapiError> {
        let cmd_uuid = Uuid::new_v4();
        let command = RemoveRoute {
            id: cmd_uuid.to_string(),
            route_id: route_id.to_string(),
        };
        self.send_cmd.send(command)?;

        loop {
            match self.recv_evt.recv().await {
                Ok(event) => {
                    log::debug!("Received event {:?}", event);
                    match event {
                        RouteWasRemoved { cmd_id, route } => {
                            if cmd_id == cmd_uuid.to_string() {
                                break Ok(route);
                            }
                        }
                        RouteWasNotRemoved { cmd_id, error, .. } => {
                            if cmd_id == cmd_uuid.to_string() {
                                break Err(HapiError::CoreError(error));
                            }
                        }
                        _ => {}
                    }
                }
                Err(error) => {
                    log::warn!("Error receiving message {:?}", error);
                    break Err(HapiError::MessageReceiveError(error));
                }
            }
        }
    }

    pub async fn get_upstreams(&mut self) -> Result<Vec<UpstreamAddress>, HapiError> {
        let cmd_uuid = Uuid::new_v4();
        let command = LookupAllUpstreams {
            id: cmd_uuid.to_string(),
        };
        self.send_cmd.send(command)?;

        loop {
            match self.recv_evt.recv().await {
                Ok(event) => {
                    log::debug!("Received event {:?}", event);
                    match event {
                        UpstreamsWereFound { cmd_id, upstreams } => {
                            if cmd_id == cmd_uuid.to_string() {
                                break Ok(upstreams);
                            }
                        }
                        _ => {}
                    }
                }
                Err(error) => {
                    log::warn!("Error receiving message {:?}", error);
                    break Err(HapiError::MessageReceiveError(error));
                }
            }
        }
    }
}

fn load_json_file_db(db: JsonFile, send_evt: Sender<Event>) -> Result<Context, HapiError> {
    let mut context = Context::build_empty();
    if db.routes.is_some() {
        for route in db.routes.unwrap().iter() {
            let r = Route::from(route.clone());
            context.add_route(r.clone())?;
            let event = RouteWasAdded { cmd_id: String::from("init_event"), route: r.clone() };
            send_evt.send(event)?;
        }
    }
    Ok(context)
}
