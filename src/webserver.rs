use crate::types::{self, ApiResponse};
use log::{debug, info};
use rust_embed::RustEmbed;
use tiny_http::{Header, Method, Request, Response, Server};

#[derive(RustEmbed)]
#[folder = "web/"]
struct Static;

pub fn start_webserver() -> Result<(), Box<dyn std::error::Error>> {
    // Get listen address from config
    let cfg = crate::config::get_config();
    let listen_addr = format!("{}:{}", cfg.listen_address, cfg.listen_port);

    // Attempt to start server
    debug!("Starting web server on {}", listen_addr);
    let server = Server::http(listen_addr.as_str())
        .map_err(|e| format!("Failed to start webserver: {}", e))?;
    info!("Web server running on {}", listen_addr);

    // Pass incoming requests to handler
    for request in server.incoming_requests() {
        handle_request(request)?;
    }

    Ok(())
}

fn handle_request(request: tiny_http::Request) -> Result<(), Box<dyn std::error::Error>> {
    let url = request.url().to_string();
    let method = request.method().clone();
    debug!("{}: {}", method.as_str(), url);

    match (method, url.as_str()) {
        (Method::Get, "/status") => {
            respond_json(request, ApiResponse::success("Service is running"));
        }

        (Method::Get, path) => {
            serve_static(request, path);
        }

        _ => {
            let response = Response::from_string("Not Found").with_status_code(404);
            request.respond(response)?;
        }
    }
    Ok(())
}

/// Respond with standardized JSON response
/// example usage:
/// respond_json(request, types::ApiResponse::success(data))
pub fn respond_json<T>(request: tiny_http::Request, response_data: types::ApiResponse<T>)
where
    T: serde::Serialize,
{
    let json_str = response_data.to_json();
    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
    let response = Response::from_string(json_str).with_header(header);
    match request.respond(response) {
        Err(e) => log::error!("respond_json: Failed to send JSON response: {}", e),
        _ => (),
    }
}

fn serve_static(request: tiny_http::Request, path: &str) {
    // Root = index.html
    let file_path = if path == "/" {
        "index.html"
    } else {
        &path[1..] // Remove leading /
    };

    match Static::get(file_path) {
        Some(content) => {
            let mime = guess_mime(file_path);
            let header = Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()).unwrap();
            let response = Response::from_data(content.data.as_ref()).with_header(header);
            let _ = request.respond(response);
        }
        None => {
            let response = Response::from_string("not found").with_status_code(404);
            let _ = request.respond(response);
        }
    }
}

fn guess_mime(path: &str) -> &'static str {
    match path.split('.').last() {
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        _ => "text/plain",
    }
}
