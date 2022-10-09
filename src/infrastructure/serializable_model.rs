use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub struct Route {
    pub name: String,
    pub methods: Vec<String>,
    pub paths: Vec<String>,
    pub strategy: Strategy,
    pub upstreams: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Probe {
    pub upstream_address: String,
    pub poll_interval_ms: u64,
    pub error_count: u64,
    pub success_count: u64,
}

impl Probe {
    pub fn default(upstream_address: &str) -> Self {
        Probe {
            upstream_address: upstream_address.to_string(),
            poll_interval_ms: 1000,
            error_count: 5,
            success_count: 5,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Strategy {
    AlwaysFirst,
    RoundRobin,
}
