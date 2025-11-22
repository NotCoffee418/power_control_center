use super::super::plan_types::{Intensity, RequestMode};

/// AC operation modes for API calls
pub const AC_MODE_HEAT: i32 = 1;
pub const AC_MODE_COOL: i32 = 4;

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
    pub fn new_off() -> Self {
        Self {
            is_on: false,
            mode: None,
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
    /// Returns true if the states are different and a command should be sent
    pub fn requires_change(&self, other: &AcState) -> bool {
        self != other
    }
}

/// Converts a RequestMode plan into a concrete AcState
/// Takes the device name to determine appropriate settings (currently unused but reserved for future use)
pub fn plan_to_state(plan: &RequestMode, _device_name: &str) -> AcState {
    match plan {
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
        RequestMode::Colder(intensity) => {
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
        RequestMode::Warmer(intensity) => {
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
            (5, temp, true) // 5 = max fan speed, powerful mode on
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ac_mode_constants() {
        // Verify AC modes are defined correctly
        assert_eq!(AC_MODE_HEAT, 1);
        assert_eq!(AC_MODE_COOL, 4);
    }

    #[test]
    fn test_ac_state_equality() {
        let state1 = AcState::new_on(4, 0, 22.0, 1, false);
        let state2 = AcState::new_on(4, 0, 22.0, 1, false);
        let state3 = AcState::new_on(1, 0, 22.0, 0, false);

        assert_eq!(state1, state2);
        assert_ne!(state1, state3);
    }

    #[test]
    fn test_ac_state_off_equality() {
        let state1 = AcState::new_off();
        let state2 = AcState::new_off();
        let state3 = AcState::new_on(4, 0, 22.0, 1, false);

        assert_eq!(state1, state2);
        assert_ne!(state1, state3);
    }

    #[test]
    fn test_requires_change() {
        let state1 = AcState::new_on(4, 0, 22.0, 1, false);
        let state2 = AcState::new_on(4, 0, 22.0, 1, false);
        let state3 = AcState::new_on(1, 0, 22.0, 0, false);

        assert!(!state1.requires_change(&state2));
        assert!(state1.requires_change(&state3));
    }

    #[test]
    fn test_plan_to_state_off() {
        let plan = RequestMode::Off;
        let state = plan_to_state(&plan, "LivingRoom");

        assert_eq!(state, AcState::new_off());
    }

    #[test]
    fn test_plan_to_state_no_change() {
        let plan = RequestMode::NoChange;
        let state = plan_to_state(&plan, "LivingRoom");

        assert_eq!(state, AcState::new_off());
    }

    #[test]
    fn test_plan_to_state_colder_low() {
        let plan = RequestMode::Colder(Intensity::Low);
        let state = plan_to_state(&plan, "LivingRoom");

        assert!(state.is_on);
        assert_eq!(state.mode, Some(4)); // Cool mode
        assert_eq!(state.temperature, Some(26.0));
        assert_eq!(state.fan_speed, Some(0)); // Auto
        assert_eq!(state.swing, Some(1)); // On for cooling
        assert!(!state.powerful_mode);
    }

    #[test]
    fn test_plan_to_state_colder_medium() {
        let plan = RequestMode::Colder(Intensity::Medium);
        let state = plan_to_state(&plan, "LivingRoom");

        assert!(state.is_on);
        assert_eq!(state.mode, Some(4)); // Cool mode
        assert_eq!(state.temperature, Some(22.0));
        assert_eq!(state.fan_speed, Some(0)); // Auto
        assert_eq!(state.swing, Some(1)); // On for cooling
        assert!(!state.powerful_mode);
    }

    #[test]
    fn test_plan_to_state_colder_high() {
        let plan = RequestMode::Colder(Intensity::High);
        let state = plan_to_state(&plan, "LivingRoom");

        assert!(state.is_on);
        assert_eq!(state.mode, Some(4)); // Cool mode
        assert_eq!(state.temperature, Some(20.0));
        assert_eq!(state.fan_speed, Some(5)); // Max
        assert_eq!(state.swing, Some(1)); // On for cooling
        assert!(state.powerful_mode);
    }

    #[test]
    fn test_plan_to_state_warmer_low() {
        let plan = RequestMode::Warmer(Intensity::Low);
        let state = plan_to_state(&plan, "LivingRoom");

        assert!(state.is_on);
        assert_eq!(state.mode, Some(1)); // Heat mode
        assert_eq!(state.temperature, Some(19.0));
        assert_eq!(state.fan_speed, Some(0)); // Auto
        assert_eq!(state.swing, Some(0)); // Off for heating
        assert!(!state.powerful_mode);
    }

    #[test]
    fn test_plan_to_state_warmer_medium() {
        let plan = RequestMode::Warmer(Intensity::Medium);
        let state = plan_to_state(&plan, "LivingRoom");

        assert!(state.is_on);
        assert_eq!(state.mode, Some(1)); // Heat mode
        assert_eq!(state.temperature, Some(22.0));
        assert_eq!(state.fan_speed, Some(0)); // Auto
        assert_eq!(state.swing, Some(0)); // Off for heating
        assert!(!state.powerful_mode);
    }

    #[test]
    fn test_plan_to_state_warmer_high() {
        let plan = RequestMode::Warmer(Intensity::High);
        let state = plan_to_state(&plan, "LivingRoom");

        assert!(state.is_on);
        assert_eq!(state.mode, Some(1)); // Heat mode
        assert_eq!(state.temperature, Some(24.0));
        assert_eq!(state.fan_speed, Some(5)); // Max
        assert_eq!(state.swing, Some(0)); // Off for heating
        assert!(state.powerful_mode);
    }

    #[test]
    fn test_state_change_detection_temperature() {
        let state1 = AcState::new_on(4, 0, 22.0, 1, false);
        let state2 = AcState::new_on(4, 0, 23.0, 1, false);

        assert!(state1.requires_change(&state2));
    }

    #[test]
    fn test_state_change_detection_mode() {
        let state1 = AcState::new_on(4, 0, 22.0, 1, false);
        let state2 = AcState::new_on(1, 0, 22.0, 1, false);

        assert!(state1.requires_change(&state2));
    }

    #[test]
    fn test_state_change_detection_powerful() {
        let state1 = AcState::new_on(4, 0, 22.0, 1, false);
        let state2 = AcState::new_on(4, 0, 22.0, 1, true);

        assert!(state1.requires_change(&state2));
    }

    #[test]
    fn test_state_change_detection_on_to_off() {
        let state1 = AcState::new_on(4, 0, 22.0, 1, false);
        let state2 = AcState::new_off();

        assert!(state1.requires_change(&state2));
    }
}
