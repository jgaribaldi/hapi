use crate::errors::HapiError;
use crate::infrastructure::serializable_model::Route;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct JsonFile {
    pub routes: Option<Vec<Route>>,
}

impl JsonFile {
    pub fn build(file_relative_path: &str) -> Result<Self, HapiError> {
        let routes_file = File::open(Path::new(file_relative_path))?;
        let reader = BufReader::new(routes_file);
        let routes: JsonFile = serde_json::from_reader(reader)?;
        Ok(routes)
    }
}
