mod ac;
mod helpers;

use axum::{
    routing::get,
    Router,
};

/// Build the API routes
pub fn api_routes() -> Router {
    Router::new()
        .route("/status", get(status_handler))
        .nest("/ac", ac::ac_routes())
}

async fn status_handler() -> axum::Json<crate::types::ApiResponse<&'static str>> {
    axum::Json(crate::types::ApiResponse::success("API is running"))
}
