use axum::{
    Json, Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Serialize, Deserialize};

use crate::{
    db,
    types::ApiResponse,
};

pub fn nodes_routes() -> Router {
    Router::new()
        .route("/configuration", get(get_node_configuration))
        .route("/configuration", post(save_node_configuration))
        .route("/definitions", get(get_node_definitions))
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct NodeConfiguration {
    pub nodes: Vec<serde_json::Value>,
    pub edges: Vec<serde_json::Value>,
}

/// GET /api/nodes/configuration
/// Returns the current node configuration from the database
async fn get_node_configuration() -> Response {
    let pool = db::get_pool().await;
    
    // Fetch the node configuration from the settings table
    let result = sqlx::query_as::<_, (String,)>(
        "SELECT setting_value FROM settings WHERE setting_key = 'node_configuration'"
    )
    .fetch_optional(pool)
    .await;
    
    match result {
        Ok(Some(record)) => {
            // Parse the JSON string to NodeConfiguration
            match serde_json::from_str::<NodeConfiguration>(&record.0) {
                Ok(config) => {
                    let response = ApiResponse::success(config);
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => {
                    log::error!("Failed to parse node configuration: {}", e);
                    let response = ApiResponse::<()>::error("Failed to parse node configuration");
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
                }
            }
        }
        Ok(None) => {
            // Return default empty configuration if not found
            let response = ApiResponse::success(NodeConfiguration::default());
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to fetch node configuration: {}", e);
            let response = ApiResponse::<()>::error("Failed to fetch node configuration");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// POST /api/nodes/configuration
/// Saves the node configuration to the database
async fn save_node_configuration(
    Json(config): Json<NodeConfiguration>,
) -> Response {
    let pool = db::get_pool().await;
    
    // Serialize the configuration to JSON
    let json_str = match serde_json::to_string(&config) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to serialize node configuration: {}", e);
            let response = ApiResponse::<()>::error("Failed to serialize node configuration");
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };
    
    // Update or insert the configuration
    let result = sqlx::query(
        "INSERT INTO settings (setting_key, setting_value) VALUES (?, ?)
         ON CONFLICT(setting_key) DO UPDATE SET setting_value = excluded.setting_value"
    )
    .bind("node_configuration")
    .bind(&json_str)
    .execute(pool)
    .await;
    
    match result {
        Ok(_) => {
            log::info!("Node configuration saved successfully");
            let response = ApiResponse::success(config);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to save node configuration: {}", e);
            let response = ApiResponse::<()>::error("Failed to save node configuration");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// GET /api/nodes/definitions
/// Returns all available node type definitions
async fn get_node_definitions() -> Response {
    let definitions = crate::nodes::get_all_node_definitions();
    let response = ApiResponse::success(definitions);
    (StatusCode::OK, Json(response)).into_response()
}
