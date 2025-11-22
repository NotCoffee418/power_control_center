mod plan_helpers;
mod plan_types;
pub mod pir_state;
pub mod ac_executor;

// Re-export types needed by other modules
pub use plan_types::{AcDevices, RequestMode};

use std::time::Duration;
use tokio;

/// Start the AC controller loop
/// Runs immediately on startup, then repeats every 5 minutes
pub async fn start_ac_controller() {
    log::info!("AC controller starting...");
    
    loop {
        // Execute AC control for all devices
        execute_ac_control_cycle().await;
        
        // Wait 5 minutes before next cycle
        tokio::time::sleep(Duration::from_secs(5 * 60)).await;
    }
}

/// Execute one cycle of AC control for all devices
async fn execute_ac_control_cycle() {
    log::info!("Starting AC control cycle");
    
    // List of all AC devices to control
    let devices = vec![
        AcDevices::LivingRoom,
        AcDevices::Veranda,
    ];
    
    // Process each device
    for device in devices {
        let device_name = device.as_str();
        log::debug!("Processing device: {}", device_name);
        
        // Fetch data and calculate plan
        let plan = plan_types::fetch_data_and_get_plan(&device).await;
        log::info!("Plan for {}: {:?}", device_name, plan);
        
        // Execute the plan
        match ac_executor::execute_plan(&device, &plan).await {
            Ok(command_sent) => {
                if command_sent {
                    log::info!("AC command executed for {}", device_name);
                } else {
                    log::debug!("No command needed for {} (state unchanged)", device_name);
                }
            }
            Err(e) => {
                log::error!("Failed to execute plan for {}: {}", device_name, e);
            }
        }
    }
    
    log::info!("AC control cycle completed");
}
