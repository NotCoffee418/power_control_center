use axum::{
    Json, Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};

use crate::{
    ac_controller::{self, AcDevices},
    db,
    types::{ApiError, ApiResponse},
};

pub fn user_home_routes() -> Router {
    Router::new()
        .route("/set", post(set_user_home))
        .route("/clear", post(clear_user_home))
}

#[derive(Deserialize)]
pub struct SetUserHomeRequest {
    pub hours: u32,
}

#[derive(Serialize)]
pub struct UserHomeResponse {
    pub success: bool,
    pub message: String,
}

/// POST /api/user-home/set
/// Set user home override for the specified number of hours
async fn set_user_home(Json(request): Json<SetUserHomeRequest>) -> Response {
    // Validate input - reasonable range for hours
    if request.hours == 0 || request.hours > 168 {
        let response = ApiError::error("Hours must be between 1 and 168 (1 week)");
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // Calculate unix timestamp for now + hours
    let now = chrono::Utc::now().timestamp();
    let override_until = now + (request.hours as i64 * 3600);
    
    let pool = db::get_pool().await;
    
    // Update or insert the setting
    let result = sqlx::query(
        "INSERT INTO settings (setting_key, setting_value) VALUES ('user_is_home_override', ?)
         ON CONFLICT(setting_key) DO UPDATE SET setting_value = excluded.setting_value"
    )
    .bind(override_until.to_string())
    .execute(pool)
    .await;
    
    match result {
        Ok(_) => {
            log::info!("User home override set for {} hours (until timestamp: {})", request.hours, override_until);
            
            // Trigger immediate AC evaluation for all devices
            tokio::spawn(async move {
                log::info!("Triggering immediate AC evaluation after user home override set");
                for device in AcDevices::all() {
                    let device_name = device.as_str();
                    match ac_controller::node_executor::execute_nodeset_for_device(&device).await {
                        ac_controller::node_executor::NodeExecutionResult::CommandExecuted => {
                            log::info!("AC command executed for {} after user home override", device_name);
                        }
                        ac_controller::node_executor::NodeExecutionResult::NoAction => {
                            log::debug!("No action needed for {} after user home override", device_name);
                        }
                        ac_controller::node_executor::NodeExecutionResult::ManualMode => {
                            log::debug!("Device {} is in manual mode, skipped evaluation", device_name);
                        }
                        ac_controller::node_executor::NodeExecutionResult::Error(e) => {
                            log::error!("Failed to execute nodeset for {} after user home override: {}", device_name, e);
                        }
                    }
                }
            });
            
            let response = ApiResponse::success(UserHomeResponse {
                success: true,
                message: format!("User home override set for {} hours", request.hours),
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to set user home override: {}", e);
            let response = ApiError::error("Failed to set user home override");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// POST /api/user-home/clear
/// Clear the user home override
async fn clear_user_home() -> Response {
    let pool = db::get_pool().await;
    
    // Set the override to 0
    let result = sqlx::query(
        "UPDATE settings SET setting_value = '0' WHERE setting_key = 'user_is_home_override'"
    )
    .execute(pool)
    .await;
    
    match result {
        Ok(_) => {
            log::info!("User home override cleared");
            
            // Trigger immediate AC evaluation for all devices
            tokio::spawn(async move {
                log::info!("Triggering immediate AC evaluation after user home override cleared");
                for device in AcDevices::all() {
                    let device_name = device.as_str();
                    match ac_controller::node_executor::execute_nodeset_for_device(&device).await {
                        ac_controller::node_executor::NodeExecutionResult::CommandExecuted => {
                            log::info!("AC command executed for {} after user home override cleared", device_name);
                        }
                        ac_controller::node_executor::NodeExecutionResult::NoAction => {
                            log::debug!("No action needed for {} after user home override cleared", device_name);
                        }
                        ac_controller::node_executor::NodeExecutionResult::ManualMode => {
                            log::debug!("Device {} is in manual mode, skipped evaluation", device_name);
                        }
                        ac_controller::node_executor::NodeExecutionResult::Error(e) => {
                            log::error!("Failed to execute nodeset for {} after user home override cleared: {}", device_name, e);
                        }
                    }
                }
            });
            
            let response = ApiResponse::success(UserHomeResponse {
                success: true,
                message: "User home override cleared".to_string(),
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to clear user home override: {}", e);
            let response = ApiError::error("Failed to clear user home override");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}
