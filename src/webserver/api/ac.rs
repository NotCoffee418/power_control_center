use axum::{
    Json, Router,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;

use crate::{
    db,
    types::{ApiError, ApiResponse},
};

pub fn ac_routes() -> Router {
    Router::new()
        .route("/get_history_page", get(get_history_page))
        .route("/get_history_count", get(get_history_count))
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
