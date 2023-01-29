use tokio::sync::broadcast::{Receiver, Sender};
use crate::events::commands::Command;
use crate::events::events::Event;
use crate::events::events::Event::{StatsWereFound, StatWasCounted};
use crate::modules::stats::Stats;

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