use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

pub struct Stats {
    // (client, method, path, upstream) => count
    counter: HashMap<(String, String, String, String), u64>,
}

impl Stats {
    pub fn build() -> Self {
        Stats {
            counter: HashMap::new(),
        }
    }

    pub fn count_request(&mut self, client: &str, method: &str, path: &str, upstream: &str) {
        let key = (
            client.to_string(),
            method.to_string(),
            path.to_string(),
            upstream.to_string(),
        );
        *self.counter.entry(key).or_insert(0) += 1;
    }

    pub fn get_all(&self) -> Vec<(String, String, String, String, u64)> {
        let mut result = Vec::new();

        for entry in self.counter.iter() {
            result.push((
                entry.0 .0.clone(),
                entry.0 .1.clone(),
                entry.0 .2.clone(),
                entry.0 .3.clone(),
                *entry.1,
            ))
        }

        result
    }
}
