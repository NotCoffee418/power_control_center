mod ac;
mod helpers;

use crate::{
    types::{ApiError, ApiResponse},
    webserver::api::helpers::{respond_invalid_endpoint, respond_invalid_method},
};
use helpers::respond_json;
use tiny_http::Method;

/// Handle requests routed to /api/*
pub async fn handle_api_request(
    request: tiny_http::Request,
    method: Method,
    url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Strip the /api prefix and delegate to sub-handlers
    let path = url.strip_prefix("/api").unwrap_or(&url);

    match path {
        // /api/status
        "/status" => match method {
            Method::Get => respond_json(request, ApiResponse::success("API is running")),
            _ => respond_invalid_method(request),
        },

        // /api/ac
        path if path.starts_with("/ac") => ac::handle_ac_routes(request, method, path).await,

        // Unknown `/api/*` routes
        _ => respond_invalid_endpoint(request),
    }
}
