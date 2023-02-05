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
    pub api_ip_address: String,
    pub api_port: u16,
}

impl HapiSettings {
    pub fn load_from_file(file_relative_path: &str) -> Result<Self, HapiError> {
        let settings_file = File::open(Path::new(file_relative_path))?;
        let reader = BufReader::new(settings_file);
        let settings: HapiSettings = serde_json::from_reader(reader)?;
        Ok(settings)
    }

    pub fn server_socket_address(&self) -> Result<SocketAddr, HapiError> {
        let full_ip_address = socket_address(self.ip_address.as_str(), self.port);
        let result: SocketAddr = full_ip_address.parse()?;
        Ok(result)
    }

    pub fn api_socket_address(&self) -> Result<SocketAddr, HapiError> {
        let full_ip_address = socket_address(self.api_ip_address.as_str(), self.api_port);
        let result: SocketAddr = full_ip_address.parse()?;
        Ok(result)
    }

    pub fn routes(&self) -> Vec<crate::modules::core::route::Route> {
        let mut result = Vec::new();

        for r in self.routes.iter() {
            let route: crate::modules::core::route::Route = r.clone().into();
            result.push(route);
        }

        result
    }
}

fn socket_address(ip: &str, port: u16) -> String {
    let mut result = String::from(ip);
    result.push_str(":");
    result.push_str(port.to_string().as_str());
    result
}
