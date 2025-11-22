use axum::{
    Json, Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;

use crate::{
    ac_controller::ac_executor::get_state_manager,
    config,
    device_requests,
    types::ApiResponse,
};

pub fn dashboard_routes() -> Router {
    Router::new()
        .route("/status", get(get_dashboard_status))
}

#[derive(Serialize)]
pub struct DashboardStatus {
    pub devices: Vec<DeviceStatus>,
    pub outdoor_temp: Option<f64>,
    pub outdoor_temp_trend: Option<f64>,
    pub solar_production_watts: Option<i32>,
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
}

/// GET /api/dashboard/status
/// Returns current status of all configured devices and environmental data
async fn get_dashboard_status() -> Response {
    let cfg = config::get_config();
    
    // Gather device statuses
    let mut devices = Vec::new();
    
    for device_name in cfg.ac_controller_endpoints.keys() {
        let state = get_state_manager().get_state(device_name);
        
        // Try to get current indoor temperature from the device
        let indoor_temp = match device_requests::ac::get_sensors(device_name).await {
            Ok(sensor_data) => Some(sensor_data.temperature),
            Err(e) => {
                log::warn!("Failed to get sensor data for {}: {}", device_name, e);
                None
            }
        };
        
        let mode_str = state.mode.map(|m| match m {
            1 => "heat".to_string(),
            4 => "cool".to_string(),
            _ => format!("mode_{}", m),
        });
        
        devices.push(DeviceStatus {
            name: device_name.clone(),
            is_on: state.is_on,
            mode: mode_str,
            temperature_setpoint: state.temperature,
            indoor_temperature: indoor_temp,
            fan_speed: state.fan_speed,
            swing: state.swing,
            powerful_mode: state.powerful_mode,
        });
    }
    
    // Get outdoor weather data
    let outdoor_temp = match device_requests::weather::get_current_outdoor_temp(
        cfg.latitude,
        cfg.longitude,
    ).await {
        Ok(temp) => Some(temp),
        Err(e) => {
            log::warn!("Failed to get outdoor temperature: {}", e);
            None
        }
    };
    
    let outdoor_temp_trend = match device_requests::weather::compute_temperature_trend(
        cfg.latitude,
        cfg.longitude,
    ).await {
        Ok(trend) => Some(trend),
        Err(e) => {
            log::warn!("Failed to get temperature trend: {}", e);
            None
        }
    };
    
    // Get solar production data
    let solar_production = match device_requests::meter::get_solar_production().await {
        Ok(production) => Some(production.current_production),
        Err(e) => {
            log::warn!("Failed to get solar production: {}", e);
            None
        }
    };
    
    let status = DashboardStatus {
        devices,
        outdoor_temp,
        outdoor_temp_trend,
        solar_production_watts: solar_production,
    };
    
    let response = ApiResponse::success(status);
    (StatusCode::OK, Json(response)).into_response()
}
