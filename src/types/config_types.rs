use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_path: String,
    pub listen_address: String,
    pub listen_port: u16,
    pub smart_meter_api_endpoint: String,
    pub ac_controller_endpoints: HashMap<String, AcControllerEndpointProperties>,
}

#[derive(Debug, Deserialize)]
pub struct AcControllerEndpointProperties {
    pub endpoint: String,
    pub api_key: String,
}
