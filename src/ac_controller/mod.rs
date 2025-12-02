pub mod devices;
pub mod pir_state;
pub mod ac_executor;
mod manual_mode_monitor;
pub mod min_on_time;
mod node_executor;
pub mod time_helpers;

// Re-export types needed by other modules
pub use devices::AcDevices;

use std::time::Duration;
use tokio;

/// Manual mode polling interval in seconds (10 seconds)
const MANUAL_MODE_POLL_INTERVAL_SECS: u64 = 10;

/// Start the AC controller loop
/// Runs immediately on startup, then repeats at the interval specified in the active profile
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
    
    // Get the initial interval from the active profile
    let mut current_interval_minutes = crate::db::nodesets::get_evaluate_every_minutes().await;
    log::info!(
        "AC controller using evaluate_every_minutes={} from active profile",
        current_interval_minutes
    );
    
    // Main control loop with dynamic interval
    loop {
        // Execute AC control for all devices
        execute_ac_control_cycle().await;
        
        // Check if the interval has changed in the active profile
        let new_interval_minutes = crate::db::nodesets::get_evaluate_every_minutes().await;
        if new_interval_minutes != current_interval_minutes {
            log::info!(
                "Evaluation interval changed from {} to {} minutes",
                current_interval_minutes,
                new_interval_minutes
            );
            current_interval_minutes = new_interval_minutes;
        }
        
        // Wait before next cycle using the current interval
        let interval_secs = (current_interval_minutes as u64) * 60;
        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
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

/// Execute one cycle of AC control for all devices using node-based execution
async fn execute_ac_control_cycle() {
    log::info!("Starting AC control cycle (node-based)");
    
    // Process each device
    for device in AcDevices::all() {
        let device_name = device.as_str();
        log::debug!("Processing device: {}", device_name);
        
        // Execute the active nodeset for this device
        match node_executor::execute_nodeset_for_device(&device).await {
            node_executor::NodeExecutionResult::CommandExecuted => {
                log::info!("AC command executed for {}", device_name);
            }
            node_executor::NodeExecutionResult::NoAction => {
                log::debug!("No action needed for {} (state unchanged or Do Nothing)", device_name);
            }
            node_executor::NodeExecutionResult::ManualMode => {
                log::debug!("Device {} is in manual mode, skipped", device_name);
            }
            node_executor::NodeExecutionResult::Error(e) => {
                log::error!("Failed to execute nodeset for {}: {}", device_name, e);
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
                            "Device '{}' transitioned from Manual to Auto mode - resetting state and triggering immediate nodeset execution",
                            device_name
                        );
                        
                        // Reset the device state to force re-sync with physical device
                        // This ensures we don't rely on potentially stale state from before the manual intervention
                        ac_executor::reset_device_state(&device);
                        
                        // Execute nodeset with forced execution to bypass state comparison
                        match node_executor::execute_nodeset_for_device_forced(&device).await {
                            node_executor::NodeExecutionResult::CommandExecuted => {
                                log::info!("Manual→Auto transition command sent for {}", device_name);
                            }
                            node_executor::NodeExecutionResult::NoAction => {
                                log::warn!("Manual→Auto transition detected but no action taken for {}", device_name);
                            }
                            node_executor::NodeExecutionResult::ManualMode => {
                                log::warn!("Manual→Auto transition detected but device {} is in manual mode", device_name);
                            }
                            node_executor::NodeExecutionResult::Error(e) => {
                                log::error!("Failed to execute Manual→Auto transition nodeset for {}: {}", device_name, e);
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
