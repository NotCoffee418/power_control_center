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
