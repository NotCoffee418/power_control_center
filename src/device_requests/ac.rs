use super::common;
use super::cache::DataCache;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::OnceLock;

// Public data types
#[derive(Debug, Deserialize, Clone)]
pub struct SensorData {
    pub temperature: f64,
    #[serde(rename = "isAutomaticMode")]
    pub is_automatic_mode: bool,
}

// Request types
#[derive(Debug, Serialize)]
pub struct TurnOnRequest {
    pub mode: i32,
    #[serde(rename = "fanSpeed")]
    pub fan_speed: i32,
    pub temperature: f64,
    pub swing: i32,
}

// Custom error type
#[derive(Debug)]
pub enum AcError {
    ApiError(String),
    NetworkError(reqwest::Error),
    EndpointNotFound(String),
}

impl fmt::Display for AcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AcError::ApiError(msg) => write!(f, "API Error: {}", msg),
            AcError::NetworkError(err) => write!(f, "Network Error: {}", err),
            AcError::EndpointNotFound(name) => {
                write!(f, "AC endpoint '{}' not found in config", name)
            }
        }
    }
}

impl std::error::Error for AcError {}

impl From<reqwest::Error> for AcError {
    fn from(err: reqwest::Error) -> Self {
        AcError::NetworkError(err)
    }
}

// API functions
pub async fn turn_off_ac(endpoint_name: &str) -> Result<bool, AcError> {
    let (base_url, api_key) = get_ac_endpoint_config(endpoint_name)?;

    info!("Turning off AC '{}'", endpoint_name);
    let url = format!("{}/api/ir/off", base_url);
    let client = common::get_client().await;

    let response = client
        .post(&url)
        .header("Authorization", format!("ApiKey {}", api_key))
        .send()
        .await?;

    handle_response(response).await
}

pub async fn turn_on_ac(
    endpoint_name: &str,
    mode: i32,
    fan_speed: i32,
    temperature: f64,
    swing: i32,
) -> Result<bool, AcError> {
    let (base_url, api_key) = get_ac_endpoint_config(endpoint_name)?;

    info!(
        "Turning on AC '{}': mode={}, fan_speed={}, temp={}°C, swing={}",
        endpoint_name, mode, fan_speed, temperature, swing
    );
    let url = format!("{}/api/ir/on", base_url);
    let request = TurnOnRequest {
        mode,
        fan_speed,
        temperature,
        swing,
    };
    let client = common::get_client().await;

    let response = client
        .post(&url)
        .header("Authorization", format!("ApiKey {}", api_key))
        .json(&request)
        .send()
        .await?;

    handle_response(response).await
}

pub async fn toggle_powerful(endpoint_name: &str) -> Result<bool, AcError> {
    let (base_url, api_key) = get_ac_endpoint_config(endpoint_name)?;

    info!("Toggling powerful mode for AC '{}'", endpoint_name);
    let url = format!("{}/api/ir/toggle-powerful", base_url);
    let client = common::get_client().await;

    let response = client
        .post(&url)
        .header("Authorization", format!("ApiKey {}", api_key))
        .send()
        .await?;

    handle_response(response).await
}

pub async fn get_sensors(endpoint_name: &str) -> Result<SensorData, AcError> {
    let (base_url, _api_key) = get_ac_endpoint_config(endpoint_name)?;

    debug!("Fetching sensor data from AC '{}'", endpoint_name);
    let url = format!("{}/api/sensors", base_url);
    let client = common::get_client().await;

    let response = client.get(&url).send().await?;

    handle_response(response).await
}

// Cache for sensor data (30 second TTL)
static SENSOR_CACHE: OnceLock<DataCache<SensorData>> = OnceLock::new();

fn get_sensor_cache() -> &'static DataCache<SensorData> {
    SENSOR_CACHE.get_or_init(|| DataCache::new(30))
}

/// Get sensor data with caching (30 second TTL)
/// Recommended for dashboard use to reduce API calls
pub async fn get_sensors_cached(endpoint_name: &str) -> Result<SensorData, AcError> {
    let cache = get_sensor_cache();
    let cache_key = format!("sensor_{}", endpoint_name);
    
    cache.get_or_fetch(&cache_key, || async {
        get_sensors(endpoint_name).await
    }).await
}

// Helper to get endpoint config
fn get_ac_endpoint_config(endpoint_name: &str) -> Result<(&str, &str), AcError> {
    let config = crate::config::get_config();

    match config.ac_controller_endpoints.get(endpoint_name) {
        Some(props) => {
            debug!(
                "Found AC endpoint '{}' at {}",
                endpoint_name, props.endpoint
            );
            Ok((&props.endpoint, &props.api_key))
        }
        None => {
            error!("AC endpoint '{}' not found in config", endpoint_name);
            Err(AcError::EndpointNotFound(endpoint_name.to_string()))
        }
    }
}

// Helper function to handle API responses
async fn handle_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T, AcError> {
    let api_response: common::ApiResponse<T> = response.json().await?;

    if api_response.success {
        debug!("API request successful");
        Ok(api_response.data)
    } else {
        error!("API request failed: {}", api_response.error);
        Err(AcError::ApiError(api_response.error))
    }
}
