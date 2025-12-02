use axum::{
    Json, Router,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    ac_controller::{AcDevices, ac_executor},
    db,
    types::{AcAction, ApiError, ApiResponse},
};

pub fn ac_routes() -> Router {
    Router::new()
        .route("/get_history_page", get(get_history_page))
        .route("/get_history_count", get(get_history_count))
        .route("/reset_device_state", post(reset_device_state))
}

#[derive(Deserialize)]
#[serde(default)]
struct HistoryPageRequest {
    page_size: i64,
    page_num: i64,
}

impl Default for HistoryPageRequest {
    fn default() -> Self {
        Self {
            page_size: 10,
            page_num: 1,
        }
    }
}

// GET /api/ac/get_history_page?page_size=10&page_num=1
// Returns Vec<db_types::AcAction>
async fn get_history_page(Query(params): Query<HistoryPageRequest>) -> Response {
    // Validate parameters
    if params.page_size <= 0 || params.page_size > 100 {
        let response = ApiError::error("Invalid page size");
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }
    if params.page_num <= 0 {
        let response = ApiError::error("Invalid page number");
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // Calculate offset
    let offset = (params.page_num - 1) * params.page_size;

    // Query DB and return result
    match db::ac_actions::get_page(params.page_size, offset).await {
        Ok(records) => {
            let response = ApiResponse::success(records);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => {
            log::error!("Database error in get_history_page: {}", err);
            let response = ApiError::error("Database error has occurred");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// GET /api/ac/get_history_count
/// Use to determine page count in frontend
async fn get_history_count() -> Response {
    match db::ac_actions::get_count().await {
        Ok(count) => {
            let response = ApiResponse::success(count);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => {
            log::error!("Database error in get_history_count: {}", err);
            let response = ApiError::error("Database error has occurred");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

#[derive(Deserialize)]
struct ResetDeviceStateRequest {
    device: String,
}

#[derive(Serialize)]
struct ResetDeviceStateResponse {
    success: bool,
    message: String,
}

/// POST /api/ac/reset_device_state
/// Resets the tracked state for a specific AC device
/// This is useful when the tracked state gets out of sync with the physical device
/// After reset, the next control cycle will treat it as first execution and force sync
async fn reset_device_state(Json(req): Json<ResetDeviceStateRequest>) -> Response {
    // Validate device name
    let device = match AcDevices::from_str(&req.device) {
        Some(d) => d,
        None => {
            let response = ApiError::error(&format!("Unknown device: {}", req.device));
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };
    
    // Reset the device state
    ac_executor::reset_device_state(&device);
    
    log::info!("Device state reset via API for device: {}", req.device);
    
    let response = ApiResponse::success(ResetDeviceStateResponse {
        success: true,
        message: format!("Device state reset for '{}'. Next control cycle will force sync with physical device.", req.device),
    });
    
    (StatusCode::OK, Json(response)).into_response()
}
