use crate::model::upstream::UpstreamAddress;
use crate::{HapiError, UpstreamProbeConfiguration};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct HapiSettings {
    pub ip_address: String,
    pub port: u16,
    pub routes: Vec<Route>,
    pub probes: Option<Vec<Probe>>,
}

impl HapiSettings {
    pub fn load_from_file(file_relative_path: &str) -> Result<Self, HapiError> {
        let settings_file = File::open(Path::new(file_relative_path))?;
        let reader = BufReader::new(settings_file);
        let settings: HapiSettings = serde_json::from_reader(reader)?;
        Ok(settings)
    }

    pub fn server_socket_address(&self) -> Result<SocketAddr, HapiError> {
        let mut full_ip_address = String::from(self.ip_address.as_str());
        full_ip_address.push_str(":");
        full_ip_address.push_str(self.port.to_string().as_str());

        let result: SocketAddr = full_ip_address.parse()?;
        Ok(result)
    }

    pub fn probes(&self) -> Vec<UpstreamProbeConfiguration> {
        let mut result = Vec::new();

        if let Some(probe_settings) = self.probes.as_ref() {
            for probe in probe_settings.iter() {
                let upc = UpstreamProbeConfiguration::build(
                    &UpstreamAddress::FQDN(probe.upstream_address.clone()),
                    probe.poll_interval_ms,
                    probe.error_count,
                    probe.success_count,
                );
                result.push(upc);
            }
        } else {
            for upstream_address in self.upstream_addresses() {
                let upc = UpstreamProbeConfiguration::build_default(&UpstreamAddress::FQDN(
                    upstream_address,
                ));
                result.push(upc);
            }
        }

        result
    }

    fn upstream_addresses(&self) -> Vec<String> {
        let mut result = HashSet::new();
        for route in self.routes.iter() {
            for upstream in route.upstreams.iter() {
                result.insert(upstream.clone());
            }
        }
        result.into_iter().collect()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Route {
    pub name: String,
    pub methods: Vec<String>,
    pub paths: Vec<String>,
    pub strategy: Strategy,
    pub upstreams: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Probe {
    pub upstream_address: String,
    pub poll_interval_ms: u64,
    pub error_count: u64,
    pub success_count: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Strategy {
    AlwaysFirst,
    RoundRobin,
}
