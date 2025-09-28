mod config;
mod types;

use env_logger::Env;
use log::{debug, error};

fn main() {
    // Set up logging
    init_logging();

    // xxx
    config::get_config();
    println!("Hello, world!");
}

fn init_logging() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    debug!("Logging initialized");

    // Set up panic logging
    std::panic::set_hook(Box::new(|panic_info| {
        error!("PANIC: {}", panic_info);
    }));
}
