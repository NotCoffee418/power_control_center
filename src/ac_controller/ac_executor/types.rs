use super::super::plan_types::{Intensity, RequestMode};

/// AC operation modes for API calls
pub const AC_MODE_COOL: i32 = 1;
pub const AC_MODE_HEAT: i32 = 4;

/// Temperature tolerance in Celsius for state change detection.
/// If the temperature difference is within this tolerance, we skip sending a new command.
pub const TEMPERATURE_TOLERANCE: f64 = 0.5;

/// Represents the actual state of an AC device
/// This is what we track to determine if we need to send new commands
#[derive(Debug, Clone, PartialEq)]
pub struct AcState {
    /// Whether the AC is currently on or off
    pub is_on: bool,
    /// AC mode: 1 = Heat, 4 = Cool
    pub mode: Option<i32>,
    /// Fan speed setting (0-5, where 0 is auto)
    pub fan_speed: Option<i32>,
    /// Target temperature in Celsius
    pub temperature: Option<f64>,
    /// Swing setting (0 = off, 1 = on)
    pub swing: Option<i32>,
    /// Whether powerful mode is active
    pub powerful_mode: bool,
}

impl AcState {
    /// Create a new state representing an off AC
    /// Sets mode to Some(0) to indicate OFF mode was explicitly set,
    /// allowing is_defined checks to properly detect that a command was sent
    pub fn new_off() -> Self {
        Self {
            is_on: false,
            mode: Some(0), // 0 = Off mode (changed from None to track command was sent)
            fan_speed: None,
            temperature: None,
            swing: None,
            powerful_mode: false,
        }
    }

    /// Create a new state representing an on AC with specific settings
    pub fn new_on(mode: i32, fan_speed: i32, temperature: f64, swing: i32, powerful_mode: bool) -> Self {
        Self {
            is_on: true,
            mode: Some(mode),
            fan_speed: Some(fan_speed),
            temperature: Some(temperature),
            swing: Some(swing),
            powerful_mode,
        }
    }

    /// Check if this state represents a change from another state
    /// Returns true if the states are different and a command should be sent.
    /// 
    /// Note: Temperature changes within ±0.5°C are considered equivalent
    /// to avoid sending redundant commands for minor temperature fluctuations.
    pub fn requires_change(&self, other: &AcState) -> bool {
        // If on/off state differs, it's definitely a change
        if self.is_on != other.is_on {
            return true;
        }
        
        // If both are off, no change needed (off state has no other properties that matter)
        if !self.is_on && !other.is_on {
            return false;
        }
        
        // Both are on, check other properties
        if self.mode != other.mode {
            return true;
        }
        
        if self.fan_speed != other.fan_speed {
            return true;
        }
        
        if self.swing != other.swing {
            return true;
        }
        
        if self.powerful_mode != other.powerful_mode {
            return true;
        }
        
        // Check temperature with tolerance
        match (self.temperature, other.temperature) {
            (Some(t1), Some(t2)) => {
                // Temperature change within tolerance is not considered a change
                (t1 - t2).abs() > TEMPERATURE_TOLERANCE
            }
            // If one has temperature and other doesn't, it's a change
            (Some(_), None) | (None, Some(_)) => true,
            // If neither has temperature (shouldn't happen for on state), no change
            (None, None) => false,
        }
    }
}

/// Converts a RequestMode plan into a concrete AcState
/// The device_name parameter is reserved for future device-specific settings
pub fn plan_to_state(mode: &RequestMode, intensity: &Intensity, _device_name: &str) -> AcState {
    match mode {
        RequestMode::Off => {
            // Explicitly turn off the device
            AcState::new_off()
        }
        RequestMode::NoChange => {
            // NoChange means keep current state, but we can't query current state here
            // So we default to off as a safe fallback
            // In practice, this should not cause changes due to state comparison in executor
            AcState::new_off()
        }
        RequestMode::Colder => {
            // Cooling mode
            let (fan_speed, temperature, powerful) = get_settings_for_intensity(intensity, true);
            AcState::new_on(
                AC_MODE_COOL,
                fan_speed,
                temperature,
                1, // swing on for cooling
                powerful,
            )
        }
        RequestMode::Warmer => {
            // Heating mode
            let (fan_speed, temperature, powerful) = get_settings_for_intensity(intensity, false);
            AcState::new_on(
                AC_MODE_HEAT,
                fan_speed,
                temperature,
                0, // swing off for heating
                powerful,
            )
        }
    }
}

/// Get appropriate settings based on intensity level
/// Returns (fan_speed, temperature, powerful_mode)
/// is_cooling determines if we're in cooling or heating mode
fn get_settings_for_intensity(intensity: &Intensity, is_cooling: bool) -> (i32, f64, bool) {
    match intensity {
        Intensity::Low => {
            // Minimal operation - just prevent extreme temperatures
            let temp = if is_cooling { 26.0 } else { 19.0 };
            (0, temp, false) // 0 = auto fan speed
        }
        Intensity::Medium => {
            // Comfortable operation for when user is home
            let temp = if is_cooling { 22.0 } else { 22.0 };
            (0, temp, false) // 0 = auto fan speed
        }
        Intensity::High => {
            // Powerful mode - when excess solar available
            let temp = if is_cooling { 20.0 } else { 24.0 };
            (1, temp, true) // 1 = High fan speed, powerful mode on
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ac_mode_constants() {
        // Verify AC modes are defined correctly (API: 1=Cool, 4=Heat)
        assert_eq!(AC_MODE_COOL, 1);
        assert_eq!(AC_MODE_HEAT, 4);
    }

    #[test]
    fn test_ac_state_equality() {
        let state1 = AcState::new_on(1, 0, 22.0, 1, false);
        let state2 = AcState::new_on(1, 0, 22.0, 1, false);
        let state3 = AcState::new_on(4, 0, 22.0, 0, false);

        assert_eq!(state1, state2);
        assert_ne!(state1, state3);
    }

    #[test]
    fn test_ac_state_off_equality() {
        let state1 = AcState::new_off();
        let state2 = AcState::new_off();
        let state3 = AcState::new_on(1, 0, 22.0, 1, false);

        assert_eq!(state1, state2);
        assert_ne!(state1, state3);
    }

    #[test]
    fn test_requires_change() {
        let state1 = AcState::new_on(1, 0, 22.0, 1, false);
        let state2 = AcState::new_on(1, 0, 22.0, 1, false);
        let state3 = AcState::new_on(4, 0, 22.0, 0, false);

        assert!(!state1.requires_change(&state2));
        assert!(state1.requires_change(&state3));
    }

    #[test]
    fn test_plan_to_state_off() {
        let state = plan_to_state(&RequestMode::Off, &Intensity::Low, "LivingRoom");

        assert_eq!(state, AcState::new_off());
    }

    #[test]
    fn test_plan_to_state_no_change() {
        let state = plan_to_state(&RequestMode::NoChange, &Intensity::Low, "LivingRoom");

        assert_eq!(state, AcState::new_off());
    }

    #[test]
    fn test_plan_to_state_colder_low() {
        let state = plan_to_state(&RequestMode::Colder, &Intensity::Low, "LivingRoom");

        assert!(state.is_on);
        assert_eq!(state.mode, Some(1)); // Cool mode
        assert_eq!(state.temperature, Some(26.0));
        assert_eq!(state.fan_speed, Some(0)); // Auto
        assert_eq!(state.swing, Some(1)); // On for cooling
        assert!(!state.powerful_mode);
    }

    #[test]
    fn test_plan_to_state_colder_medium() {
        let state = plan_to_state(&RequestMode::Colder, &Intensity::Medium, "LivingRoom");

        assert!(state.is_on);
        assert_eq!(state.mode, Some(1)); // Cool mode
        assert_eq!(state.temperature, Some(22.0));
        assert_eq!(state.fan_speed, Some(0)); // Auto
        assert_eq!(state.swing, Some(1)); // On for cooling
        assert!(!state.powerful_mode);
    }

    #[test]
    fn test_plan_to_state_colder_high() {
        let state = plan_to_state(&RequestMode::Colder, &Intensity::High, "LivingRoom");

        assert!(state.is_on);
        assert_eq!(state.mode, Some(1)); // Cool mode
        assert_eq!(state.temperature, Some(20.0));
        assert_eq!(state.fan_speed, Some(1)); // High
        assert_eq!(state.swing, Some(1)); // On for cooling
        assert!(state.powerful_mode);
    }

    #[test]
    fn test_plan_to_state_warmer_low() {
        let state = plan_to_state(&RequestMode::Warmer, &Intensity::Low, "LivingRoom");

        assert!(state.is_on);
        assert_eq!(state.mode, Some(4)); // Heat mode
        assert_eq!(state.temperature, Some(19.0));
        assert_eq!(state.fan_speed, Some(0)); // Auto
        assert_eq!(state.swing, Some(0)); // Off for heating
        assert!(!state.powerful_mode);
    }

    #[test]
    fn test_plan_to_state_warmer_medium() {
        let state = plan_to_state(&RequestMode::Warmer, &Intensity::Medium, "LivingRoom");

        assert!(state.is_on);
        assert_eq!(state.mode, Some(4)); // Heat mode
        assert_eq!(state.temperature, Some(22.0));
        assert_eq!(state.fan_speed, Some(0)); // Auto
        assert_eq!(state.swing, Some(0)); // Off for heating
        assert!(!state.powerful_mode);
    }

    #[test]
    fn test_plan_to_state_warmer_high() {
        let state = plan_to_state(&RequestMode::Warmer, &Intensity::High, "LivingRoom");

        assert!(state.is_on);
        assert_eq!(state.mode, Some(4)); // Heat mode
        assert_eq!(state.temperature, Some(24.0));
        assert_eq!(state.fan_speed, Some(1)); // High
        assert_eq!(state.swing, Some(0)); // Off for heating
        assert!(state.powerful_mode);
    }

    #[test]
    fn test_state_change_detection_temperature() {
        let state1 = AcState::new_on(1, 0, 22.0, 1, false);
        let state2 = AcState::new_on(1, 0, 23.0, 1, false);

        assert!(state1.requires_change(&state2));
    }

    #[test]
    fn test_state_change_detection_mode() {
        let state1 = AcState::new_on(1, 0, 22.0, 1, false);
        let state2 = AcState::new_on(4, 0, 22.0, 1, false);

        assert!(state1.requires_change(&state2));
    }

    #[test]
    fn test_state_change_detection_powerful() {
        let state1 = AcState::new_on(1, 0, 22.0, 1, false);
        let state2 = AcState::new_on(1, 0, 22.0, 1, true);

        assert!(state1.requires_change(&state2));
    }

    #[test]
    fn test_state_change_detection_on_to_off() {
        let state1 = AcState::new_on(1, 0, 22.0, 1, false);
        let state2 = AcState::new_off();

        assert!(state1.requires_change(&state2));
    }

    #[test]
    fn test_temperature_tolerance_within_threshold() {
        // Temperature difference of 0.3°C should NOT require a change (within ±0.5°C tolerance)
        let state1 = AcState::new_on(1, 0, 22.0, 1, false);
        let state2 = AcState::new_on(1, 0, 22.3, 1, false);

        assert!(!state1.requires_change(&state2), "0.3°C difference should not require a change");
    }

    #[test]
    fn test_temperature_tolerance_at_threshold() {
        // Temperature difference of exactly 0.5°C should NOT require a change (at tolerance boundary)
        let state1 = AcState::new_on(1, 0, 22.0, 1, false);
        let state2 = AcState::new_on(1, 0, 22.5, 1, false);

        assert!(!state1.requires_change(&state2), "0.5°C difference (at tolerance) should not require a change");
    }

    #[test]
    fn test_temperature_tolerance_above_threshold() {
        // Temperature difference of 0.51°C should require a change (above tolerance)
        let state1 = AcState::new_on(1, 0, 22.0, 1, false);
        let state2 = AcState::new_on(1, 0, 22.51, 1, false);

        assert!(state1.requires_change(&state2), "0.51°C difference should require a change");
    }

    #[test]
    fn test_temperature_tolerance_negative_difference() {
        // Negative temperature difference should also be handled
        let state1 = AcState::new_on(1, 0, 22.0, 1, false);
        let state2 = AcState::new_on(1, 0, 21.6, 1, false); // 0.4°C lower

        assert!(!state1.requires_change(&state2), "-0.4°C difference should not require a change");
    }

    #[test]
    fn test_temperature_tolerance_large_difference() {
        // Large temperature difference should always require a change
        let state1 = AcState::new_on(1, 0, 22.0, 1, false);
        let state2 = AcState::new_on(1, 0, 25.0, 1, false); // 3°C higher

        assert!(state1.requires_change(&state2), "3°C difference should require a change");
    }

    #[test]
    fn test_same_state_no_change() {
        // Exactly the same state should not require a change
        let state1 = AcState::new_on(1, 0, 22.0, 1, false);
        let state2 = AcState::new_on(1, 0, 22.0, 1, false);

        assert!(!state1.requires_change(&state2), "Same state should not require a change");
    }

    #[test]
    fn test_off_to_off_no_change() {
        // Both off should not require a change
        let state1 = AcState::new_off();
        let state2 = AcState::new_off();

        assert!(!state1.requires_change(&state2), "Off to off should not require a change");
    }

    #[test]
    fn test_off_to_on_requires_change() {
        // Off to on should require a change
        let state1 = AcState::new_off();
        let state2 = AcState::new_on(1, 0, 22.0, 1, false);

        assert!(state1.requires_change(&state2), "Off to on should require a change");
    }
}
