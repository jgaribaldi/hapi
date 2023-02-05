use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;
use crate::errors::HapiError;
use crate::events::commands::Command;
use crate::events::commands::Command::{CountStat, LookupStats};
use crate::events::events::Event;
use crate::events::events::Event::{StatsWereFound, StatWasCounted};
use crate::modules::stats::Stats;

pub(crate) async fn handle_stats(mut recv_cmd: Receiver<Command>, send_evt: Sender<Event>) {
    let mut stats = Stats::build();

    while let Ok(command) = recv_cmd.recv().await {
        let maybe_event = match command {
            CountStat { id } => {
                // TODO: fix
                stats.count_request("client", "method", "path", "upstream");
                Some(StatWasCounted { cmd_id: id })
            },
            LookupStats { id } => {
                let stats = stats.get_all();
                Some(StatsWereFound { cmd_id: id, stats })
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

pub(crate) struct StatsClient {
    send_cmd: Sender<Command>,
    recv_evt: Receiver<Event>,
}

impl StatsClient {
    pub fn build(send_cmd: Sender<Command>, recv_evt: Receiver<Event>) -> Self {
        Self {
            send_cmd,
            recv_evt,
        }
    }

    pub async fn get_all_stats(&mut self) -> Result<Vec<(String, String, String, String, u64)>, HapiError> {
        let cmd_uuid = Uuid::new_v4();
        let command = LookupStats { id: cmd_uuid.to_string() };
        self.send_cmd.send(command)?;

        loop {
            match self.recv_evt.recv().await {
                Ok(event) => {
                    log::debug!("Received event {:?}", event);
                    match event {
                        StatsWereFound { cmd_id, stats } => {
                            if cmd_id == cmd_uuid.to_string() {
                                break Ok(stats)
                            }
                        },
                        _ => {},
                    }
                },
                Err(error) => {
                    log::warn!("Error receiving message {:?}", error);
                    break Err(HapiError::MessageReceiveError(error))
                },
            }
        }
    }
}