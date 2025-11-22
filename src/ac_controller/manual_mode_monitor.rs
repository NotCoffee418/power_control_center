use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Tracks whether each device is in manual or automatic mode
pub struct ManualModeMonitor {
    /// Maps device name to is_automatic_mode flag
    /// true = Auto mode, false = Manual mode
    device_modes: Arc<RwLock<HashMap<String, bool>>>,
}

impl ManualModeMonitor {
    pub fn new() -> Self {
        Self {
            device_modes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update the mode for a device and detect if it transitioned from Manual to Auto
    /// Returns true if device just transitioned from Manual (false) to Auto (true)
    pub fn update_mode(&self, device_name: &str, is_automatic_mode: bool) -> bool {
        let mut modes = self.device_modes.write().unwrap();
        
        // Get the previous mode (None means we haven't tracked it yet)
        let previous_mode = modes.get(device_name).copied();
        
        // Update to the new mode
        modes.insert(device_name.to_string(), is_automatic_mode);
        
        // Detect Manual→Auto transition
        // Previous mode was Some(false) (Manual) and new mode is true (Auto)
        matches!(previous_mode, Some(false)) && is_automatic_mode
    }

    /// Get the current mode for a device
    /// Returns None if device hasn't been tracked yet
    pub fn get_mode(&self, device_name: &str) -> Option<bool> {
        let modes = self.device_modes.read().unwrap();
        modes.get(device_name).copied()
    }

    /// Check if a device is currently in manual mode
    /// Returns true if device is in manual mode or hasn't been tracked yet
    /// (we default to manual for safety)
    pub fn is_manual_mode(&self, device_name: &str) -> bool {
        let modes = self.device_modes.read().unwrap();
        match modes.get(device_name) {
            Some(is_auto) => !is_auto,
            None => true, // Default to manual mode if not yet tracked
        }
    }
}

/// Global instance of the manual mode monitor
static MANUAL_MODE_MONITOR: std::sync::OnceLock<ManualModeMonitor> = std::sync::OnceLock::new();

/// Get the global manual mode monitor instance
pub fn get_manual_mode_monitor() -> &'static ManualModeMonitor {
    MANUAL_MODE_MONITOR.get_or_init(ManualModeMonitor::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_manual() {
        let monitor = ManualModeMonitor::new();
        assert!(monitor.is_manual_mode("TestDevice"));
    }

    #[test]
    fn test_no_transition_on_first_update() {
        let monitor = ManualModeMonitor::new();
        
        // First update to auto should not trigger transition (no previous state)
        let transitioned = monitor.update_mode("TestDevice", true);
        assert!(!transitioned);
    }

    #[test]
    fn test_manual_to_auto_transition() {
        let monitor = ManualModeMonitor::new();
        
        // Set to manual mode
        let t1 = monitor.update_mode("TestDevice", false);
        assert!(!t1);
        assert!(monitor.is_manual_mode("TestDevice"));
        
        // Transition to auto mode - should trigger transition
        let t2 = monitor.update_mode("TestDevice", true);
        assert!(t2, "Should detect Manual→Auto transition");
        assert!(!monitor.is_manual_mode("TestDevice"));
    }

    #[test]
    fn test_auto_to_manual_no_transition() {
        let monitor = ManualModeMonitor::new();
        
        // Set to auto mode
        monitor.update_mode("TestDevice", true);
        
        // Change to manual mode - should NOT trigger our transition
        let transitioned = monitor.update_mode("TestDevice", false);
        assert!(!transitioned, "Auto→Manual should not trigger transition");
    }

    #[test]
    fn test_staying_in_auto_no_transition() {
        let monitor = ManualModeMonitor::new();
        
        // Set to auto
        monitor.update_mode("TestDevice", true);
        
        // Stay in auto - no transition
        let transitioned = monitor.update_mode("TestDevice", true);
        assert!(!transitioned);
    }

    #[test]
    fn test_staying_in_manual_no_transition() {
        let monitor = ManualModeMonitor::new();
        
        // Set to manual
        monitor.update_mode("TestDevice", false);
        
        // Stay in manual - no transition
        let transitioned = monitor.update_mode("TestDevice", false);
        assert!(!transitioned);
    }

    #[test]
    fn test_multiple_devices() {
        let monitor = ManualModeMonitor::new();
        
        // Device1: Manual → Auto
        monitor.update_mode("Device1", false);
        let t1 = monitor.update_mode("Device1", true);
        assert!(t1);
        
        // Device2: stays in Manual
        monitor.update_mode("Device2", false);
        let t2 = monitor.update_mode("Device2", false);
        assert!(!t2);
        
        // Verify states
        assert!(!monitor.is_manual_mode("Device1"));
        assert!(monitor.is_manual_mode("Device2"));
    }

    #[test]
    fn test_get_mode() {
        let monitor = ManualModeMonitor::new();
        
        // Untracked device
        assert_eq!(monitor.get_mode("Unknown"), None);
        
        // Manual mode
        monitor.update_mode("Device1", false);
        assert_eq!(monitor.get_mode("Device1"), Some(false));
        
        // Auto mode
        monitor.update_mode("Device2", true);
        assert_eq!(monitor.get_mode("Device2"), Some(true));
    }
}
