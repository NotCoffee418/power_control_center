mod types;

pub use types::{AcState, AC_MODE_COOL, AC_MODE_HEAT};

use super::plan_types::{AcDevices, RequestMode};
use crate::device_requests;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use types::plan_to_state;

/// Global state manager for all AC devices
/// Tracks the last known state of each AC to avoid unnecessary API calls
static AC_STATE_MANAGER: std::sync::OnceLock<AcStateManager> = std::sync::OnceLock::new();

/// Manages state for all AC devices
pub struct AcStateManager {
    states: Arc<RwLock<HashMap<String, AcState>>>,
    /// Tracks whether each device has had its first command sent after startup
    /// This ensures we always send commands on first execution regardless of state
    initialized_devices: Arc<RwLock<HashMap<String, bool>>>,
}

impl AcStateManager {
    fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            initialized_devices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the current state for a device, or return a new "off" state if not tracked yet
    pub fn get_state(&self, device_name: &str) -> AcState {
        let states = self.states.read().unwrap();
        states
            .get(device_name)
            .cloned()
            .unwrap_or_else(AcState::new_off)
    }

    /// Update the state for a device
    fn set_state(&self, device_name: &str, state: AcState) {
        let mut states = self.states.write().unwrap();
        states.insert(device_name.to_string(), state);
    }

    /// Check if a device has been initialized (had its first command sent)
    fn is_device_initialized(&self, device_name: &str) -> bool {
        let initialized = self.initialized_devices.read().unwrap();
        initialized.get(device_name).copied().unwrap_or(false)
    }

    /// Mark a device as initialized after its first command
    fn mark_device_initialized(&self, device_name: &str) {
        let mut initialized = self.initialized_devices.write().unwrap();
        initialized.insert(device_name.to_string(), true);
    }

    /// Clear the initialization flag for a specific device
    fn clear_device_initialization(&self, device_name: &str) {
        let mut initialized = self.initialized_devices.write().unwrap();
        initialized.remove(device_name);
    }

    /// Clear all initialization flags
    fn clear_all_initialization(&self) {
        let mut initialized = self.initialized_devices.write().unwrap();
        initialized.clear();
    }
}

/// Get the global state manager instance
pub fn get_state_manager() -> &'static AcStateManager {
    AC_STATE_MANAGER.get_or_init(AcStateManager::new)
}

/// Execute an AC plan for a specific device
/// This will compare the plan with the current state and only send API calls if needed
///
/// # Arguments
/// * `device` - The AC device to control
/// * `plan` - The desired plan from the planning module
///
/// # Returns
/// * `Ok(true)` if a command was sent successfully
/// * `Ok(false)` if no command was needed (state unchanged)
/// * `Err` if an API call failed
pub async fn execute_plan(
    device: &AcDevices,
    plan: &RequestMode,
) -> Result<bool, Box<dyn std::error::Error>> {
    let device_name = device.as_str();
    let state_manager = get_state_manager();

    // Check if this is the first execution for this device
    let is_first_execution = !state_manager.is_device_initialized(device_name);

    // Get current state
    let current_state = state_manager.get_state(device_name);

    // Convert plan to desired state
    let desired_state = plan_to_state(plan, device_name);

    // Check if we need to make changes
    // On first execution, always send command regardless of state to ensure sync with physical device
    if !is_first_execution && !current_state.requires_change(&desired_state) {
        log::debug!(
            "No state change required for device '{}', skipping API call",
            device_name
        );
        return Ok(false);
    }

    if is_first_execution {
        log::info!(
            "First execution for device '{}', sending command to ensure sync with physical state",
            device_name
        );
    }

    log::info!(
        "State change detected for device '{}', executing plan",
        device_name
    );

    // Execute the necessary API calls to achieve the desired state
    let result = execute_state_change(device_name, &current_state, &desired_state).await;

    // Update state if successful
    if result.is_ok() {
        state_manager.set_state(device_name, desired_state);
        state_manager.mark_device_initialized(device_name);
        log::info!("Successfully updated state for device '{}'", device_name);
    }

    result.map(|_| true)
}

/// Execute the necessary API calls to transition from current state to desired state
async fn execute_state_change(
    device_name: &str,
    current_state: &AcState,
    desired_state: &AcState,
) -> Result<(), Box<dyn std::error::Error>> {
    // Case 1: Turning off the AC
    if desired_state.is_on == false && current_state.is_on {
        log::info!("Turning off AC '{}'", device_name);
        device_requests::ac::turn_off_ac(device_name).await?;
        return Ok(());
    }

    // Case 2: AC should be off and is already off
    if desired_state.is_on == false && !current_state.is_on {
        // Nothing to do
        return Ok(());
    }

    // Case 3: Turning on or changing settings when AC is on
    if desired_state.is_on {
        let mode = desired_state.mode.expect("Mode should be set when AC is on");
        let fan_speed = desired_state
            .fan_speed
            .expect("Fan speed should be set when AC is on");
        let temperature = desired_state
            .temperature
            .expect("Temperature should be set when AC is on");
        let swing = desired_state
            .swing
            .expect("Swing should be set when AC is on");

        // Send the turn on command with all settings
        log::info!(
            "Turning on AC '{}': mode={}, fan_speed={}, temp={}°C, swing={}",
            device_name,
            mode,
            fan_speed,
            temperature,
            swing
        );
        device_requests::ac::turn_on_ac(device_name, mode, fan_speed, temperature, swing).await?;

        // Handle powerful mode toggle if needed
        // Note: We need to check if powerful mode changed and is different from what turn_on sets
        if desired_state.powerful_mode != current_state.powerful_mode {
            if desired_state.powerful_mode {
                log::info!("Enabling powerful mode for AC '{}'", device_name);
                device_requests::ac::toggle_powerful(device_name).await?;
            }
            // If powerful mode should be off but was on, we toggle it off
            else if current_state.powerful_mode {
                log::info!("Disabling powerful mode for AC '{}'", device_name);
                device_requests::ac::toggle_powerful(device_name).await?;
            }
        }
    }

    Ok(())
}

/// Check if a device is currently off according to tracked state
/// Returns true if the device is off or not yet tracked (defaults to off)
pub fn is_device_off(device: &AcDevices) -> bool {
    let device_name = device.as_str();
    let state_manager = get_state_manager();
    let current_state = state_manager.get_state(device_name);
    !current_state.is_on
}

/// Reset the state for a specific device (useful for testing or manual override)
pub fn reset_device_state(device: &AcDevices) {
    let device_name = device.as_str();
    let state_manager = get_state_manager();
    state_manager.set_state(device_name, AcState::new_off());
    state_manager.clear_device_initialization(device_name);
    log::info!("Reset state for device '{}'", device_name);
}

/// Reset all device states (useful for testing or system restart)
pub fn reset_all_states() {
    let state_manager = get_state_manager();
    let mut states = state_manager.states.write().unwrap();
    states.clear();
    drop(states); // Release the lock before calling clear_all_initialization
    state_manager.clear_all_initialization();
    log::info!("Reset all device states");
}

#[cfg(test)]
mod tests {
    use super::super::plan_types::Intensity;
    use super::*;

    #[test]
    fn test_state_manager_get_default() {
        let manager = AcStateManager::new();
        let state = manager.get_state("TestDevice");
        assert_eq!(state, AcState::new_off());
    }

    #[test]
    fn test_state_manager_set_and_get() {
        let manager = AcStateManager::new();
        let test_state = AcState::new_on(4, 0, 22.0, 1, false);

        manager.set_state("TestDevice", test_state.clone());
        let retrieved_state = manager.get_state("TestDevice");

        assert_eq!(retrieved_state, test_state);
    }

    #[test]
    fn test_state_manager_multiple_devices() {
        let manager = AcStateManager::new();
        let state1 = AcState::new_on(4, 0, 22.0, 1, false);
        let state2 = AcState::new_on(1, 0, 24.0, 0, false);

        manager.set_state("Device1", state1.clone());
        manager.set_state("Device2", state2.clone());

        assert_eq!(manager.get_state("Device1"), state1);
        assert_eq!(manager.get_state("Device2"), state2);
    }

    #[test]
    fn test_reset_all_states() {
        let state1 = AcState::new_on(4, 0, 22.0, 1, false);
        
        // Set state through global manager
        let manager = get_state_manager();
        manager.set_state("TestDevice", state1);

        // Verify it's set
        let retrieved = manager.get_state("TestDevice");
        assert!(retrieved.is_on);

        // Reset all
        reset_all_states();

        // Verify it's reset to default (off)
        let after_reset = manager.get_state("TestDevice");
        assert!(!after_reset.is_on);
    }

    #[test]
    fn test_plan_conversion_integration() {
        // Test that plans are correctly converted to states
        let plan_off = RequestMode::NoChange;
        let plan_cool = RequestMode::Colder(Intensity::Medium);
        let plan_heat = RequestMode::Warmer(Intensity::High);

        let state_off = plan_to_state(&plan_off, "LivingRoom");
        let state_cool = plan_to_state(&plan_cool, "LivingRoom");
        let state_heat = plan_to_state(&plan_heat, "LivingRoom");

        assert!(!state_off.is_on);
        assert!(state_cool.is_on);
        assert_eq!(state_cool.mode, Some(4)); // Cool
        assert!(state_heat.is_on);
        assert_eq!(state_heat.mode, Some(1)); // Heat
        assert!(state_heat.powerful_mode); // High intensity should enable powerful
    }

    #[test]
    fn test_state_change_detection_comprehensive() {
        // Test various state change scenarios
        let off_state = AcState::new_off();
        let cool_low = AcState::new_on(4, 0, 26.0, 1, false);
        let cool_high = AcState::new_on(4, 5, 20.0, 1, true);
        let heat_med = AcState::new_on(1, 0, 22.0, 0, false);

        // Off to Cool should require change
        assert!(off_state.requires_change(&cool_low));

        // Cool low to Cool high should require change
        assert!(cool_low.requires_change(&cool_high));

        // Cool to Heat should require change
        assert!(cool_low.requires_change(&heat_med));

        // Same state should not require change
        let cool_low_copy = AcState::new_on(4, 0, 26.0, 1, false);
        assert!(!cool_low.requires_change(&cool_low_copy));
    }

    #[test]
    fn test_is_device_off() {
        // Reset all states first to ensure clean test
        reset_all_states();
        
        // Device not yet tracked should default to off
        assert!(is_device_off(&AcDevices::LivingRoom));
        
        // Set device to on
        let manager = get_state_manager();
        let on_state = AcState::new_on(4, 0, 22.0, 1, false);
        manager.set_state("LivingRoom", on_state);
        
        // Should now be on (not off)
        assert!(!is_device_off(&AcDevices::LivingRoom));
        
        // Set device to off
        let off_state = AcState::new_off();
        manager.set_state("LivingRoom", off_state);
        
        // Should now be off
        assert!(is_device_off(&AcDevices::LivingRoom));
    }

    #[test]
    fn test_device_initialization_tracking() {
        // Create a new manager for testing
        let manager = AcStateManager::new();
        
        // Device should not be initialized initially
        assert!(!manager.is_device_initialized("TestDevice"));
        
        // Mark device as initialized
        manager.mark_device_initialized("TestDevice");
        
        // Device should now be initialized
        assert!(manager.is_device_initialized("TestDevice"));
    }

    #[test]
    fn test_reset_device_state_clears_initialization() {
        // Reset all first
        reset_all_states();
        
        let manager = get_state_manager();
        
        // Set state and mark as initialized
        manager.set_state("LivingRoom", AcState::new_on(4, 0, 22.0, 1, false));
        manager.mark_device_initialized("LivingRoom");
        
        // Verify it's initialized
        assert!(manager.is_device_initialized("LivingRoom"));
        
        // Reset device state
        reset_device_state(&AcDevices::LivingRoom);
        
        // Device should no longer be initialized
        assert!(!manager.is_device_initialized("LivingRoom"));
        
        // And state should be off
        let state = manager.get_state("LivingRoom");
        assert!(!state.is_on);
    }

    #[test]
    fn test_reset_all_states_clears_initialization() {
        let manager = get_state_manager();
        
        // Set multiple devices
        manager.set_state("Device1", AcState::new_on(4, 0, 22.0, 1, false));
        manager.mark_device_initialized("Device1");
        manager.set_state("Device2", AcState::new_on(1, 0, 24.0, 0, false));
        manager.mark_device_initialized("Device2");
        
        // Verify they're initialized
        assert!(manager.is_device_initialized("Device1"));
        assert!(manager.is_device_initialized("Device2"));
        
        // Reset all
        reset_all_states();
        
        // Neither device should be initialized
        assert!(!manager.is_device_initialized("Device1"));
        assert!(!manager.is_device_initialized("Device2"));
    }
}
