use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_path: String,
    pub listen_address: String,
    pub listen_port: u16,
    pub smart_meter_api_endpoint: String,
    pub ac_controller_endpoints: HashMap<String, AcControllerEndpointProperties>,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default = "default_pir_api_key")]
    pub pir_api_key: String,
    #[serde(default = "default_pir_timeout_minutes")]
    pub pir_timeout_minutes: u32,
}

fn default_pir_api_key() -> String {
    String::new()
}

fn default_pir_timeout_minutes() -> u32 {
    5
}

#[derive(Debug, Deserialize)]
pub struct AcControllerEndpointProperties {
    pub endpoint: String,
    pub api_key: String,
}
