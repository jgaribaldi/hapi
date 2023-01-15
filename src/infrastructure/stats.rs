use std::sync::{Arc, Mutex};
use crate::modules::stats::Stats;

pub async fn count_request(
    stats: Arc<Mutex<Stats>>,
    client: &str,
    method: &str,
    path: &str,
    upstream: &str,
) {
    let mut sts = stats.lock().unwrap();
    sts.count_request(client, method, path, upstream)
}

