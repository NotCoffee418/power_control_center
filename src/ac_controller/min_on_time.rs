use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// Minimum time a device must stay on after being turned on (in minutes)
const MINIMUM_ON_TIME_MINUTES: i64 = 30;

/// Global minimum on-time state manager
static MIN_ON_TIME_STATE: OnceLock<Arc<MinOnTimeState>> = OnceLock::new();

/// Thread-safe minimum on-time tracking state
pub struct MinOnTimeState {
    /// Maps device name to the time it was last turned on
    last_turn_on: RwLock<HashMap<String, DateTime<Utc>>>,
}

impl MinOnTimeState {
    fn new() -> Self {
        Self {
            last_turn_on: RwLock::new(HashMap::new()),
        }
    }

    /// Record when a device was turned on
    pub fn record_turn_on(&self, device: &str) {
        let mut map = self.last_turn_on.write().unwrap();
        map.insert(device.to_string(), Utc::now());
        log::debug!("Recorded turn-on time for device: {}", device);
    }

    /// Clear the turn-on time for a device (e.g., when PIR is triggered)
    /// This allows the device to be turned off immediately
    pub fn clear_turn_on_time(&self, device: &str) {
        let mut map = self.last_turn_on.write().unwrap();
        map.remove(device);
        log::debug!("Cleared turn-on time for device: {}", device);
    }

    /// Check if a device has been on for at least the minimum time
    /// Returns true if the device can be turned off (either no turn-on recorded or minimum time has passed)
    pub fn can_turn_off(&self, device: &str) -> bool {
        let map = self.last_turn_on.read().unwrap();
        
        if let Some(turn_on_time) = map.get(device) {
            let now = Utc::now();
            let duration = now.signed_duration_since(*turn_on_time);
            let minutes_on = duration.num_minutes();
            
            if minutes_on >= MINIMUM_ON_TIME_MINUTES {
                log::debug!(
                    "Device {} has been on for {} minutes (>= {} minimum), can turn off",
                    device,
                    minutes_on,
                    MINIMUM_ON_TIME_MINUTES
                );
                return true;
            } else {
                log::info!(
                    "Device {} has been on for only {} minutes (< {} minimum), cannot turn off yet",
                    device,
                    minutes_on,
                    MINIMUM_ON_TIME_MINUTES
                );
                return false;
            }
        }
        
        // No turn-on time recorded, device can be turned off
        true
    }

    /// Get the last turn-on time for a device (for testing/debugging)
    pub fn get_last_turn_on(&self, device: &str) -> Option<DateTime<Utc>> {
        let map = self.last_turn_on.read().unwrap();
        map.get(device).copied()
    }

    /// Get how many minutes the device has been on (for debugging)
    pub fn get_minutes_on(&self, device: &str) -> Option<i64> {
        let map = self.last_turn_on.read().unwrap();
        if let Some(turn_on_time) = map.get(device) {
            let now = Utc::now();
            let duration = now.signed_duration_since(*turn_on_time);
            Some(duration.num_minutes())
        } else {
            None
        }
    }
}

/// Get the global minimum on-time state instance
pub fn get_min_on_time_state() -> &'static Arc<MinOnTimeState> {
    MIN_ON_TIME_STATE.get_or_init(|| Arc::new(MinOnTimeState::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_can_turn_off_immediately() {
        let state = MinOnTimeState::new();
        
        // Initially no turn-on recorded, can turn off
        assert!(state.can_turn_off("TestDevice"));
        
        // Record turn-on
        state.record_turn_on("TestDevice");
        
        // Cannot turn off immediately (less than 30 minutes)
        assert!(!state.can_turn_off("TestDevice"));
    }

    #[test]
    fn test_clear_turn_on_time_allows_turn_off() {
        let state = MinOnTimeState::new();
        
        // Record turn-on
        state.record_turn_on("TestDevice");
        
        // Cannot turn off immediately
        assert!(!state.can_turn_off("TestDevice"));
        
        // Clear the turn-on time
        state.clear_turn_on_time("TestDevice");
        
        // Can now turn off
        assert!(state.can_turn_off("TestDevice"));
    }

    #[test]
    fn test_multiple_devices() {
        let state = MinOnTimeState::new();
        
        state.record_turn_on("Device1");
        state.record_turn_on("Device2");
        
        // Both cannot turn off immediately
        assert!(!state.can_turn_off("Device1"));
        assert!(!state.can_turn_off("Device2"));
        
        // Device without turn-on can turn off
        assert!(state.can_turn_off("Device3"));
        
        // Clear one device
        state.clear_turn_on_time("Device1");
        assert!(state.can_turn_off("Device1"));
        assert!(!state.can_turn_off("Device2"));
    }

    #[test]
    fn test_get_last_turn_on() {
        let state = MinOnTimeState::new();
        
        assert!(state.get_last_turn_on("TestDevice").is_none());
        
        state.record_turn_on("TestDevice");
        
        let turn_on_time = state.get_last_turn_on("TestDevice");
        assert!(turn_on_time.is_some());
        
        // Turn on time should be very recent (within last second)
        let now = Utc::now();
        let diff = now.signed_duration_since(turn_on_time.unwrap());
        assert!(diff.num_seconds() < 2);
    }

    #[test]
    fn test_get_minutes_on() {
        let state = MinOnTimeState::new();
        
        assert!(state.get_minutes_on("TestDevice").is_none());
        
        state.record_turn_on("TestDevice");
        
        // Should be 0 minutes (just turned on)
        let minutes = state.get_minutes_on("TestDevice");
        assert!(minutes.is_some());
        assert!(minutes.unwrap() >= 0);
        assert!(minutes.unwrap() < 1);
    }
}
