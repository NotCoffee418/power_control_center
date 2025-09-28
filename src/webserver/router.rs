use crate::webserver::api_controllers::handle_api_request;
use log::{debug, info};
use rust_embed::RustEmbed;
use tiny_http::{Header, Method, Response, Server};

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

    // minor security check to prevent directory traversal attacks
    if url.contains("..") {
        let response = Response::from_string("Bad Request").with_status_code(400);
        request.respond(response)?;
        return Ok(());
    }

    // route api requests
    if url.starts_with("/api/") {
        return handle_api_request(request, method, url);
    }

    // serve static files
    match (method, url.as_str()) {
        (Method::Get, path) => {
            serve_static(request, path);
        }

        // Post request to static or other shenanigans.
        _ => {
            let response = Response::from_string("Bad Request").with_status_code(400);
            request.respond(response)?;
        }
    }
    Ok(())
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
