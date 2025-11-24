use axum::{
    Router,
    body::Body,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use log::info;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Static;

pub async fn start_webserver() -> Result<(), Box<dyn std::error::Error>> {
    // Get listen address from config
    let cfg = crate::config::get_config();
    let listen_addr = format!("{}:{}", cfg.listen_address, cfg.listen_port);

    info!("Starting web server on {}", listen_addr);

    // Build the axum router
    let app = Router::new()
        .nest("/api", crate::webserver::api::api_routes())
        .fallback(serve_static);

    // Start the server
    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
    info!("Web server running on {}", listen_addr);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn serve_static(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Security check to prevent directory traversal attacks
    if path.contains("..") {
        return (StatusCode::BAD_REQUEST, "Bad Request").into_response();
    }

    // Root = index.html
    let file_path = if path.is_empty() { "index.html" } else { path };

    match Static::get(file_path) {
        Some(content) => {
            let mime = guess_mime(file_path);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(content.data))
                .unwrap()
        }
        None => {
            // For SPA routing: if the path doesn't contain a file extension
            // and the file doesn't exist, serve index.html
            if !path.contains('.') && !path.is_empty() {
                if let Some(index) = Static::get("index.html") {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "text/html")
                        .body(Body::from(index.data))
                        .unwrap();
                }
            }
            (StatusCode::NOT_FOUND, "not found").into_response()
        }
    }
}

fn guess_mime(path: &str) -> &'static str {
    match path.split('.').last() {
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("webmanifest") => "application/manifest+json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        Some("svg") => "image/svg+xml",
        _ => "text/plain",
    }
}
