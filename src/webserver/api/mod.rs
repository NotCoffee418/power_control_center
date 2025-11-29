mod ac;
mod pir;
mod dashboard;
pub mod nodes;
mod simulator;
mod cause_reasons;

use axum::{
    routing::get,
    Router,
};

/// Build the API routes
pub fn api_routes() -> Router {
    Router::new()
        .route("/status", get(status_handler))
        .nest("/ac", ac::ac_routes())
        .nest("/pir", pir::pir_routes())
        .nest("/dashboard", dashboard::dashboard_routes())
        .nest("/nodes", nodes::nodes_routes())
        .nest("/simulator", simulator::simulator_routes())
        .nest("/cause-reasons", cause_reasons::cause_reasons_routes())
}

async fn status_handler() -> axum::Json<crate::types::ApiResponse<&'static str>> {
    axum::Json(crate::types::ApiResponse::success("API is running"))
}
