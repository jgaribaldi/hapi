use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::sleep;

use crate::Context;

pub async fn probe_upstreams(context: Arc<Mutex<Context>>) {
    loop {
        sleep(Duration::from_secs(5)).await;

        let upstreams;
        {
            let ctx = context.lock().unwrap();
            upstreams = ctx.get_upstreams();
        }

        for upstream in upstreams {
            let poll_result = TcpStream::connect(&upstream).await;

            match poll_result {
                Ok(_) => {
                    log::trace!("Upstream available {}", &upstream);
                }
                Err(_) => {
                    log::warn!("Upstream unavailable {} - Disabling", &upstream);
                    {
                        let mut ctx = context.lock().unwrap();
                        ctx.disable_upstream_for_all_routes(&upstream.as_str());
                    }
                }
            }
        }
    }
}
