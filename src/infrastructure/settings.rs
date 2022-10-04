use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::iter::FromIterator;
use std::net::SocketAddr;
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;

use crate::{AlwaysFirstUpstreamStrategy, HapiError, RoundRobinUpstreamStrategy, Upstream};

#[derive(Serialize, Deserialize, Debug)]
pub struct HapiSettings {
    pub ip_address: String,
    pub port: u16,
    routes: Vec<Route>,
    probes: Option<Vec<Probe>>,
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

    pub fn probes(&self) -> Vec<Probe> {
        match self.probes.as_ref() {
            Some(probe_settings) => {
                let mut probes = Vec::new();
                for ps in probe_settings {
                    probes.push(ps.clone())
                }
                let upstreams = self.upstream_addresses();

                // create default probe configuration for missing upstreams, that is upstreams
                // found in the "routes" section but not in the "probes" section of the settings
                // file
                let missing_upstreams = upstream_difference(&probes, &upstreams);
                if missing_upstreams.len() > 0 {
                    for mu in missing_upstreams {
                        probes.push(Probe::default(&mu));
                    }
                }
                probes
            }
            None => {
                let mut probes = Vec::new();
                for upstream in self.upstream_addresses() {
                    probes.push(Probe::default(upstream.as_str()));
                }
                probes
            }
        }
    }

    pub fn routes(&self) -> Vec<crate::model::route::Route> {
        let mut result = Vec::new();

        for r in self.routes.iter() {
            let name = r.name.clone();
            let methods = r.methods.clone();
            let paths = r.paths.clone();
            let upstreams: Vec<Upstream> = r
                .upstreams
                .iter()
                .map(|ups_addr| Upstream::build_from_fqdn(ups_addr.as_str()))
                .collect();

            match r.strategy {
                Strategy::AlwaysFirst => result.push(crate::model::route::Route::build(
                    name,
                    methods,
                    paths,
                    upstreams,
                    Box::new(AlwaysFirstUpstreamStrategy::build()),
                )),
                Strategy::RoundRobin => result.push(crate::model::route::Route::build(
                    name,
                    methods,
                    paths,
                    upstreams,
                    Box::new(RoundRobinUpstreamStrategy::build(0)),
                )),
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

fn upstream_difference(found_probes: &Vec<Probe>, found_upstreams: &Vec<String>) -> Vec<String> {
    let probe_set: HashSet<String> = HashSet::from_iter(probe_to_upstream_address(found_probes));
    let upstream_set: HashSet<String> = HashSet::from_iter(found_upstreams.clone());

    let mut result = Vec::new();
    for upstream in upstream_set.difference(&probe_set) {
        result.push(upstream.to_string());
    }
    result
}

fn probe_to_upstream_address(probes: &Vec<Probe>) -> Vec<String> {
    let mut result = Vec::new();
    for p in probes {
        result.push(p.upstream_address.clone());
    }
    result
}

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
