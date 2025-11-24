pub mod plan_helpers;
mod plan_types;
pub mod pir_state;
pub mod ac_executor;
mod manual_mode_monitor;

// Re-export types needed by other modules
pub use plan_types::{AcDevices, RequestMode, PlanResult};
pub use manual_mode_monitor::get_manual_mode_monitor;

use std::time::Duration;
use tokio;

/// Control cycle interval in seconds (5 minutes)
const CONTROL_CYCLE_INTERVAL_SECS: u64 = 300;

/// Manual mode polling interval in seconds (10 seconds)
const MANUAL_MODE_POLL_INTERVAL_SECS: u64 = 10;

/// Start the AC controller loop
/// Runs immediately on startup, then repeats every 5 minutes
/// Also spawns a separate task to monitor devices in manual mode
pub async fn start_ac_controller() {
    log::info!("AC controller starting...");
    
    // Collect initial device states before starting control cycles
    // This ensures we know Auto/Manual mode and temperature before planning
    collect_initial_device_states().await;
    
    // Start the manual mode monitoring task
    tokio::spawn(async move {
        manual_mode_monitoring_loop().await;
    });
    
    // Main control loop (5 minutes)
    loop {
        // Execute AC control for all devices
        execute_ac_control_cycle().await;
        
        // Wait before next cycle
        tokio::time::sleep(Duration::from_secs(CONTROL_CYCLE_INTERVAL_SECS)).await;
    }
}

/// Collect initial device states (Auto/Manual mode and temperature) before first control cycle
/// This ensures we have device state information before attempting to plan and execute
async fn collect_initial_device_states() {
    log::info!("Collecting initial device states before first control cycle");
    
    let monitor = manual_mode_monitor::get_manual_mode_monitor();
    
    for device in AcDevices::all() {
        let device_name = device.as_str();
        
        // Fetch sensor data to get both temperature and Auto/Manual mode
        match crate::device_requests::ac::get_sensors(device_name).await {
            Ok(sensor_data) => {
                // Store the Auto/Manual mode
                monitor.update_mode(device_name, sensor_data.is_automatic_mode);
                log::info!(
                    "Initial state for {}: {} mode, temperature: {:.1}°C",
                    device_name,
                    if sensor_data.is_automatic_mode { "Auto" } else { "Manual" },
                    sensor_data.temperature
                );
            }
            Err(e) => {
                log::warn!(
                    "Failed to fetch initial state for {}: {}. Device will default to Manual mode for safety.",
                    device_name,
                    e
                );
            }
        }
    }
    
    log::info!("Initial device state collection complete");
}

/// Execute one cycle of AC control for all devices
async fn execute_ac_control_cycle() {
    log::info!("Starting AC control cycle");
    
    // Process each device
    for device in AcDevices::all() {
        let device_name = device.as_str();
        log::debug!("Processing device: {}", device_name);
        
        // Fetch data and calculate plan
        // Note: fetch_data_and_get_plan handles errors internally and returns NoChange on failure
        let plan = plan_types::fetch_data_and_get_plan(&device).await;
        log::info!("Plan for {}: {:?}", device_name, plan);
        
        // Execute the plan
        match ac_executor::execute_plan(&device, &plan, false).await {
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

/// Monitor devices in manual mode and detect transitions to auto mode
/// Polls devices every 10 seconds to quickly respond to mode changes
async fn manual_mode_monitoring_loop() {
    log::info!("Manual mode monitoring loop starting...");
    
    // Wait a bit before starting to allow the initial control cycle to complete
    // Initial device states are already collected in start_ac_controller before this task starts
    tokio::time::sleep(Duration::from_secs(5)).await;
    log::info!("Starting transition monitoring (initial states already collected)");
    
    loop {
        log::debug!("Checking manual mode devices");
        
        // Get the manual mode monitor once before the loop
        let monitor = manual_mode_monitor::get_manual_mode_monitor();
        
        // Check each device
        for device in AcDevices::all() {
            let device_name = device.as_str();
            
            // Fetch current sensor data to check automatic mode status
            match crate::device_requests::ac::get_sensors(device_name).await {
                Ok(sensor_data) => {
                    // Update mode and check for Manual→Auto transition
                    let transitioned_to_auto = monitor.update_mode(device_name, sensor_data.is_automatic_mode);
                    
                    if transitioned_to_auto {
                        log::info!(
                            "Device '{}' transitioned from Manual to Auto mode - triggering immediate plan execution",
                            device_name
                        );
                        
                        // Fetch data and calculate plan
                        let plan = plan_types::fetch_data_and_get_plan(&device).await;
                        log::info!("Manual→Auto transition plan for {}: {:?}", device_name, plan);
                        
                        // Execute the plan with force_execution flag to bypass NoChange optimization
                        // Override the cause to ManualToAutoTransition
                        let transition_plan = plan_types::PlanResult::new(
                            plan.mode, 
                            crate::types::CauseReason::ManualToAutoTransition
                        );
                        
                        match ac_executor::execute_plan(&device, &transition_plan, true).await {
                            Ok(command_sent) => {
                                if command_sent {
                                    log::info!("Manual→Auto transition command sent for {}", device_name);
                                } else {
                                    log::warn!("Manual→Auto transition detected but no command was sent for {}", device_name);
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to execute Manual→Auto transition plan for {}: {}", device_name, e);
                            }
                        }
                    } else if sensor_data.is_automatic_mode {
                        log::debug!("Device '{}' is in Auto mode (no transition)", device_name);
                    } else {
                        log::debug!("Device '{}' is in Manual mode", device_name);
                    }
                }
                Err(e) => {
                    log::debug!("Failed to check mode for {}: {}", device_name, e);
                    // Don't log as error since this is a frequent check
                }
            }
        }
        
        // Wait before next check
        tokio::time::sleep(Duration::from_secs(MANUAL_MODE_POLL_INTERVAL_SECS)).await;
    }
}
