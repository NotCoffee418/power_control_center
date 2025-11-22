use axum::{
    Json, Router,
    extract::Query,
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::Deserialize;
use log::{info, warn};

use crate::{
    ac_controller::{pir_state, ac_executor, AcDevices, RequestMode, PlanResult},
    types::{ApiError, ApiResponse, CauseReason},
};

pub fn pir_routes() -> Router {
    Router::new()
        .route("/detect", post(pir_detect))
        .route("/alive", post(pir_alive))
}

#[derive(Deserialize)]
struct PirDetectRequest {
    device: String,
}

/// POST /api/pir/detect?device=Veranda
/// Records a PIR detection and immediately turns off the corresponding AC device
async fn pir_detect(
    headers: HeaderMap,
    Query(params): Query<PirDetectRequest>,
) -> Response {
    // Verify API key
    if !verify_api_key(&headers) {
        warn!("Unauthorized PIR detection attempt");
        let response = ApiError::error("Unauthorized");
        return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
    }

    info!("PIR detection received for device: {}", params.device);

    // Record the detection
    let pir_state = pir_state::get_pir_state();
    pir_state.record_detection(&params.device);

    // Convert device name to AcDevices enum
    let device_enum = match AcDevices::from_str(&params.device) {
        Some(d) => d,
        None => {
            warn!("Unknown device name in PIR detection: {}", params.device);
            let response = ApiError::error("Unknown device");
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };

    // Check if device is already off - if so, no need to call executor
    if ac_executor::is_device_off(&device_enum) {
        info!("PIR detection for device {}, AC already off - no action needed", params.device);
        let response = ApiResponse::success("PIR detection recorded, AC was already off");
        return (StatusCode::OK, Json(response)).into_response();
    }

    // Device is on, use executor to turn it off
    let plan = PlanResult::new(RequestMode::Off, CauseReason::PirDetection);
    match ac_executor::execute_plan(&device_enum, &plan).await {
        Ok(_) => {
            info!("AC turned off for device {} due to PIR detection", params.device);
            let response = ApiResponse::success("PIR detection recorded and AC turned off");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            warn!(
                "Failed to turn off AC for device {} after PIR detection: {}",
                params.device, e
            );
            // Still record the detection even if AC control fails
            let response = ApiResponse::success("PIR detection recorded, but AC control failed");
            (StatusCode::OK, Json(response)).into_response()
        }
    }
}

#[derive(Deserialize)]
struct PirAliveRequest {
    #[serde(default)]
    device: String,
}

/// POST /api/pir/alive?device=Veranda
/// Receives a keep-alive signal from PIR devices
async fn pir_alive(
    headers: HeaderMap,
    Query(params): Query<PirAliveRequest>,
) -> Response {
    // Verify API key
    if !verify_api_key(&headers) {
        warn!("Unauthorized PIR alive attempt");
        let response = ApiError::error("Unauthorized");
        return (StatusCode::UNAUTHORIZED, Json(response)).into_response();
    }

    let device_info = if params.device.is_empty() {
        "unknown".to_string()
    } else {
        params.device.clone()
    };

    info!("PIR alive signal received from device: {}", device_info);

    let response = ApiResponse::success("Alive signal acknowledged");
    (StatusCode::OK, Json(response)).into_response()
}

/// Verify the API key from the Authorization header
fn verify_api_key(headers: &HeaderMap) -> bool {
    let config = crate::config::get_config();
    
    // If no API key is configured, allow access (backward compatibility)
    if config.pir_api_key.is_empty() {
        return true;
    }

    // Check for Authorization header
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            // Support both "Bearer <key>" and "ApiKey <key>" formats
            let key = if auth_str.starts_with("Bearer ") {
                &auth_str[7..]
            } else if auth_str.starts_with("ApiKey ") {
                &auth_str[7..]
            } else {
                auth_str
            };

            return key == config.pir_api_key;
        }
    }

    false
}
