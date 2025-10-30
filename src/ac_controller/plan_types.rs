use super::plan_helpers;
use crate::device_requests;

// Configurable parameters for AC behavior plans
/// Minimum active solar power to consider Powerful (High Intensity) mode
const SOLAR_HIGH_INTENSITY_WATT_THRESHOLD: u32 = 2000; // Watts

/// Temperature thresholds for comfort
const COMFORTABLE_TEMP_MIN: f64 = 20.0; // °C
const COMFORTABLE_TEMP_MAX: f64 = 24.0; // °C

/// Temperature thresholds for extreme discomfort
const TOO_COLD_THRESHOLD: f64 = 18.0; // °C
const TOO_HOT_THRESHOLD: f64 = 27.0; // °C

/// AC operation modes for API calls (for future implementation)
const AC_MODE_HEAT: i32 = 1;
const AC_MODE_COOL: i32 = 4;


/// Vague request for changing temperature
/// To be specified by settings
#[derive(Debug, PartialEq)]
pub(super) enum RequestMode {
    Colder(Intensity),
    Warmer(Intensity),
    NoChange,
}

/// Intensity levels of desired temperature change
#[derive(Debug, PartialEq)]
pub(super) enum Intensity {
    Low,    // Maintain not freezing/smelting temperature
    Medium, // Keep it comfortable
    High,   // Using Powerful, when excess solar power available
}

/// AcDevices must be defined in config
pub(super) enum AcDevices {
    LivingRoom,
    Veranda,
}
impl AcDevices {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            AcDevices::LivingRoom => "LivingRoom",
            AcDevices::Veranda => "Veranda",
        }
    }
}

/// Fetch data and get the desired AC plan for a specific device
/// This is the async wrapper that fetches data and calls get_plan
pub(super) async fn fetch_data_and_get_plan(device: &AcDevices) -> RequestMode {
    // Get current conditions
    let current_temp = match get_current_temperature(device).await {
        Some(temp) => temp,
        None => {
            // If we can't get temperature, default to no change
            return RequestMode::NoChange;
        }
    };

    let solar_production = get_solar_production_watts().await.unwrap_or(0);
    let user_is_home = plan_helpers::is_user_home_and_awake();

    // Call the pure function with fetched data
    get_plan(current_temp, solar_production, user_is_home)
}

/// Get the desired AC plan based on provided conditions
/// This is a pure function that can be easily unit tested
pub(super) fn get_plan(current_temp: f64, solar_production: u32, user_is_home: bool) -> RequestMode {
    // Determine intensity based on solar production
    let intensity = if solar_production >= SOLAR_HIGH_INTENSITY_WATT_THRESHOLD {
        Intensity::High
    } else if user_is_home {
        Intensity::Medium
    } else {
        Intensity::Low
    };

    // Decide on the mode based on temperature
    if current_temp < TOO_COLD_THRESHOLD {
        // Too cold - need heating
        RequestMode::Warmer(intensity)
    } else if current_temp > TOO_HOT_THRESHOLD {
        // Too hot - need cooling
        RequestMode::Colder(intensity)
    } else if current_temp < COMFORTABLE_TEMP_MIN && user_is_home {
        // A bit cold and user is home - use calculated intensity
        RequestMode::Warmer(intensity)
    } else if current_temp > COMFORTABLE_TEMP_MAX && user_is_home {
        // A bit warm and user is home - use calculated intensity
        RequestMode::Colder(intensity)
    } else {
        // Temperature is comfortable or user is not home
        RequestMode::NoChange
    }
}

/// Get current temperature for a specific AC device
async fn get_current_temperature(device: &AcDevices) -> Option<f64> {
    let device_name = device.as_str();
    
    match device_requests::ac::get_sensors(device_name).await {
        Ok(sensor_data) => Some(sensor_data.temperature),
        Err(e) => {
            log::error!("Failed to get temperature for {}: {}", device_name, e);
            None
        }
    }
}

/// Get current solar production in watts
async fn get_solar_production_watts() -> Option<u32> {
    match device_requests::meter::get_solar_production().await {
        Ok(production) => {
            // Only return positive values, treat negative as 0
            if production.current_production >= 0 {
                Some(production.current_production as u32)
            } else {
                Some(0)
            }
        },
        Err(e) => {
            log::error!("Failed to get solar production: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ac_devices_as_str() {
        assert_eq!(AcDevices::LivingRoom.as_str(), "LivingRoom");
        assert_eq!(AcDevices::Veranda.as_str(), "Veranda");
    }

    #[test]
    fn test_constants_are_sensible() {
        // Verify temperature thresholds are in the right order
        assert!(TOO_COLD_THRESHOLD < COMFORTABLE_TEMP_MIN);
        assert!(COMFORTABLE_TEMP_MIN < COMFORTABLE_TEMP_MAX);
        assert!(COMFORTABLE_TEMP_MAX < TOO_HOT_THRESHOLD);
        
        // Verify solar threshold is positive
        assert!(SOLAR_HIGH_INTENSITY_WATT_THRESHOLD > 0);
    }

    #[test]
    fn test_ac_mode_constants() {
        // Verify AC modes are defined
        assert_eq!(AC_MODE_HEAT, 1);
        assert_eq!(AC_MODE_COOL, 4);
    }

    // Cold mode tests
    #[test]
    fn test_cold_mode_extreme_temp_user_home() {
        // Very cold temperature (17°C), user is home, no solar
        let plan = get_plan(17.0, 0, true);
        match plan {
            RequestMode::Warmer(Intensity::Medium) => {}, // Expected: Medium because user is home
            _ => panic!("Expected Warmer with Medium intensity, got {:?}", plan),
        }
    }

    #[test]
    fn test_cold_mode_extreme_temp_high_solar() {
        // Very cold temperature (17°C), user not home, high solar
        let plan = get_plan(17.0, 2500, false);
        match plan {
            RequestMode::Warmer(Intensity::High) => {}, // Expected: High because high solar
            _ => panic!("Expected Warmer with High intensity, got {:?}", plan),
        }
    }

    // Warm mode tests
    #[test]
    fn test_warm_mode_extreme_temp_user_home() {
        // Very hot temperature (28°C), user is home, no solar
        let plan = get_plan(28.0, 0, true);
        match plan {
            RequestMode::Colder(Intensity::Medium) => {}, // Expected: Medium because user is home
            _ => panic!("Expected Colder with Medium intensity, got {:?}", plan),
        }
    }

    #[test]
    fn test_warm_mode_extreme_temp_high_solar() {
        // Very hot temperature (28°C), user not home, high solar
        let plan = get_plan(28.0, 2500, false);
        match plan {
            RequestMode::Colder(Intensity::High) => {}, // Expected: High because high solar
            _ => panic!("Expected Colder with High intensity, got {:?}", plan),
        }
    }

    // Additional edge case tests
    #[test]
    fn test_comfortable_temp_no_change() {
        // Comfortable temperature (22°C), should not change
        let plan = get_plan(22.0, 0, true);
        match plan {
            RequestMode::NoChange => {}, // Expected: No change
            _ => panic!("Expected NoChange, got {:?}", plan),
        }
    }

    #[test]
    fn test_slightly_cold_user_home() {
        // Slightly cold (19°C), user is home, no solar
        let plan = get_plan(19.0, 0, true);
        match plan {
            RequestMode::Warmer(Intensity::Medium) => {}, // Expected: Warm with Medium
            _ => panic!("Expected Warmer with Medium intensity, got {:?}", plan),
        }
    }

    #[test]
    fn test_slightly_warm_user_home() {
        // Slightly warm (25°C), user is home, no solar
        let plan = get_plan(25.0, 0, true);
        match plan {
            RequestMode::Colder(Intensity::Medium) => {}, // Expected: Cool with Medium
            _ => panic!("Expected Colder with Medium intensity, got {:?}", plan),
        }
    }

    #[test]
    fn test_slightly_cold_user_not_home() {
        // Slightly cold (19°C), user NOT home, no solar
        let plan = get_plan(19.0, 0, false);
        match plan {
            RequestMode::NoChange => {}, // Expected: No change when user not home
            _ => panic!("Expected NoChange, got {:?}", plan),
        }
    }
}
