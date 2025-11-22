use super::common;
use super::cache::DataCache;
use log::{debug, error, info};
use serde::Deserialize;
use std::sync::OnceLock;

// Public data types
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct RawMeterReading {
    pub timestamp: String,

    // Current consumption/production
    pub current_consumption_kw: f64,
    pub current_production_kw: f64,
    pub l1_consumption_kw: f64,
    pub l2_consumption_kw: f64,
    pub l3_consumption_kw: f64,
    pub l1_production_kw: f64,
    pub l2_production_kw: f64,
    pub l3_production_kw: f64,

    // Totals
    pub total_consumption_day_kwh: f64,
    pub total_consumption_night_kwh: f64,
    pub total_production_day_kwh: f64,
    pub total_production_night_kwh: f64,

    // Electrical info
    pub current_tariff: i32, // 1 = Day, 2 = Night
    pub l1_voltage_v: f64,
    pub l2_voltage_v: f64,
    pub l3_voltage_v: f64,
    pub l1_current_a: f64,
    pub l2_current_a: f64,
    pub l3_current_a: f64,

    // Switches/status
    pub switch_electricity: i32,
    pub switch_gas: i32,

    // Serial numbers
    pub meter_serial_electricity: String,
    pub meter_serial_gas: String,

    // Gas
    pub gas_consumption_m3: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SolarProduction {
    #[serde(rename = "currentProduction")]
    pub current_production: i32,
}

// Custom error type
#[derive(Debug)]
pub enum SmartMeterError {
    ApiError(String),
    NetworkError(reqwest::Error),
    NoReadingsAvailable,
}

impl From<reqwest::Error> for SmartMeterError {
    fn from(err: reqwest::Error) -> Self {
        SmartMeterError::NetworkError(err)
    }
}

impl std::fmt::Display for SmartMeterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmartMeterError::ApiError(msg) => write!(f, "API error: {}", msg),
            SmartMeterError::NetworkError(err) => write!(f, "Network error: {}", err),
            SmartMeterError::NoReadingsAvailable => write!(f, "No readings available yet"),
        }
    }
}

impl std::error::Error for SmartMeterError {}

pub async fn get_latest_reading() -> Result<RawMeterReading, SmartMeterError> {
    let base_url = get_smart_meter_base_url();
    let url = format!("{}/latest", base_url);

    info!("Fetching latest smart meter reading");
    let client = common::get_client().await;

    let response = client.get(&url).send().await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        error!("No smart meter readings available yet");
        return Err(SmartMeterError::NoReadingsAvailable);
    }

    if !response.status().is_success() {
        error!(
            "Smart meter API returned error status: {}",
            response.status()
        );
        return Err(SmartMeterError::ApiError(format!(
            "HTTP {}",
            response.status()
        )));
    }

    let reading: RawMeterReading = response.json().await?;
    debug!("Successfully fetched smart meter reading");
    Ok(reading)
}

pub async fn get_solar_production() -> Result<SolarProduction, SmartMeterError> {
    let base_url = get_smart_meter_base_url();
    let url = format!("{}/solar", base_url);

    info!("Fetching current solar production");
    let client = common::get_client().await;

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        error!("Solar API returned error status: {}", response.status());

        // Try to extract error message from response body
        if let Ok(error_body) = response.json::<serde_json::Value>().await {
            if let Some(error_msg) = error_body.get("error").and_then(|e| e.as_str()) {
                return Err(SmartMeterError::ApiError(error_msg.to_string()));
            }
        }

        return Err(SmartMeterError::ApiError("Unknown error".to_string()));
    }

    let production: SolarProduction = response.json().await?;
    debug!(
        "Successfully fetched solar production: {} W",
        production.current_production
    );
    Ok(production)
}

// Helper to get base URL
fn get_smart_meter_base_url() -> String {
    let config = crate::config::get_config();
    config.smart_meter_api_endpoint.clone()
}

// Cache for meter data (10 second TTL - real-time data)
static METER_READING_CACHE: OnceLock<DataCache<RawMeterReading>> = OnceLock::new();
static SOLAR_PRODUCTION_CACHE: OnceLock<DataCache<SolarProduction>> = OnceLock::new();

fn get_meter_reading_cache() -> &'static DataCache<RawMeterReading> {
    METER_READING_CACHE.get_or_init(|| DataCache::new(10)) // 10 seconds
}

fn get_solar_production_cache() -> &'static DataCache<SolarProduction> {
    SOLAR_PRODUCTION_CACHE.get_or_init(|| DataCache::new(10)) // 10 seconds
}

/// Get latest meter reading with caching (10 second TTL)
/// Recommended for dashboard use to reduce API calls
pub async fn get_latest_reading_cached() -> Result<RawMeterReading, SmartMeterError> {
    let cache = get_meter_reading_cache();
    
    cache.get_or_fetch("latest", || async {
        get_latest_reading().await
    }).await
}

/// Get solar production with caching (10 second TTL)
/// Recommended for dashboard use to reduce API calls
pub async fn get_solar_production_cached() -> Result<SolarProduction, SmartMeterError> {
    let cache = get_solar_production_cache();
    
    cache.get_or_fetch("solar", || async {
        get_solar_production().await
    }).await
}
