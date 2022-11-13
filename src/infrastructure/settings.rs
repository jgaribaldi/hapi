use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;

use crate::infrastructure::serializable_model::{Probe, Route};
use crate::HapiError;

#[derive(Serialize, Deserialize, Debug)]
pub struct HapiSettings {
    pub ip_address: String,
    pub port: u16,
    pub probes: Option<Vec<Probe>>,
    routes: Vec<Route>,
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

    pub fn api_socket_address(&self) -> Result<SocketAddr, HapiError> {
        let api_port = self.port + 1;
        let mut full_ip_address = String::from(self.ip_address.as_str());
        full_ip_address.push_str(":");
        full_ip_address.push_str(api_port.to_string().as_str());

        let result: SocketAddr = full_ip_address.parse()?;
        Ok(result)
    }

    pub fn routes(&self) -> Vec<crate::model::route::Route> {
        let mut result = Vec::new();

        for r in self.routes.iter() {
            let route: crate::model::route::Route = r.clone().into();
            result.push(route);
        }

        result
    }
}
