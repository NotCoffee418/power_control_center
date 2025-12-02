//! Node-based executor for AC control
//!
//! This module provides the bridge between the node-based execution engine
//! and the actual AC control commands. It replaces the old plan_types-based
//! execution with the more flexible node-based system.

use std::collections::HashMap;

use crate::{
    ac_controller::{
        ac_executor::{get_state_manager, AC_MODE_COOL, AC_MODE_HEAT},
        manual_mode_monitor, plan_helpers, pir_state, AcDevices,
    },
    config,
    db,
    device_requests,
    nodes::{
        ActiveCommandData, ActionResult, ExecutionInputs, ExecutionResult, NodesetExecutor,
        execution::PIR_NEVER_DETECTED,
    },
    types::CauseReason,
};

use super::ac_executor::AcState;

/// Default outdoor temperature used when weather API is unavailable
const DEFAULT_OUTDOOR_TEMPERATURE: f64 = 20.0;

/// Result of node-based AC control execution
#[derive(Debug)]
pub enum NodeExecutionResult {
    /// Command was executed successfully
    CommandExecuted,
    /// No action was taken (Do Nothing node reached or no change needed)
    NoAction,
    /// Device is in manual mode, skipping execution
    ManualMode,
    /// Nodeset validation or execution failed
    Error(String),
}

/// Execute the active nodeset for a specific device
/// This function:
/// 1. Checks if device is in automatic mode (skips if in manual mode)
/// 2. Gathers all necessary input data for the execution context
/// 3. Loads and executes the active nodeset
/// 4. Converts the execution result to actual AC commands
/// 5. Handles state management and logging
pub async fn execute_nodeset_for_device(device: &AcDevices) -> NodeExecutionResult {
    let device_name = device.as_str();
    log::debug!("Executing nodeset for device: {}", device_name);

    // Check if device is in manual mode
    let monitor = manual_mode_monitor::get_manual_mode_monitor();
    let is_automatic_mode = match monitor.get_mode(device_name) {
        Some(is_auto) => is_auto,
        None => {
            // Mode not yet tracked - fetch from device
            match device_requests::ac::get_sensors_cached(device_name).await {
                Ok(sensor_data) => {
                    monitor.update_mode(device_name, sensor_data.is_automatic_mode);
                    sensor_data.is_automatic_mode
                }
                Err(e) => {
                    log::warn!(
                        "Failed to fetch mode for device '{}': {}. Skipping execution.",
                        device_name,
                        e
                    );
                    return NodeExecutionResult::Error(format!("Failed to fetch device mode: {}", e));
                }
            }
        }
    };

    if !is_automatic_mode {
        log::info!(
            "Device '{}' is in manual mode, skipping automatic command execution",
            device_name
        );
        return NodeExecutionResult::ManualMode;
    }

    // Execute nodeset core logic
    let result = execute_nodeset_core(device).await;
    
    match result {
        Ok(execution_result) => {
            // Convert execution result to AC commands
            execute_result_to_commands(device, execution_result).await
        }
        Err(e) => e,
    }
}

/// Gather all inputs needed for nodeset execution
async fn gather_execution_inputs(device: &AcDevices) -> Result<ExecutionInputs, String> {
    let device_name = device.as_str();
    let config = config::get_config();

    // Get device sensor temperature
    let device_sensor_temperature = match device_requests::ac::get_sensors_cached(device_name).await {
        Ok(sensor_data) => sensor_data.temperature,
        Err(e) => {
            return Err(format!("Failed to get sensor data: {}", e));
        }
    };

    // Get auto mode status (already checked above, but we need it for inputs)
    let is_auto_mode = manual_mode_monitor::get_manual_mode_monitor()
        .get_mode(device_name)
        .unwrap_or(true);

    // Get last change minutes from database
    let last_change_minutes = match db::ac_actions::get_last_action_timestamp(device_name).await {
        Ok(Some(timestamp)) => {
            let now = chrono::Utc::now().timestamp() as i32;
            ((now - timestamp) / 60).max(0) as i64
        }
        Ok(None) => i64::MAX, // No actions ever recorded
        Err(e) => {
            log::warn!("Failed to get last action timestamp: {}", e);
            i64::MAX
        }
    };

    // Get outdoor temperature
    let outdoor_temperature = match device_requests::weather::get_current_outdoor_temp_cached(
        config.latitude,
        config.longitude,
    )
    .await
    {
        Ok(temp) => temp,
        Err(e) => {
            log::warn!("Failed to get outdoor temperature: {}. Using default.", e);
            DEFAULT_OUTDOOR_TEMPERATURE
        }
    };

    // Get is_user_home
    let is_user_home = plan_helpers::is_user_home_and_awake();

    // Get net power and raw solar
    let (net_power_watt, raw_solar_watt) = match device_requests::meter::get_latest_reading_cached().await {
        Ok(reading) => {
            let net = ((reading.current_consumption_kw - reading.current_production_kw) * 1000.0) as i64;
            let solar = match device_requests::meter::get_solar_production_cached().await {
                Ok(production) => production.current_production.max(0) as i64,
                Err(_) => (reading.current_production_kw * 1000.0).max(0.0) as i64,
            };
            (net, solar)
        }
        Err(e) => {
            log::warn!("Failed to get meter reading: {}. Using defaults.", e);
            (0, 0)
        }
    };

    // Get avg_next_24h_outdoor_temp
    let avg_next_24h_outdoor_temp = match device_requests::weather::get_avg_next_24h_outdoor_temp_cached(
        config.latitude,
        config.longitude,
    )
    .await
    {
        Ok(avg) => avg,
        Err(e) => {
            log::warn!("Failed to get 24h average outdoor temperature: {}. Using current.", e);
            outdoor_temperature
        }
    };

    // Get PIR state
    let pir = pir_state::get_pir_state();
    let mut pir_state_map = HashMap::new();
    
    if let Some(last_detection) = pir.get_last_detection(device_name) {
        let now = chrono::Utc::now();
        let minutes_ago = now.signed_duration_since(last_detection).num_minutes();
        let is_triggered = pir.has_recent_detection(device_name, config.pir_timeout_minutes);
        pir_state_map.insert(device_name.to_string(), (is_triggered, minutes_ago));
    } else {
        // No detection ever - use sentinel value indicating never detected
        pir_state_map.insert(device_name.to_string(), (false, PIR_NEVER_DETECTED));
    }

    // Get active command from state manager
    let state_manager = get_state_manager();
    let ac_state = state_manager.get_state(device_name);
    let is_defined = ac_state.is_on || ac_state.mode.is_some();
    
    let active_command = ActiveCommandData {
        is_defined,
        is_on: ac_state.is_on,
        temperature: ac_state.temperature.unwrap_or(0.0),
        mode: ac_state.mode.unwrap_or(0),
        fan_speed: ac_state.fan_speed.unwrap_or(0),
        swing: ac_state.swing.unwrap_or(0),
        is_powerful: ac_state.powerful_mode,
    };

    Ok(ExecutionInputs {
        device: device_name.to_string(),
        device_sensor_temperature,
        is_auto_mode,
        last_change_minutes,
        outdoor_temperature,
        is_user_home,
        net_power_watt,
        raw_solar_watt,
        avg_next_24h_outdoor_temp,
        pir_state: pir_state_map,
        active_command,
    })
}

/// Load the active nodeset from the database
async fn load_active_nodeset() -> Result<(Vec<serde_json::Value>, Vec<serde_json::Value>), String> {
    let pool = db::get_pool().await;

    // Get the active nodeset id
    let active_id = match get_active_nodeset_id(pool).await {
        Ok(id) => id,
        Err(e) => return Err(format!("Failed to get active nodeset id: {}", e)),
    };

    // Fetch the nodeset
    let result = sqlx::query_as::<_, (String,)>("SELECT node_json FROM nodesets WHERE id = ?")
        .bind(active_id)
        .fetch_optional(pool)
        .await;

    match result {
        Ok(Some((node_json,))) => {
            let parsed: serde_json::Value = serde_json::from_str(&node_json)
                .map_err(|e| format!("Failed to parse nodeset JSON: {}", e))?;
            
            let nodes = parsed
                .get("nodes")
                .and_then(|n| n.as_array())
                .map(|arr| arr.clone())
                .unwrap_or_default();
            
            let edges = parsed
                .get("edges")
                .and_then(|e| e.as_array())
                .map(|arr| arr.clone())
                .unwrap_or_default();
            
            Ok((nodes, edges))
        }
        Ok(None) => {
            // No nodeset found - return empty
            Ok((vec![], vec![]))
        }
        Err(e) => Err(format!("Failed to fetch nodeset: {}", e)),
    }
}

/// Get the active nodeset ID from settings
async fn get_active_nodeset_id(pool: &sqlx::SqlitePool) -> Result<i64, sqlx::Error> {
    const DEFAULT_NODESET_ID: i64 = 0;
    
    let result = sqlx::query_as::<_, (String,)>(
        "SELECT setting_value FROM settings WHERE setting_key = 'active_nodeset'"
    )
    .fetch_optional(pool)
    .await?;

    match result {
        Some((value,)) => value.parse::<i64>().map_err(|e| {
            log::error!("Failed to parse active_nodeset value '{}': {}", value, e);
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid active_nodeset value",
            )))
        }),
        None => Ok(DEFAULT_NODESET_ID),
    }
}

/// Handle reset_active_command flag if set in the execution result
/// This resets the device state to undefined (as on startup)
fn handle_reset_active_command_if_needed(device: &AcDevices, result: &ExecutionResult) {
    if result.reset_active_command {
        let device_name = device.as_str();
        log::info!(
            "Reset Active Command triggered for device '{}'. Resetting state to undefined.",
            device_name
        );
        super::ac_executor::reset_device_state(device);
    }
}

/// Convert execution result to actual AC commands
async fn execute_result_to_commands(device: &AcDevices, result: ExecutionResult) -> NodeExecutionResult {
    let device_name = device.as_str();

    // Check for execution errors
    if let Some(error) = result.error {
        log::error!("Nodeset execution error for {}: {}", device_name, error);
        return NodeExecutionResult::Error(error);
    }

    // Handle reset_active_command flag - reset the device state to undefined
    handle_reset_active_command_if_needed(device, &result);

    // Handle different terminal types
    match result.terminal_type.as_deref() {
        Some("Do Nothing") => {
            log::info!(
                "Do Nothing node reached for device '{}'. No action taken.",
                device_name
            );
            NodeExecutionResult::NoAction
        }
        Some("Execute Action") => {
            if let Some(action) = result.action {
                execute_action_result(device, &action).await
            } else {
                log::error!("Execute Action terminal reached but no action data for {}", device_name);
                NodeExecutionResult::Error("Execute Action terminal reached but no action data".to_string())
            }
        }
        _ => {
            log::error!(
                "No valid terminal node reached for device '{}'. Execution incomplete.",
                device_name
            );
            NodeExecutionResult::Error("No valid terminal node reached".to_string())
        }
    }
}

/// Execute an ActionResult by sending the appropriate AC commands
async fn execute_action_result(device: &AcDevices, action: &ActionResult) -> NodeExecutionResult {
    let device_name = device.as_str();
    let state_manager = get_state_manager();
    let current_state = state_manager.get_state(device_name);
    
    // Parse the cause_reason to get the ID for logging
    let cause_id: i32 = match action.cause_reason.parse() {
        Ok(id) => id,
        Err(e) => {
            log::warn!(
                "Failed to parse cause_reason '{}' for device '{}': {}. Using Undefined.",
                action.cause_reason, device_name, e
            );
            CauseReason::Undefined.id()
        }
    };

    // Convert the action to a desired AcState
    let desired_state = action_to_ac_state(action);

    // Check minimum on-time for turn-off operations
    if !desired_state.is_on && current_state.is_on {
        let min_on_time_state = super::min_on_time::get_min_on_time_state();
        if !min_on_time_state.can_turn_off(device_name) {
            log::info!(
                "Device '{}' has not been on for minimum time, not turning off yet",
                device_name
            );
            return NodeExecutionResult::NoAction;
        }
    }

    // Check if state change is needed
    // First execution or state differs requires sending command
    let is_first_execution = !state_manager_is_device_initialized(device_name);
    
    // Log state comparison for debugging
    log::info!(
        "State comparison for '{}': current_on={}, desired_on={}, first_exec={}, requires_change={}",
        device_name,
        current_state.is_on,
        desired_state.is_on,
        is_first_execution,
        current_state.requires_change(&desired_state)
    );
    
    if !is_first_execution && !current_state.requires_change(&desired_state) {
        log::info!(
            "No state change required for device '{}', skipping command (current matches desired)",
            device_name
        );
        return NodeExecutionResult::NoAction;
    }

    if is_first_execution {
        log::info!(
            "First execution for device '{}', sending command to ensure sync",
            device_name
        );
    }

    // Execute the AC command
    let result = send_ac_command(device_name, &current_state, &desired_state, cause_id, is_first_execution).await;

    handle_command_result(device_name, result, &current_state, &desired_state, action, false)
}

/// Convert an ActionResult to an AcState
fn action_to_ac_state(action: &ActionResult) -> AcState {
    // Convert enable_swing boolean to swing integer (0 = off, 1 = on)
    let swing = if action.enable_swing { 1 } else { 0 };
    
    match action.mode.as_str() {
        "Off" => AcState::new_off(),
        "Heat" => {
            let fan_speed = parse_fan_speed(&action.fan_speed);
            AcState::new_on(
                AC_MODE_HEAT,
                fan_speed,
                action.temperature,
                swing,
                action.is_powerful,
            )
        }
        "Cool" => {
            let fan_speed = parse_fan_speed(&action.fan_speed);
            AcState::new_on(
                AC_MODE_COOL,
                fan_speed,
                action.temperature,
                swing,
                action.is_powerful,
            )
        }
        _ => {
            log::warn!("Unknown action mode '{}', defaulting to Off", action.mode);
            AcState::new_off()
        }
    }
}

/// Parse fan speed string to i32
fn parse_fan_speed(fan_speed: &str) -> i32 {
    match fan_speed {
        "Auto" => 0,
        "High" => 1,
        "Medium" => 2,
        "Low" => 3,
        "Quiet" => 4,
        _ => 0, // Default to Auto
    }
}

/// Check if device is initialized in state manager
fn state_manager_is_device_initialized(device_name: &str) -> bool {
    let state_manager = get_state_manager();
    state_manager.is_device_initialized(device_name)
}

/// Update state manager with new state
fn update_state_manager(device_name: &str, state: &AcState) {
    let state_manager = get_state_manager();
    state_manager.set_state(device_name, state.clone());
    state_manager.mark_device_initialized(device_name);
}

/// Send AC command based on state transition
/// 
/// # Arguments
/// * `device_name` - Name of the AC device
/// * `current_state` - Current tracked state of the device
/// * `desired_state` - Desired state from nodeset execution
/// * `cause_id` - ID of the cause reason for logging
/// * `is_first_execution` - Whether this is the first command after startup (forces sync)
async fn send_ac_command(
    device_name: &str,
    current_state: &AcState,
    desired_state: &AcState,
    cause_id: i32,
    is_first_execution: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Case 1: Turning off (from on state)
    if !desired_state.is_on && current_state.is_on {
        log::info!("Turning off AC '{}'", device_name);
        device_requests::ac::turn_off_ac(device_name, cause_id).await?;
        return Ok(());
    }

    // Case 2: AC should be off and is already off
    // Only send off command on first execution to sync with physical device state
    if !desired_state.is_on && !current_state.is_on {
        if is_first_execution {
            log::info!("Sending OFF command to '{}' to ensure sync with physical device", device_name);
            device_requests::ac::turn_off_ac(device_name, cause_id).await?;
        }
        return Ok(());
    }

    // Case 3: Turning on or changing settings
    if desired_state.is_on {
        // Extract AC parameters with proper error handling
        let mode = desired_state.mode.ok_or_else(|| {
            format!("Mode not set when AC is on for device '{}'", device_name)
        })?;
        let fan_speed = desired_state.fan_speed.ok_or_else(|| {
            format!("Fan speed not set when AC is on for device '{}'", device_name)
        })?;
        let temperature = desired_state.temperature.ok_or_else(|| {
            format!("Temperature not set when AC is on for device '{}'", device_name)
        })?;
        let swing = desired_state.swing.ok_or_else(|| {
            format!("Swing not set when AC is on for device '{}'", device_name)
        })?;

        log::info!(
            "Turning on AC '{}': mode={}, fan_speed={}, temp={}°C, swing={}",
            device_name,
            mode,
            fan_speed,
            temperature,
            swing
        );
        device_requests::ac::turn_on_ac(device_name, mode, fan_speed, temperature, swing, cause_id).await?;

        // Handle powerful mode toggle
        if desired_state.powerful_mode != current_state.powerful_mode {
            if desired_state.powerful_mode {
                log::info!("Enabling powerful mode for AC '{}'", device_name);
                device_requests::ac::toggle_powerful(device_name, cause_id).await?;
            } else if current_state.powerful_mode {
                log::info!("Disabling powerful mode for AC '{}'", device_name);
                device_requests::ac::toggle_powerful(device_name, cause_id).await?;
            }
        }
    }

    Ok(())
}

/// Execute nodeset for device when transitioning from manual to auto mode
/// 
/// This function is called when a device transitions from Manual to Auto mode.
/// It bypasses:
/// - Manual mode check (since we know it just transitioned to Auto)
/// - State comparison (to immediately sync AC state with nodeset decision)
///
/// The cause_reason is overridden to ManualToAutoTransition for proper logging.
pub async fn execute_nodeset_for_device_forced(device: &AcDevices) -> NodeExecutionResult {
    let device_name = device.as_str();
    log::info!("Forced nodeset execution for device '{}' (manual to auto transition)", device_name);

    // Execute nodeset core logic (shared with regular execution)
    let result = execute_nodeset_core(device).await;
    
    match result {
        Ok(execution_result) => {
            // For forced execution, use the forced result handler
            execute_result_to_commands_forced(device, execution_result).await
        }
        Err(e) => e,
    }
}

/// Core nodeset execution logic shared between regular and forced execution
/// 
/// Returns the ExecutionResult on success, or a NodeExecutionResult::Error on failure
async fn execute_nodeset_core(device: &AcDevices) -> Result<ExecutionResult, NodeExecutionResult> {
    let device_name = device.as_str();

    // Gather execution inputs
    let inputs = match gather_execution_inputs(device).await {
        Ok(inputs) => inputs,
        Err(e) => {
            log::error!("Failed to gather execution inputs for {}: {}", device_name, e);
            return Err(NodeExecutionResult::Error(format!("Failed to gather inputs: {}", e)));
        }
    };

    // Load the active nodeset
    let (nodes, edges) = match load_active_nodeset().await {
        Ok(data) => data,
        Err(e) => {
            log::error!("Failed to load active nodeset: {}", e);
            return Err(NodeExecutionResult::Error(format!("Failed to load nodeset: {}", e)));
        }
    };

    // Validate the nodeset
    let validation_errors = crate::nodes::validate_nodeset_for_execution(&nodes, &edges);
    if !validation_errors.is_empty() {
        log::error!("Nodeset validation failed: {}", validation_errors.join("; "));
        return Err(NodeExecutionResult::Error(format!("Nodeset validation failed: {}", validation_errors.join("; "))));
    }

    // Create and execute the nodeset
    let mut executor = match NodesetExecutor::new(&nodes, &edges, inputs) {
        Ok(e) => e,
        Err(e) => {
            log::error!("Failed to create executor for {}: {}", device_name, e);
            return Err(NodeExecutionResult::Error(format!("Failed to create executor: {}", e)));
        }
    };

    Ok(executor.execute())
}

/// Convert execution result to AC commands with forced execution
async fn execute_result_to_commands_forced(device: &AcDevices, result: ExecutionResult) -> NodeExecutionResult {
    let device_name = device.as_str();

    // Check for execution errors
    if let Some(error) = result.error {
        log::error!("Nodeset execution error for {}: {}", device_name, error);
        return NodeExecutionResult::Error(error);
    }

    // Handle reset_active_command flag - reset the device state to undefined
    handle_reset_active_command_if_needed(device, &result);

    // Handle different terminal types
    match result.terminal_type.as_deref() {
        Some("Do Nothing") => {
            log::info!(
                "Do Nothing node reached for device '{}'. No action taken.",
                device_name
            );
            NodeExecutionResult::NoAction
        }
        Some("Execute Action") => {
            if let Some(action) = result.action {
                execute_action_result_forced(device, &action).await
            } else {
                log::error!("Execute Action terminal reached but no action data for {}", device_name);
                NodeExecutionResult::Error("Execute Action terminal reached but no action data".to_string())
            }
        }
        _ => {
            log::error!(
                "No valid terminal node reached for device '{}'. Execution incomplete.",
                device_name
            );
            NodeExecutionResult::Error("No valid terminal node reached".to_string())
        }
    }
}

/// Execute an ActionResult with forced execution (bypass state comparison)
/// 
/// This is used when transitioning from Manual to Auto mode. The state comparison
/// is bypassed because we want to immediately sync the AC state with the nodeset
/// decision, regardless of whether the tracked state matches.
async fn execute_action_result_forced(device: &AcDevices, action: &ActionResult) -> NodeExecutionResult {
    let device_name = device.as_str();
    let state_manager = get_state_manager();
    let current_state = state_manager.get_state(device_name);
    
    // Override cause_reason to ManualToAutoTransition for proper logging
    let cause_id = CauseReason::ManualToAutoTransition.id();

    // Convert the action to a desired AcState
    let desired_state = action_to_ac_state(action);

    // Execute the AC command with forced=true to ensure sync
    let result = send_ac_command(device_name, &current_state, &desired_state, cause_id, true).await;

    handle_command_result(device_name, result, &current_state, &desired_state, action, true)
}

/// Handle the result of an AC command execution
/// Shared between regular and forced execution
fn handle_command_result(
    device_name: &str,
    result: Result<(), Box<dyn std::error::Error>>,
    current_state: &AcState,
    desired_state: &AcState,
    action: &ActionResult,
    is_forced: bool,
) -> NodeExecutionResult {
    match result {
        Ok(()) => {
            // Update state manager
            update_state_manager(device_name, desired_state);
            
            // Record turn-on time if applicable
            if desired_state.is_on && !current_state.is_on {
                super::min_on_time::get_min_on_time_state().record_turn_on(device_name);
            }
            
            let forced_str = if is_forced { "forced " } else { "" };
            log::info!(
                "Successfully executed {}AC command for device '{}': {:?}",
                forced_str,
                device_name,
                action
            );
            NodeExecutionResult::CommandExecuted
        }
        Err(e) => {
            let forced_str = if is_forced { "forced " } else { "" };
            log::error!("Failed to execute {}AC command for {}: {}", forced_str, device_name, e);
            NodeExecutionResult::Error(format!("Failed to execute command: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_to_ac_state_off() {
        let action = ActionResult {
            device: "TestDevice".to_string(),
            temperature: 22.0,
            mode: "Off".to_string(),
            fan_speed: "Auto".to_string(),
            is_powerful: false,
            enable_swing: false,
            cause_reason: "0".to_string(),
        };
        
        let state = action_to_ac_state(&action);
        assert!(!state.is_on);
        assert!(state.mode.is_none());
    }

    #[test]
    fn test_action_to_ac_state_heat() {
        let action = ActionResult {
            device: "TestDevice".to_string(),
            temperature: 24.0,
            mode: "Heat".to_string(),
            fan_speed: "Auto".to_string(),
            is_powerful: false,
            enable_swing: false,
            cause_reason: "0".to_string(),
        };
        
        let state = action_to_ac_state(&action);
        assert!(state.is_on);
        assert_eq!(state.mode, Some(AC_MODE_HEAT));
        assert_eq!(state.temperature, Some(24.0));
        assert_eq!(state.fan_speed, Some(0)); // Auto
        assert_eq!(state.swing, Some(0)); // Off because enable_swing is false
        assert!(!state.powerful_mode);
    }

    #[test]
    fn test_action_to_ac_state_cool() {
        let action = ActionResult {
            device: "TestDevice".to_string(),
            temperature: 20.0,
            mode: "Cool".to_string(),
            fan_speed: "High".to_string(),
            is_powerful: true,
            enable_swing: true,
            cause_reason: "0".to_string(),
        };
        
        let state = action_to_ac_state(&action);
        assert!(state.is_on);
        assert_eq!(state.mode, Some(AC_MODE_COOL));
        assert_eq!(state.temperature, Some(20.0));
        assert_eq!(state.fan_speed, Some(1)); // High
        assert_eq!(state.swing, Some(1)); // On because enable_swing is true
        assert!(state.powerful_mode);
    }

    #[test]
    fn test_action_to_ac_state_swing_enabled() {
        // Test that enable_swing=true results in swing=1
        let action = ActionResult {
            device: "TestDevice".to_string(),
            temperature: 22.0,
            mode: "Heat".to_string(),
            fan_speed: "Auto".to_string(),
            is_powerful: false,
            enable_swing: true,
            cause_reason: "0".to_string(),
        };
        
        let state = action_to_ac_state(&action);
        assert_eq!(state.swing, Some(1)); // On because enable_swing is true
    }

    #[test]
    fn test_action_to_ac_state_swing_disabled() {
        // Test that enable_swing=false results in swing=0
        let action = ActionResult {
            device: "TestDevice".to_string(),
            temperature: 22.0,
            mode: "Cool".to_string(),
            fan_speed: "Auto".to_string(),
            is_powerful: false,
            enable_swing: false,
            cause_reason: "0".to_string(),
        };
        
        let state = action_to_ac_state(&action);
        assert_eq!(state.swing, Some(0)); // Off because enable_swing is false
    }

    #[test]
    fn test_parse_fan_speed() {
        assert_eq!(parse_fan_speed("Auto"), 0);
        assert_eq!(parse_fan_speed("High"), 1);
        assert_eq!(parse_fan_speed("Medium"), 2);
        assert_eq!(parse_fan_speed("Low"), 3);
        assert_eq!(parse_fan_speed("Quiet"), 4);
        assert_eq!(parse_fan_speed("Unknown"), 0); // Default to Auto
    }

    #[test]
    fn test_action_to_ac_state_fan_speeds() {
        for (speed_str, expected) in [
            ("Auto", 0),
            ("High", 1),
            ("Medium", 2),
            ("Low", 3),
            ("Quiet", 4),
        ] {
            let action = ActionResult {
                device: "TestDevice".to_string(),
                temperature: 22.0,
                mode: "Cool".to_string(),
                fan_speed: speed_str.to_string(),
                is_powerful: false,
                enable_swing: false,
                cause_reason: "0".to_string(),
            };
            
            let state = action_to_ac_state(&action);
            assert_eq!(state.fan_speed, Some(expected), "Fan speed for {} should be {}", speed_str, expected);
        }
    }

    #[test]
    fn test_node_execution_result_debug() {
        // Verify NodeExecutionResult can be debug-formatted
        let result = NodeExecutionResult::CommandExecuted;
        assert!(format!("{:?}", result).contains("CommandExecuted"));
        
        let result = NodeExecutionResult::NoAction;
        assert!(format!("{:?}", result).contains("NoAction"));
        
        let result = NodeExecutionResult::ManualMode;
        assert!(format!("{:?}", result).contains("ManualMode"));
        
        let result = NodeExecutionResult::Error("test error".to_string());
        assert!(format!("{:?}", result).contains("Error"));
    }
}
