use axum::{
    Json, Router,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Serialize, Deserialize};

use crate::{
    ac_controller::ac_executor::{get_state_manager, AC_MODE_COOL, AC_MODE_HEAT},
    config,
    db,
    device_requests,
    types::ApiResponse,
};

pub fn dashboard_routes() -> Router {
    Router::new()
        .route("/status", get(get_dashboard_status))
        .route("/recent-commands", get(get_recent_commands))
}

#[derive(Serialize)]
pub struct DashboardStatus {
    pub devices: Vec<DeviceStatus>,
    pub outdoor_temp: Option<f64>,
    pub outdoor_temp_trend: Option<f64>,
    pub solar_production_watts: Option<i32>,
    pub current_consumption_watt: Option<i32>,
    pub current_production_watt: Option<i32>,
    pub net_power_w: Option<i32>,
    pub pir_timeout_minutes: u32,
    pub user_is_home: bool,
}

#[derive(Serialize)]
pub struct DeviceStatus {
    pub name: String,
    pub is_on: bool,
    pub mode: Option<String>,
    pub temperature_setpoint: Option<f64>,
    pub indoor_temperature: Option<f64>,
    pub fan_speed: Option<i32>,
    pub swing: Option<i32>,
    pub powerful_mode: bool,
    pub is_automatic_mode: bool,
    pub last_pir_detection: Option<i64>, // Unix timestamp in seconds
}

const KW_TO_W_MULTIPLIER: f64 = 1000.0;

/// Default mode when sensor data is unavailable - assume manual mode for safety
const DEFAULT_IS_AUTOMATIC_MODE: bool = false;

/// GET /api/dashboard/status
/// Returns current status of all configured devices and environmental data
async fn get_dashboard_status() -> Response {
    let cfg = config::get_config();
    
    // Get PIR state once for all devices (cheap operation, just returns a static reference)
    let pir_state = crate::ac_controller::pir_state::get_pir_state();
    
    // Gather device statuses
    let mut devices = Vec::new();
    
    for device_name in cfg.ac_controller_endpoints.keys() {
        let state = get_state_manager().get_state(device_name);
        
        // Try to get current indoor temperature and automatic mode from the device (using cache)
        let (indoor_temp, is_automatic_mode) = match device_requests::ac::get_sensors_cached(device_name).await {
            Ok(sensor_data) => (Some(sensor_data.temperature), sensor_data.is_automatic_mode),
            Err(e) => {
                log::warn!("Failed to get sensor data for {}: {}", device_name, e);
                (None, DEFAULT_IS_AUTOMATIC_MODE)
            }
        };
        
        // Convert mode integer to string (AC_MODE_COOL=1, AC_MODE_HEAT=4)
        let mode_str = state.mode.map(|m| match m {
            AC_MODE_COOL => "cool".to_string(),
            AC_MODE_HEAT => "heat".to_string(),
            _ => format!("mode_{}", m),
        });
        
        // Get last PIR detection time
        let last_pir_detection = pir_state.get_last_detection(device_name)
            .map(|dt| dt.timestamp());
        
        devices.push(DeviceStatus {
            name: device_name.clone(),
            is_on: state.is_on,
            mode: mode_str,
            temperature_setpoint: state.temperature,
            indoor_temperature: indoor_temp,
            fan_speed: state.fan_speed,
            swing: state.swing,
            powerful_mode: state.powerful_mode,
            is_automatic_mode,
            last_pir_detection,
        });
    }
    
    // Get outdoor weather data (using cache)
    let outdoor_temp = match device_requests::weather::get_current_outdoor_temp_cached(
        cfg.latitude,
        cfg.longitude,
    ).await {
        Ok(temp) => Some(temp),
        Err(e) => {
            log::warn!("Failed to get outdoor temperature: {}", e);
            None
        }
    };
    
    let outdoor_temp_trend = match device_requests::weather::compute_temperature_trend_cached(
        cfg.latitude,
        cfg.longitude,
    ).await {
        Ok(trend) => Some(trend),
        Err(e) => {
            log::warn!("Failed to get temperature trend: {}", e);
            None
        }
    };
    
    // Get solar production data (using cache)
    let solar_production = match device_requests::meter::get_solar_production_cached().await {
        Ok(production) => Some(production.current_production),
        Err(e) => {
            log::warn!("Failed to get solar production: {}", e);
            None
        }
    };
    
    // Get real-time power consumption/production from meter (using cache)
    let (current_consumption, current_production, net_power) = match device_requests::meter::get_latest_reading_cached().await {
        Ok(reading) => {
            // Calculate net power: negative means producing more than consuming
            let net = ((reading.current_consumption_kw - reading.current_production_kw) * KW_TO_W_MULTIPLIER) as i32;
            let consumption_watt = (reading.current_consumption_kw * KW_TO_W_MULTIPLIER) as i32;
            let production_watt = (reading.current_production_kw * KW_TO_W_MULTIPLIER) as i32;
            (
                Some(consumption_watt),
                Some(production_watt),
                Some(net),
            )
        }
        Err(e) => {
            log::warn!("Failed to get meter reading: {}", e);
            (None, None, None)
        }
    };
    
    // Get user home status
    let user_is_home = crate::ac_controller::time_helpers::is_user_home_and_awake();
    
    let status = DashboardStatus {
        devices,
        outdoor_temp,
        outdoor_temp_trend,
        solar_production_watts: solar_production,
        current_consumption_watt: current_consumption,
        current_production_watt: current_production,
        net_power_w: net_power,
        pir_timeout_minutes: cfg.pir_timeout_minutes,
        user_is_home,
    };
    
    let response = ApiResponse::success(status);
    (StatusCode::OK, Json(response)).into_response()
}

#[derive(Deserialize)]
pub struct RecentCommandsQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    10
}

#[derive(Serialize)]
pub struct AcActionWithCause {
    #[serde(flatten)]
    pub action: crate::types::db_types::AcAction,
    pub cause_label: String,
    pub cause_description: String,
}

#[derive(Serialize)]
pub struct RecentCommandsResponse {
    pub commands: Vec<AcActionWithCause>,
    pub total_count: i64,
    pub page: i64,
    pub per_page: i64,
}

/// GET /api/dashboard/recent-commands?page=1&per_page=10
/// Returns recent AC commands with pagination
async fn get_recent_commands(Query(params): Query<RecentCommandsQuery>) -> Response {
    let page = params.page.max(1);
    let per_page = params.per_page.clamp(1, 100);
    let offset = (page - 1) * per_page;
    
    let commands = match db::ac_actions::get_page(per_page, offset).await {
        Ok(cmds) => cmds,
        Err(e) => {
            log::error!("Failed to fetch recent commands: {}", e);
            let response = crate::types::ApiError::error("Failed to fetch commands");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };
    
    // Define default cause reason for unknown IDs
    let default_cause = ("Undefined".to_string(), "No specific reason recorded".to_string());
    
    // Fetch all cause reasons from database in a single query
    let cause_map: std::collections::HashMap<i32, (String, String)> = match db::cause_reasons::get_all(true).await {
        Ok(reasons) => {
            reasons.into_iter()
                .map(|reason| (reason.id, (reason.label, reason.description)))
                .collect()
        }
        Err(e) => {
            log::error!("Failed to fetch cause reasons from database: {}", e);
            std::collections::HashMap::new()
        }
    };
    
    // Enrich commands with cause information from database
    let enriched_commands: Vec<AcActionWithCause> = commands.into_iter().map(|action| {
        let (cause_label, cause_description) = cause_map
            .get(&action.cause_id)
            .unwrap_or(&default_cause)
            .clone();
        AcActionWithCause {
            action,
            cause_label,
            cause_description,
        }
    }).collect();
    
    let total_count = match db::ac_actions::get_count().await {
        Ok(count) => count,
        Err(e) => {
            log::error!("Failed to fetch command count: {}", e);
            0
        }
    };
    
    let response_data = RecentCommandsResponse {
        commands: enriched_commands,
        total_count,
        page,
        per_page,
    };
    
    let response = ApiResponse::success(response_data);
    (StatusCode::OK, Json(response)).into_response()
}
