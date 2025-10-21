use std::{error, path};

use tiny_http::Method;

use crate::{
    db,
    types::{ApiError, ApiResponse},
    webserver::api::{
        helpers::extract_get_params, respond_invalid_endpoint, respond_invalid_method, respond_json,
    },
};

pub async fn handle_ac_routes(
    request: tiny_http::Request,
    method: Method,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // base path: /api/ac
    let path = path.strip_prefix("/ac").unwrap_or(&path);

    match path {
        "/get_history_page" => match method {
            Method::Get => get_history_page(request).await,
            _ => respond_invalid_method(request),
        },
        "/get_history_count" => match method {
            Method::Get => get_history_count(request).await,
            _ => respond_invalid_method(request),
        },

        // Unknown `/api/*` routes
        _ => respond_invalid_endpoint(request),
    }
}

// GET /api/ac/get_history_page?page_size=10&page_num=1
// Returns Vec<db_types::AcAction>
async fn get_history_page(request: tiny_http::Request) -> Result<(), Box<dyn std::error::Error>> {
    // Extract parameters
    let params = extract_get_params(request.url());
    let page_size: i64 = match params.get("page_size").unwrap_or(&"10".to_string()).parse() {
        Ok(size) if size > 0 && size <= 100 => size,
        _ => {
            respond_json(
                request,
                ApiError::error_with_status("Invalid page size", 400),
            )?;
            return Ok(());
        }
    };
    // Parse page number
    let page_num: i64 = match params.get("page_num").unwrap_or(&"1".to_string()).parse() {
        Ok(num) if num > 0 => num,
        _ => {
            respond_json(
                request,
                ApiError::error_with_status("Invalid page number", 400),
            )?;
            return Ok(());
        }
    };

    // Calculate offset
    let offset = (page_num - 1) * page_size;

    // Query DB and validate
    match db::ac_actions::get_page(page_size, offset).await {
        Ok(records) => {
            respond_json(request, ApiResponse::success(records))?;
            return Ok(());
        }
        Err(err) => {
            respond_json(
                request,
                ApiError::error_with_status(&format!("Database error has occurred"), 500),
            )?;
            return Err(err.into());
        }
    }
}

/// GET /api/ac/get_history_count
/// Use to determine page count in frontend
async fn get_history_count(request: tiny_http::Request) -> Result<(), Box<dyn std::error::Error>> {
    // get_count().await? will return the i64 or propagate an error
    let count = db::ac_actions::get_count().await?;
    respond_json(request, ApiResponse::success(count))?;
    Ok(())
}
