mod ac_controller;
mod config;
mod db;
mod types;
mod webserver;

use env_logger::Env;
use log::{debug, error};
use tokio;

#[tokio::main]
async fn main() {
    // Set up logging
    init_logging();

    _ = db::get_pool().await;

    // Start AC controller
    let bg_handle = tokio::spawn(async move {
        ac_controller::start_ac_controller().await;
    });

    // Start webserver
    let webserver_handle = tokio::spawn(async move {
        if let Err(err) = webserver::start_webserver().await {
            panic!("Webserver error: {}", err);
        }
    });

    let _ = tokio::join!(bg_handle, webserver_handle);
}

fn init_logging() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    debug!("Logging initialized");

    // Set up panic logging
    std::panic::set_hook(Box::new(|panic_info| {
        error!("PANIC: {}", panic_info);
    }));
}
