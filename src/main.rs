mod ac_controller;
mod config;
mod types;
mod webserver;

use env_logger::Env;
use log::{debug, error};
use std::thread;

fn main() {
    // Set up logging
    init_logging();

    // Start AC controller in the background in a seperate thread
    let bg_handle = thread::spawn(|| {
        ac_controller::start_ac_controller();
    });

    // Run webserver on main thread
    webserver::start_webserver();
    bg_handle.join().unwrap();
}

fn init_logging() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    debug!("Logging initialized");

    // Set up panic logging
    std::panic::set_hook(Box::new(|panic_info| {
        error!("PANIC: {}", panic_info);
    }));
}
