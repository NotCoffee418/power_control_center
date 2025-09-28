use tiny_http::{Header, Response};

use crate::types;

/// Respond with standardized JSON response
/// example usage:
/// respond_json(request, types::ApiResponse::success(data))
pub fn respond_json<T>(request: tiny_http::Request, response_data: types::ApiResponse<T>)
where
    T: serde::Serialize,
{
    let json_str = response_data.to_json();
    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
    let response = Response::from_string(json_str)
        .with_header(header)
        .with_status_code(response_data.http_status);
    match request.respond(response) {
        Err(e) => log::error!("respond_json: Failed to send JSON response: {}", e),
        _ => (),
    }
}
