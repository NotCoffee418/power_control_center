use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};

use crate::{db, types::ApiResponse};

pub fn cause_reasons_routes() -> Router {
    Router::new()
        .route("/", get(list_cause_reasons))
        .route("/all", get(list_all_cause_reasons))
        .route("/", post(create_cause_reason))
        .route("/:id", get(get_cause_reason))
        .route("/:id", put(update_cause_reason))
        .route("/:id", delete(delete_cause_reason))
        .route("/:id/hidden", put(set_hidden_status))
}

/// Request for creating a new cause reason
#[derive(Serialize, Deserialize)]
pub struct CreateCauseReasonRequest {
    pub label: String,
    pub description: String,
}

/// Request for updating a cause reason
#[derive(Serialize, Deserialize)]
pub struct UpdateCauseReasonRequest {
    pub label: String,
    pub description: String,
}

/// Request for setting hidden status
#[derive(Serialize, Deserialize)]
pub struct SetHiddenRequest {
    pub is_hidden: bool,
}

/// GET /api/cause-reasons
/// Returns all visible cause reasons
async fn list_cause_reasons() -> Response {
    match db::cause_reasons::get_all(false).await {
        Ok(reasons) => {
            let response = ApiResponse::success(reasons);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to list cause reasons: {}", e);
            let response = ApiResponse::<()>::error("Failed to list cause reasons");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// GET /api/cause-reasons/all
/// Returns all cause reasons including hidden ones
async fn list_all_cause_reasons() -> Response {
    match db::cause_reasons::get_all(true).await {
        Ok(reasons) => {
            let response = ApiResponse::success(reasons);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to list all cause reasons: {}", e);
            let response = ApiResponse::<()>::error("Failed to list cause reasons");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// GET /api/cause-reasons/:id
/// Returns a specific cause reason
async fn get_cause_reason(Path(id): Path<i32>) -> Response {
    match db::cause_reasons::get_by_id(id).await {
        Ok(Some(reason)) => {
            let response = ApiResponse::success(reason);
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => {
            let response = ApiResponse::<()>::error("Cause reason not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to get cause reason: {}", e);
            let response = ApiResponse::<()>::error("Failed to get cause reason");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// POST /api/cause-reasons
/// Creates a new cause reason
async fn create_cause_reason(Json(request): Json<CreateCauseReasonRequest>) -> Response {
    // Validate input
    if request.label.trim().is_empty() {
        let response = ApiResponse::<()>::error("Label cannot be empty");
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }
    
    if request.description.trim().is_empty() {
        let response = ApiResponse::<()>::error("Description cannot be empty");
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }
    
    match db::cause_reasons::create(&request.label, &request.description).await {
        Ok(reason) => {
            log::info!("Created cause reason with id {}", reason.id);
            let response = ApiResponse::success(reason);
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to create cause reason: {}", e);
            let response = ApiResponse::<()>::error("Failed to create cause reason");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// PUT /api/cause-reasons/:id
/// Updates a cause reason
async fn update_cause_reason(
    Path(id): Path<i32>,
    Json(request): Json<UpdateCauseReasonRequest>,
) -> Response {
    // Validate input
    if request.label.trim().is_empty() {
        let response = ApiResponse::<()>::error("Label cannot be empty");
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }
    
    if request.description.trim().is_empty() {
        let response = ApiResponse::<()>::error("Description cannot be empty");
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }
    
    // Check if the cause reason is editable
    match db::cause_reasons::get_by_id(id).await {
        Ok(Some(reason)) => {
            if !reason.is_editable {
                let response = ApiResponse::<()>::error("This cause reason cannot be modified");
                return (StatusCode::FORBIDDEN, Json(response)).into_response();
            }
        }
        Ok(None) => {
            let response = ApiResponse::<()>::error("Cause reason not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(e) => {
            log::error!("Failed to check cause reason: {}", e);
            let response = ApiResponse::<()>::error("Failed to check cause reason");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    }
    
    match db::cause_reasons::update(id, &request.label, &request.description).await {
        Ok(true) => {
            log::info!("Updated cause reason {}", id);
            // Fetch the updated record
            match db::cause_reasons::get_by_id(id).await {
                Ok(Some(reason)) => {
                    let response = ApiResponse::success(reason);
                    (StatusCode::OK, Json(response)).into_response()
                }
                _ => {
                    let response = ApiResponse::success("Cause reason updated");
                    (StatusCode::OK, Json(response)).into_response()
                }
            }
        }
        Ok(false) => {
            let response = ApiResponse::<()>::error("Cause reason not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to update cause reason: {}", e);
            let response = ApiResponse::<()>::error("Failed to update cause reason");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// DELETE /api/cause-reasons/:id
/// Deletes a cause reason
async fn delete_cause_reason(Path(id): Path<i32>) -> Response {
    // Cannot delete the Undefined reason (ID 0)
    if id == 0 {
        let response = ApiResponse::<()>::error("Cannot delete the Undefined cause reason");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }
    
    // Check if the cause reason is editable
    match db::cause_reasons::get_by_id(id).await {
        Ok(Some(reason)) => {
            if !reason.is_editable {
                let response = ApiResponse::<()>::error("This cause reason cannot be deleted");
                return (StatusCode::FORBIDDEN, Json(response)).into_response();
            }
        }
        Ok(None) => {
            let response = ApiResponse::<()>::error("Cause reason not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(e) => {
            log::error!("Failed to check cause reason: {}", e);
            let response = ApiResponse::<()>::error("Failed to check cause reason");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    }
    
    match db::cause_reasons::delete(id).await {
        Ok(true) => {
            log::info!("Deleted cause reason {}", id);
            let response = ApiResponse::success("Cause reason deleted");
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(false) => {
            let response = ApiResponse::<()>::error("Cause reason not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to delete cause reason: {}", e);
            let response = ApiResponse::<()>::error("Failed to delete cause reason");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// PUT /api/cause-reasons/:id/hidden
/// Sets the hidden status of a cause reason
async fn set_hidden_status(
    Path(id): Path<i32>,
    Json(request): Json<SetHiddenRequest>,
) -> Response {
    // Cannot hide the Undefined reason (ID 0)
    if id == 0 && request.is_hidden {
        let response = ApiResponse::<()>::error("Cannot hide the Undefined cause reason");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }
    
    // Check if the cause reason is editable (non-editable items cannot be hidden)
    match db::cause_reasons::get_by_id(id).await {
        Ok(Some(reason)) => {
            if !reason.is_editable && request.is_hidden {
                let response = ApiResponse::<()>::error("This cause reason cannot be hidden");
                return (StatusCode::FORBIDDEN, Json(response)).into_response();
            }
        }
        Ok(None) => {
            let response = ApiResponse::<()>::error("Cause reason not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(e) => {
            log::error!("Failed to check cause reason: {}", e);
            let response = ApiResponse::<()>::error("Failed to check cause reason");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    }
    
    match db::cause_reasons::set_hidden(id, request.is_hidden).await {
        Ok(true) => {
            log::info!("Set cause reason {} hidden status to {}", id, request.is_hidden);
            // Fetch the updated record
            match db::cause_reasons::get_by_id(id).await {
                Ok(Some(reason)) => {
                    let response = ApiResponse::success(reason);
                    (StatusCode::OK, Json(response)).into_response()
                }
                _ => {
                    let response = ApiResponse::success("Hidden status updated");
                    (StatusCode::OK, Json(response)).into_response()
                }
            }
        }
        Ok(false) => {
            let response = ApiResponse::<()>::error("Cause reason not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to set hidden status: {}", e);
            let response = ApiResponse::<()>::error("Failed to set hidden status");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}
