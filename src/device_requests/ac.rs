use super::common;
use super::cache::DataCache;
use log::{debug, error, info, warn};
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
    DatabaseError(String),
}

impl fmt::Display for AcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AcError::ApiError(msg) => write!(f, "API Error: {}", msg),
            AcError::NetworkError(err) => write!(f, "Network Error: {}", err),
            AcError::EndpointNotFound(name) => {
                write!(f, "AC endpoint '{}' not found in config", name)
            }
            AcError::DatabaseError(msg) => write!(f, "Database Error: {}", msg),
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
pub async fn turn_off_ac(endpoint_name: &str, cause_id: i32) -> Result<bool, AcError> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_SECS: u64 = 5;
    
    let (base_url, api_key) = get_ac_endpoint_config(endpoint_name)?;

    info!("Turning off AC '{}'", endpoint_name);
    let url = format!("{}/api/ir/off", base_url);
    
    // Retry loop
    for attempt in 1..=MAX_RETRIES {
        let client = common::get_client().await;
        
        match client
            .post(&url)
            .header("Authorization", format!("ApiKey {}", api_key))
            .send()
            .await
        {
            Ok(response) => {
                match handle_response(response).await {
                    Ok(result) => {
                        // Success - log to database (enqueued if DB unavailable)
                        log_ac_command(endpoint_name, "off", None, None, None, None, cause_id).await;
                        return Ok(result);
                    }
                    Err(e) => {
                        if attempt < MAX_RETRIES {
                            log::warn!("AC turn off failed (attempt {}/{}): {}. Retrying in {}s...", 
                                attempt, MAX_RETRIES, e, RETRY_DELAY_SECS);
                            tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                        } else {
                            log::error!("AC turn off failed after {} attempts: {}", MAX_RETRIES, e);
                            return Err(e);
                        }
                    }
                }
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    log::warn!("AC turn off network error (attempt {}/{}): {}. Retrying in {}s...", 
                        attempt, MAX_RETRIES, e, RETRY_DELAY_SECS);
                    tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                } else {
                    log::error!("AC turn off network error after {} attempts: {}", MAX_RETRIES, e);
                    return Err(AcError::from(e));
                }
            }
        }
    }
    
    unreachable!("Retry loop should have returned within MAX_RETRIES attempts")
}

pub async fn turn_on_ac(
    endpoint_name: &str,
    mode: i32,
    fan_speed: i32,
    temperature: f64,
    swing: i32,
    cause_id: i32,
) -> Result<bool, AcError> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_SECS: u64 = 5;
    
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
    
    // Retry loop
    for attempt in 1..=MAX_RETRIES {
        let client = common::get_client().await;
        
        match client
            .post(&url)
            .header("Authorization", format!("ApiKey {}", api_key))
            .json(&request)
            .send()
            .await
        {
            Ok(response) => {
                match handle_response(response).await {
                    Ok(result) => {
                        // Success - log to database (enqueued if DB unavailable)
                        log_ac_command(
                            endpoint_name,
                            "on",
                            Some(mode),
                            Some(fan_speed),
                            Some(temperature as f32),
                            Some(swing),
                            cause_id,
                        ).await;
                        return Ok(result);
                    }
                    Err(e) => {
                        if attempt < MAX_RETRIES {
                            log::warn!("AC turn on failed (attempt {}/{}): {}. Retrying in {}s...", 
                                attempt, MAX_RETRIES, e, RETRY_DELAY_SECS);
                            tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                        } else {
                            log::error!("AC turn on failed after {} attempts: {}", MAX_RETRIES, e);
                            return Err(e);
                        }
                    }
                }
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    log::warn!("AC turn on network error (attempt {}/{}): {}. Retrying in {}s...", 
                        attempt, MAX_RETRIES, e, RETRY_DELAY_SECS);
                    tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                } else {
                    log::error!("AC turn on network error after {} attempts: {}", MAX_RETRIES, e);
                    return Err(AcError::from(e));
                }
            }
        }
    }
    
    unreachable!("Retry loop should have returned within MAX_RETRIES attempts")
}

pub async fn toggle_powerful(endpoint_name: &str, cause_id: i32) -> Result<bool, AcError> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_SECS: u64 = 5;
    
    let (base_url, api_key) = get_ac_endpoint_config(endpoint_name)?;

    info!("Toggling powerful mode for AC '{}'", endpoint_name);
    let url = format!("{}/api/ir/toggle-powerful", base_url);
    
    // Retry loop
    for attempt in 1..=MAX_RETRIES {
        let client = common::get_client().await;
        
        match client
            .post(&url)
            .header("Authorization", format!("ApiKey {}", api_key))
            .send()
            .await
        {
            Ok(response) => {
                match handle_response(response).await {
                    Ok(result) => {
                        // Success - log to database (enqueued if DB unavailable)
                        log_ac_command(endpoint_name, "toggle-powerful", None, None, None, None, cause_id).await;
                        return Ok(result);
                    }
                    Err(e) => {
                        if attempt < MAX_RETRIES {
                            log::warn!("Toggle powerful failed (attempt {}/{}): {}. Retrying in {}s...", 
                                attempt, MAX_RETRIES, e, RETRY_DELAY_SECS);
                            tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                        } else {
                            log::error!("Toggle powerful failed after {} attempts: {}", MAX_RETRIES, e);
                            return Err(e);
                        }
                    }
                }
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    log::warn!("Toggle powerful network error (attempt {}/{}): {}. Retrying in {}s...", 
                        attempt, MAX_RETRIES, e, RETRY_DELAY_SECS);
                    tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                } else {
                    log::error!("Toggle powerful network error after {} attempts: {}", MAX_RETRIES, e);
                    return Err(AcError::from(e));
                }
            }
        }
    }
    
    unreachable!("Retry loop should have returned within MAX_RETRIES attempts")
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
        match api_response.data {
            Some(data) => Ok(data),
            None => {
                error!("API request succeeded but data field is missing");
                Err(AcError::ApiError("Response data is missing".to_string()))
            }
        }
    } else {
        error!("API request failed: {}", api_response.error);
        Err(AcError::ApiError(api_response.error))
    }
}

/// Log AC command to database with environmental context
/// On failure, enqueues the log entry for retry instead of returning an error
/// This decouples physical device commands from database logging
async fn log_ac_command(
    endpoint_name: &str,
    action_type: &str,
    mode: Option<i32>,
    fan_speed: Option<i32>,
    temperature: Option<f32>,
    swing: Option<i32>,
    cause_id: i32,
) {
    // Try to get indoor temperature from the device
    let measured_temp = match get_sensors(endpoint_name).await {
        Ok(sensor_data) => Some(sensor_data.temperature as f32),
        Err(e) => {
            debug!("Could not fetch sensor data for logging: {}", e);
            None
        }
    };
    
    // Try to get current net power usage
    let net_power = match super::meter::get_latest_reading().await {
        Ok(reading) => {
            // Calculate net power: positive means importing, negative means exporting
            Some(((reading.current_consumption_kw - reading.current_production_kw) * 1000.0) as i32)
        }
        Err(e) => {
            debug!("Could not fetch meter reading for logging: {}", e);
            None
        }
    };
    
    // Try to get current raw solar production
    let solar_production = match super::meter::get_solar_production().await {
        Ok(production) => Some(production.current_production),
        Err(e) => {
            debug!("Could not fetch solar production for logging: {}", e);
            None
        }
    };
    
    // For now, we don't have a reliable way to detect if humans are home
    // This could be enhanced in the future with presence detection
    let is_human_home = None;
    
    let ac_action = crate::types::db_types::AcAction::new_for_insert(
        endpoint_name.to_string(),
        action_type.to_string(),
        mode,
        fan_speed,
        temperature,
        swing,
        measured_temp,
        net_power,
        solar_production,
        is_human_home,
        cause_id,
    );
    
    // Log to database - if it fails, enqueue for retry instead of failing the command
    // Clone before insert so we can reuse the action if database fails
    match crate::db::ac_actions::insert(ac_action.clone()).await {
        Ok(_) => {
            debug!("AC command logged to database successfully");
        }
        Err(e) => {
            warn!(
                "Failed to log AC command to database: {} - enqueuing for retry",
                e
            );
            // Enqueue the action for background retry
            super::logging_queue::get_logging_queue()
                .enqueue(ac_action)
                .await;
        }
    }
}
