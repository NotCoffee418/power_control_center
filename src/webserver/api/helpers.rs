use std::collections::HashMap;

use sqlx::error;
use tiny_http::{Header, Response};

use crate::types::{self, ApiError, ApiResponse};

/// Respond with standardized JSON response
/// example usage:
/// respond_json(request, types::ApiResponse::success(data))
pub(super) fn respond_json<T>(
    request: tiny_http::Request,
    response_data: types::ApiResponse<T>,
) -> Result<(), Box<dyn std::error::Error>>
where
    T: serde::Serialize,
{
    let json_str = response_data.to_json();
    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
    let response = Response::from_string(json_str)
        .with_header(header)
        .with_status_code(response_data.http_status);
    match request.respond(response) {
        Err(e) => {
            log::error!("respond_json: Failed to send JSON response: {}", e);
            Ok(()) // this right?
        }
        _ => Ok(()),
    }
}

pub(super) fn respond_invalid_endpoint(
    request: tiny_http::Request,
) -> Result<(), Box<dyn std::error::Error>> {
    respond_json(request, ApiError::error_with_status("Not found", 404));
    Ok(())
}

pub(super) fn respond_invalid_method(
    request: tiny_http::Request,
) -> Result<(), Box<dyn std::error::Error>> {
    respond_json(
        request,
        ApiError::error_with_status("Method not allowed", 405),
    );
    Ok(())
}

pub(super) fn extract_get_params(url: &str) -> HashMap<String, String> {
    println!("Extracting GET params from URL: {}", url);
    url.split('?')
        .nth(1)
        .map(|query| {
            query
                .split('&')
                .filter_map(|param| {
                    param.split_once('=').map(|(k, v)| {
                        (
                            urlencoding::decode(k).unwrap_or_default().into_owned(),
                            urlencoding::decode(v).unwrap_or_default().into_owned(),
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn extract_post_params(mut request: tiny_http::Request) -> HashMap<String, String> {
    let mut body = String::new();
    request.as_reader().read_to_string(&mut body).unwrap();

    // If it's form data (application/x-www-form-urlencoded)
    body.split('&')
        .filter_map(|param| {
            param.split_once('=').map(|(k, v)| {
                (
                    urlencoding::decode(k).unwrap_or_default().into_owned(),
                    urlencoding::decode(v).unwrap_or_default().into_owned(),
                )
            })
        })
        .collect()
}
