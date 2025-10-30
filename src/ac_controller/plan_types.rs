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

/// AC operation modes
const AC_MODE_HEAT: i32 = 1;
const AC_MODE_COOL: i32 = 4;


/// Vague request for changing temperature
/// To be specified by settings
pub(super) enum RequestMode {
    Colder(Intensity),
    Warmer(Intensity),
    NoChange,
}

/// Intensity levels of desired temperature change
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

/// Get the desired AC plan for a specific device
/// This determines what mode the AC should be in based on various conditions
pub(super) async fn get_plan(device: &AcDevices) -> RequestMode {
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
        // A bit cold and user is home - heat gently
        RequestMode::Warmer(Intensity::Medium)
    } else if current_temp > COMFORTABLE_TEMP_MAX && user_is_home {
        // A bit warm and user is home - cool gently
        RequestMode::Colder(Intensity::Medium)
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
        Ok(production) => Some(production.current_production as u32),
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
}
