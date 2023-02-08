use crate::errors::HapiError;
use crate::events::commands::Command;
use crate::events::commands::Command::LookupStats;
use crate::events::events::Event;
use crate::events::events::Event::{StatsWereFound, UpstreamWasFound};
use crate::modules::stats::Stats;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast::{Receiver, Sender};
use uuid::Uuid;

pub(crate) async fn handle_stats(
    mut recv_cmd: Receiver<Command>,
    send_evt: Sender<Event>,
    recv_evt: Receiver<Event>,
) {
    let stats = Arc::new(Mutex::new(Stats::build()));
    let stats2 = stats.clone();

    tokio::spawn(async move {
        let stats = stats.clone();
        event_listener(recv_evt, stats).await;
    });

    while let Ok(command) = recv_cmd.recv().await {
        let maybe_event = match command {
            LookupStats { id } => {
                let sts = stats2.lock().unwrap();
                let result = sts.get_all();
                Some(StatsWereFound {
                    cmd_id: id,
                    stats: result,
                })
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

pub(crate) struct StatsClient {
    send_cmd: Sender<Command>,
    recv_evt: Receiver<Event>,
}

impl StatsClient {
    pub fn build(send_cmd: Sender<Command>, recv_evt: Receiver<Event>) -> Self {
        Self { send_cmd, recv_evt }
    }

    pub async fn get_all_stats(
        &mut self,
    ) -> Result<Vec<(String, String, String, String, u64)>, HapiError> {
        let cmd_uuid = Uuid::new_v4();
        let command = LookupStats {
            id: cmd_uuid.to_string(),
        };
        self.send_cmd.send(command)?;

        loop {
            match self.recv_evt.recv().await {
                Ok(event) => {
                    log::debug!("Received event {:?}", event);
                    match event {
                        StatsWereFound { cmd_id, stats } => {
                            if cmd_id == cmd_uuid.to_string() {
                                break Ok(stats);
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

async fn event_listener(mut recv_evt: Receiver<Event>, stats: Arc<Mutex<Stats>>) {
    while let Ok(event) = recv_evt.recv().await {
        match event {
            UpstreamWasFound {
                upstream_address,
                client,
                path,
                method,
                ..
            } => {
                let mut sts = stats.lock().unwrap();
                sts.count_request(
                    client.as_str(),
                    method.as_str(),
                    path.as_str(),
                    upstream_address.to_string().as_str(),
                )
            }
            _ => {}
        }
    }
}
