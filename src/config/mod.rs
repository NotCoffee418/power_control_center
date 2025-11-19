use crate::types::*;
use serde_json;
use std::sync::OnceLock;

#[cfg(test)]
mod tests;

pub const CONFIG_FILE_PATH: &str = "/etc/power_control_center/config.json";
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Load and return a reference to the global configuration
/// Reuse existing config if already loaded from file.
/// Config cannot be changed at runtime.
pub fn get_config() -> &'static Config {
    CONFIG.get_or_init(|| {
        let config_str = std::fs::read_to_string(CONFIG_FILE_PATH).unwrap_or_else(|e| {
            panic!("Failed to read config file {}: {}", CONFIG_FILE_PATH, e);
        });
        get_config_from_json_str(&config_str)
    })
}

/// Parse configuration from a JSON string.
/// used by `get_config` and tests.
/// Panics if parsing fails.
fn get_config_from_json_str(json_str: &str) -> Config {
    serde_json::from_str(json_str).unwrap_or_else(|e| panic!("Failed to parse config JSON: {}", e))
}

#[cfg(test)]
impl Default for Config {
    fn default() -> Self {
        use std::collections::HashMap;

        Config {
            database_path: String::new(),
            listen_address: String::new(),
            listen_port: 0,
            smart_meter_api_endpoint: String::new(),
            ac_controller_endpoints: HashMap::new(),
            latitude: 0.0,
            longitude: 0.0,
        }
    }
}
/// Build config with custom values for unit tests.
#[cfg(test)]
impl Config {
    pub fn with_memory_db(mut self) -> Self {
        self.database_path = ":memory:".to_string();
        self
    }

    // Set global config to our customized test config.
    // Now callable by `get_config`
    pub fn build(self) -> &'static Config {
        CONFIG
            .set(self)
            .expect("set_test_config failed. Config already initialized.");
        CONFIG.get().unwrap()
    }
}
