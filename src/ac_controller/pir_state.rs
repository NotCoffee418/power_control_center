use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// Global PIR state manager
static PIR_STATE: OnceLock<Arc<PirState>> = OnceLock::new();

/// Thread-safe PIR detection state
pub struct PirState {
    last_detection: RwLock<HashMap<String, DateTime<Utc>>>,
}

impl PirState {
    fn new() -> Self {
        Self {
            last_detection: RwLock::new(HashMap::new()),
        }
    }

    /// Record a PIR detection for a specific device
    pub fn record_detection(&self, device: &str) {
        let mut map = self.last_detection.write().unwrap();
        map.insert(device.to_string(), Utc::now());
        log::info!("PIR detection recorded for device: {}", device);
    }

    /// Check if a device has had a recent PIR detection within the timeout
    pub fn has_recent_detection(&self, device: &str, timeout_minutes: u32) -> bool {
        let map = self.last_detection.read().unwrap();
        
        if let Some(last_time) = map.get(device) {
            let now = Utc::now();
            let duration = now.signed_duration_since(*last_time);
            let minutes_ago = duration.num_minutes();
            
            if minutes_ago >= 0 && minutes_ago < timeout_minutes as i64 {
                log::debug!(
                    "Device {} had PIR detection {} minutes ago (within {} minute timeout)",
                    device,
                    minutes_ago,
                    timeout_minutes
                );
                return true;
            }
        }
        
        false
    }

    /// Get the last detection time for a device (for testing/debugging)
    #[allow(dead_code)]
    pub fn get_last_detection(&self, device: &str) -> Option<DateTime<Utc>> {
        let map = self.last_detection.read().unwrap();
        map.get(device).copied()
    }
}

/// Get the global PIR state instance
pub fn get_pir_state() -> &'static Arc<PirState> {
    PIR_STATE.get_or_init(|| Arc::new(PirState::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_check_detection() {
        let state = PirState::new();
        
        // Initially no detection
        assert!(!state.has_recent_detection("TestDevice", 5));
        
        // Record detection
        state.record_detection("TestDevice");
        
        // Should have recent detection
        assert!(state.has_recent_detection("TestDevice", 5));
        
        // Different device should not have detection
        assert!(!state.has_recent_detection("OtherDevice", 5));
    }

    #[test]
    fn test_detection_timeout() {
        let state = PirState::new();
        
        // Record detection
        state.record_detection("TestDevice");
        
        // Should be detected with 1 minute timeout
        assert!(state.has_recent_detection("TestDevice", 1));
        
        // Simulate time passing (we can't actually wait, so we'll test the logic)
        // This test verifies the logic works, actual timeout would need time manipulation
        
        // With 0 minute timeout, it should not be recent
        assert!(!state.has_recent_detection("TestDevice", 0));
    }

    #[test]
    fn test_multiple_devices() {
        let state = PirState::new();
        
        state.record_detection("Device1");
        state.record_detection("Device2");
        
        assert!(state.has_recent_detection("Device1", 5));
        assert!(state.has_recent_detection("Device2", 5));
        assert!(!state.has_recent_detection("Device3", 5));
    }

    #[test]
    fn test_get_last_detection() {
        let state = PirState::new();
        
        assert!(state.get_last_detection("TestDevice").is_none());
        
        state.record_detection("TestDevice");
        
        let detection_time = state.get_last_detection("TestDevice");
        assert!(detection_time.is_some());
        
        // Detection should be very recent (within last second)
        let now = Utc::now();
        let diff = now.signed_duration_since(detection_time.unwrap());
        assert!(diff.num_seconds() < 2);
    }
}
