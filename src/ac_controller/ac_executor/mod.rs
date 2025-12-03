mod types;

pub use types::{AcState, AC_MODE_OFF, AC_MODE_COOL, AC_MODE_HEAT};

use super::devices::AcDevices;
use crate::device_requests;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

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
    pub fn set_state(&self, device_name: &str, state: AcState) {
        let mut states = self.states.write().unwrap();
        states.insert(device_name.to_string(), state);
    }

    /// Check if a device has been initialized (had its first command sent)
    pub fn is_device_initialized(&self, device_name: &str) -> bool {
        let initialized = self.initialized_devices.read().unwrap();
        initialized.get(device_name).copied().unwrap_or(false)
    }

    /// Mark a device as initialized after its first command
    pub fn mark_device_initialized(&self, device_name: &str) {
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
    {
        let mut states = state_manager.states.write().unwrap();
        states.clear();
    }
    state_manager.clear_all_initialization();
    log::info!("Reset all device states");
}

/// Turn off a device directly with a specific cause
/// This is a simplified function for cases like PIR detection where we just need to turn off the AC
/// without going through the full planning system
///
/// # Arguments
/// * `device` - The AC device to turn off
/// * `cause` - The reason for turning off the device
///
/// # Returns
/// * `Ok(true)` if the command was sent successfully
/// * `Ok(false)` if the device is already off (no command needed)
/// * `Err` if the API call failed
pub async fn turn_off_device(
    device: &AcDevices,
    cause: crate::types::CauseReason,
) -> Result<bool, Box<dyn std::error::Error>> {
    let device_name = device.as_str();
    let state_manager = get_state_manager();
    
    // Check if device is already off
    let current_state = state_manager.get_state(device_name);
    if !current_state.is_on {
        log::debug!("Device '{}' is already off, no action needed", device_name);
        return Ok(false);
    }
    
    // Turn off the device
    log::info!("Turning off AC '{}' due to {:?}", device_name, cause);
    device_requests::ac::turn_off_ac(device_name, cause.id()).await?;
    
    // Update the tracked state
    state_manager.set_state(device_name, AcState::new_off());
    
    Ok(true)
}

#[cfg(test)]
mod tests {
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
    fn test_state_change_detection_comprehensive() {
        // Test various state change scenarios
        let off_state = AcState::new_off();
        let cool_low = AcState::new_on(1, 0, 26.0, 1, false);
        let cool_high = AcState::new_on(1, 1, 20.0, 1, true);
        let heat_med = AcState::new_on(4, 0, 22.0, 0, false);

        // Off to Cool should require change
        assert!(off_state.requires_change(&cool_low));

        // Cool low to Cool high should require change
        assert!(cool_low.requires_change(&cool_high));

        // Cool to Heat should require change
        assert!(cool_low.requires_change(&heat_med));

        // Same state should not require change
        let cool_low_copy = AcState::new_on(1, 0, 26.0, 1, false);
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
        let on_state = AcState::new_on(1, 0, 22.0, 1, false);
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
