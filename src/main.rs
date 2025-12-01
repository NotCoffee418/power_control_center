mod ac_controller;
mod config;
mod db;
mod device_requests;
mod nodes;
mod types;
mod webserver;

use env_logger::Env;
use log::{debug, error};
use tokio;

#[tokio::main]
async fn main() {
    // Set up logging
    init_logging();

    // Prepare database
    {
        let pool = db::get_pool().await;
        if let Err(e) = sqlx::migrate!("./migrations").run(pool).await {
            panic!("Failed to run database migrations: {}", e);
        } else {
            debug!("Database migrations OK.");
        }

        // Initialize defaults for empty tables (cause_reasons, nodesets)
        db::defaults::initialize_defaults(pool).await;
    }

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
