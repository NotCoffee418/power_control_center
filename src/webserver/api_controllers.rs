use crate::types::{ApiError, ApiResponse};
use crate::webserver::helpers::respond_json;
use tiny_http::{Method, Response};

/// Handle requests routed to /api/*
pub fn handle_api_request(
    request: tiny_http::Request,
    method: Method,
    url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Example: /api/status
    match (method, url.as_str()) {
        (Method::Get, "/api/status") => {
            respond_json(request, ApiResponse::success("API is running"));
        }

        // Add more API endpoints here
        _ => {
            respond_json(
                request,
                ApiError::error_with_status("not found or invalid method", 404),
            );
        }
    }
    Ok(())
}
