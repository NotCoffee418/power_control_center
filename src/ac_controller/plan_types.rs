use super::plan_helpers;
use crate::device_requests;
use crate::config;
use crate::types::CauseReason;

// Configurable parameters for AC behavior plans
/// Minimum active solar power to consider Powerful (High Intensity) mode
const SOLAR_HIGH_INTENSITY_WATT_THRESHOLD: u32 = 2000; // Watts

/// Temperature thresholds for comfort
const COMFORTABLE_TEMP_MIN: f64 = 20.0; // °C
const COMFORTABLE_TEMP_MAX: f64 = 24.0; // °C

/// Temperature thresholds for extreme discomfort
const TOO_COLD_THRESHOLD: f64 = 18.0; // °C
const TOO_HOT_THRESHOLD: f64 = 27.0; // °C

/// Significant temperature change threshold (for weather forecasting logic)
const SIGNIFICANT_TEMP_CHANGE: f64 = 3.0; // °C

/// Ice Exception: Outdoor temperature threshold below which AC should be off
const ICE_EXCEPTION_OUTDOOR_THRESHOLD: f64 = 5.0; // °C

/// Ice Exception: Indoor temperature threshold below which we override the ice exception
const ICE_EXCEPTION_INDOOR_OVERRIDE: f64 = 12.0; // °C

/// Input parameters for AC planning
#[derive(Debug, Clone)]
pub(super) struct PlanInput {
    pub current_indoor_temp: f64,
    pub solar_production: u32,
    pub user_is_home: bool,
    pub current_outdoor_temp: f64,
    pub avg_next_12h_outdoor_temp: f64,
}


/// Result of AC planning including the mode and the cause/reason
#[derive(Debug, PartialEq)]
pub struct PlanResult {
    pub mode: RequestMode,
    pub cause: CauseReason,
}

impl PlanResult {
    pub fn new(mode: RequestMode, cause: CauseReason) -> Self {
        Self { mode, cause }
    }
}

/// Vague request for changing temperature
/// To be specified by settings
#[derive(Debug, PartialEq)]
pub enum RequestMode {
    Colder(Intensity),
    Warmer(Intensity),
    NoChange,
}

/// Intensity levels of desired temperature change
#[derive(Debug, PartialEq)]
pub enum Intensity {
    Low,    // Maintain not freezing/smelting temperature
    Medium, // Keep it comfortable
    High,   // Using Powerful, when excess solar power available
}

/// AcDevices must be defined in config
pub enum AcDevices {
    LivingRoom,
    Veranda,
}
impl AcDevices {
    pub fn as_str(&self) -> &'static str {
        match self {
            AcDevices::LivingRoom => "LivingRoom",
            AcDevices::Veranda => "Veranda",
        }
    }

    /// Convert a device name string to AcDevices enum
    /// Returns None if the device name is not recognized
    pub fn from_str(device: &str) -> Option<Self> {
        match device {
            "LivingRoom" => Some(AcDevices::LivingRoom),
            "Veranda" => Some(AcDevices::Veranda),
            _ => None,
        }
    }

    /// Get all AC devices for iteration
    pub fn all() -> Vec<Self> {
        vec![
            AcDevices::LivingRoom,
            AcDevices::Veranda,
        ]
    }
}

/// Fetch data and get the desired AC plan for a specific device
/// This is the async wrapper that fetches data and calls get_plan
pub(super) async fn fetch_data_and_get_plan(device: &AcDevices) -> PlanResult {
    // Check for recent PIR detection first
    let pir_state = super::pir_state::get_pir_state();
    let config = config::get_config();
    
    if pir_state.has_recent_detection(device.as_str(), config.pir_timeout_minutes) {
        log::info!(
            "PIR detection active for {}, overriding plan to turn off AC",
            device.as_str()
        );
        // Return NoChange as a signal to turn off or keep off the AC
        // The actual turn-off is handled by the PIR detect endpoint
        // This prevents the AC from being turned back on during normal evaluation
        return PlanResult::new(RequestMode::NoChange, CauseReason::Undefined);
    }

    // Get current conditions
    let current_indoor_temp = match get_current_temperature(device).await {
        Some(temp) => temp,
        None => {
            // If we can't get temperature, default to no change
            return PlanResult::new(RequestMode::NoChange, CauseReason::Undefined);
        }
    };

    let solar_production = get_solar_production_watts().await.unwrap_or(0);
    let user_is_home = plan_helpers::is_user_home_and_awake();
    let current_outdoor_temp = get_current_outdoor_temp().await;
    let avg_next_12h_outdoor_temp = get_avg_next_12h_outdoor_temp().await;

    // Build input struct
    let input = PlanInput {
        current_indoor_temp,
        solar_production,
        user_is_home,
        current_outdoor_temp,
        avg_next_12h_outdoor_temp,
    };

    // Call the pure function with fetched data
    get_plan(&input)
}

/// Get the desired AC plan based on provided conditions
/// This is a pure function that can be easily unit tested
pub(super) fn get_plan(input: &PlanInput) -> PlanResult {
    // Ice Exception: If outdoor temp is below 5°C, turn AC OFF to prevent ice formation
    // UNLESS indoor temp is below 12°C, then we continue with normal planning
    if input.current_outdoor_temp < ICE_EXCEPTION_OUTDOOR_THRESHOLD 
        && input.current_indoor_temp >= ICE_EXCEPTION_INDOOR_OVERRIDE {
        log::info!(
            "Ice Exception triggered: outdoor temp {:.1}°C < {:.1}°C, indoor temp {:.1}°C >= {:.1}°C. AC will be OFF.",
            input.current_outdoor_temp,
            ICE_EXCEPTION_OUTDOOR_THRESHOLD,
            input.current_indoor_temp,
            ICE_EXCEPTION_INDOOR_OVERRIDE
        );
        return PlanResult::new(RequestMode::NoChange, CauseReason::IceException);
    }
    
    let mode = calculate_request_mode(input);
    PlanResult::new(mode, CauseReason::Undefined)
}

/// Calculate the request mode based on temperature and other conditions
fn calculate_request_mode(input: &PlanInput) -> RequestMode {
    // Calculate temperature forecast trend
    let temp_trend = input.avg_next_12h_outdoor_temp - input.current_outdoor_temp;
    let getting_significantly_colder = temp_trend < -SIGNIFICANT_TEMP_CHANGE;
    let getting_significantly_warmer = temp_trend > SIGNIFICANT_TEMP_CHANGE;
    
    // Adjust solar threshold based on weather forecast
    // If it's getting significantly colder or warmer, we want to use excess capacity now
    let effective_solar_threshold = if getting_significantly_colder || getting_significantly_warmer {
        // Lower threshold = easier to trigger high intensity
        SOLAR_HIGH_INTENSITY_WATT_THRESHOLD / 2
    } else {
        SOLAR_HIGH_INTENSITY_WATT_THRESHOLD
    };

    // Determine intensity based on solar production and weather forecast
    let intensity = if input.solar_production >= effective_solar_threshold {
        Intensity::High
    } else if input.user_is_home {
        Intensity::Medium
    } else {
        Intensity::Low
    };

    // Decide on the mode based on temperature
    if input.current_indoor_temp < TOO_COLD_THRESHOLD {
        // Too cold - need heating
        RequestMode::Warmer(intensity)
    } else if input.current_indoor_temp > TOO_HOT_THRESHOLD {
        // Too hot - need cooling
        RequestMode::Colder(intensity)
    } else if input.current_indoor_temp < COMFORTABLE_TEMP_MIN && input.user_is_home {
        // A bit cold and user is home - use calculated intensity
        RequestMode::Warmer(intensity)
    } else if input.current_indoor_temp > COMFORTABLE_TEMP_MAX && input.user_is_home {
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
/// Falls back to power meter production value if solar API fails
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
            log::warn!("Failed to get solar production from API: {}. Trying power meter fallback.", e);
            
            // Fallback: Try to get production from power meter
            match device_requests::meter::get_latest_reading().await {
                Ok(reading) => {
                    // current_production_kw is in kilowatts, convert to watts
                    // Use max(0.0) to ensure non-negative before conversion
                    let production_watts = (reading.current_production_kw * 1000.0).max(0.0) as u32;
                    
                    // Only use if production is positive (indicating solar is producing)
                    if production_watts > 0 {
                        log::info!("Using power meter production as solar fallback: {} W", production_watts);
                        Some(production_watts)
                    } else {
                        log::info!("Power meter shows no production, assuming 0 solar");
                        Some(0)
                    }
                },
                Err(meter_err) => {
                    log::error!("Failed to get power meter reading as fallback: {}. Assuming 0 solar.", meter_err);
                    Some(0)
                }
            }
        }
    }
}

/// Get current outdoor temperature
/// Uses cached version with stale fallback on API failure
async fn get_current_outdoor_temp() -> f64 {
    let cfg = config::get_config();
    match device_requests::weather::get_current_outdoor_temp_cached(cfg.latitude, cfg.longitude).await {
        Ok(temp) => temp,
        Err(e) => {
            log::error!("Failed to get current outdoor temperature: {}. Using default.", e);
            20.0 // Default to 20°C on error (only if no stale cache exists)
        }
    }
}

/// Get average outdoor temperature for next 12 hours
/// Note: This uses the non-cached version as trend needs both current and forecast,
/// and compute_temperature_trend_cached already handles caching with stale fallback
async fn get_avg_next_12h_outdoor_temp() -> f64 {
    let cfg = config::get_config();
    // Use current temp from cache since we just fetched it
    match device_requests::weather::get_current_outdoor_temp_cached(cfg.latitude, cfg.longitude).await {
        Ok(current) => {
            // Try to get the trend (which is cached with stale fallback)
            match device_requests::weather::compute_temperature_trend_cached(cfg.latitude, cfg.longitude).await {
                Ok(trend) => current + trend,
                Err(e) => {
                    log::error!("Failed to get temperature trend: {}. Using current as forecast.", e);
                    current // Use current temp as forecast if trend unavailable
                }
            }
        }
        Err(e) => {
            log::error!("Failed to get outdoor temperature for forecast: {}. Using default.", e);
            20.0 // Default to 20°C on error (only if no stale cache exists)
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
    fn test_ac_devices_from_str() {
        // Test valid device names
        assert!(matches!(AcDevices::from_str("LivingRoom"), Some(AcDevices::LivingRoom)));
        assert!(matches!(AcDevices::from_str("Veranda"), Some(AcDevices::Veranda)));
        
        // Test invalid device names
        assert!(AcDevices::from_str("Unknown").is_none());
        assert!(AcDevices::from_str("").is_none());
        assert!(AcDevices::from_str("livingroom").is_none()); // Case sensitive
    }

    #[test]
    fn test_ac_devices_round_trip() {
        // Test that as_str and from_str are consistent
        let devices = vec![AcDevices::LivingRoom, AcDevices::Veranda];
        for device in devices {
            let device_str = device.as_str();
            let parsed = AcDevices::from_str(device_str);
            assert!(parsed.is_some());
            assert_eq!(parsed.unwrap().as_str(), device_str);
        }
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



    // Cold mode tests
    #[test]
    fn test_cold_mode_extreme_temp_user_home() {
        // Very cold temperature (17°C), user is home, no solar
        let input = PlanInput {
            current_indoor_temp: 17.0,
            solar_production: 0,
            user_is_home: true,
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 15.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer(Intensity::Medium) => {}, // Expected: Medium because user is home
            _ => panic!("Expected Warmer with Medium intensity, got {:?}", plan.mode),
        }
    }

    #[test]
    fn test_cold_mode_extreme_temp_high_solar() {
        // Very cold temperature (17°C), user not home, high solar
        let input = PlanInput {
            current_indoor_temp: 17.0,
            solar_production: 2500,
            user_is_home: false,
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 15.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer(Intensity::High) => {}, // Expected: High because high solar
            _ => panic!("Expected Warmer with High intensity, got {:?}", plan.mode),
        }
    }

    // Warm mode tests
    #[test]
    fn test_warm_mode_extreme_temp_user_home() {
        // Very hot temperature (28°C), user is home, no solar
        let input = PlanInput {
            current_indoor_temp: 28.0,
            solar_production: 0,
            user_is_home: true,
            current_outdoor_temp: 30.0,
            avg_next_12h_outdoor_temp: 30.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder(Intensity::Medium) => {}, // Expected: Medium because user is home
            _ => panic!("Expected Colder with Medium intensity, got {:?}", plan.mode),
        }
    }

    #[test]
    fn test_warm_mode_extreme_temp_high_solar() {
        // Very hot temperature (28°C), user not home, high solar
        let input = PlanInput {
            current_indoor_temp: 28.0,
            solar_production: 2500,
            user_is_home: false,
            current_outdoor_temp: 30.0,
            avg_next_12h_outdoor_temp: 30.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder(Intensity::High) => {}, // Expected: High because high solar
            _ => panic!("Expected Colder with High intensity, got {:?}", plan.mode),
        }
    }

    // Additional edge case tests
    #[test]
    fn test_comfortable_temp_no_change() {
        // Comfortable temperature (22°C), should not change
        let input = PlanInput {
            current_indoor_temp: 22.0,
            solar_production: 0,
            user_is_home: true,
            current_outdoor_temp: 20.0,
            avg_next_12h_outdoor_temp: 20.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::NoChange => {}, // Expected: No change
            _ => panic!("Expected NoChange, got {:?}", plan.mode),
        }
    }

    #[test]
    fn test_slightly_cold_user_home() {
        // Slightly cold (19°C), user is home, no solar
        let input = PlanInput {
            current_indoor_temp: 19.0,
            solar_production: 0,
            user_is_home: true,
            current_outdoor_temp: 18.0,
            avg_next_12h_outdoor_temp: 18.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer(Intensity::Medium) => {}, // Expected: Warm with Medium
            _ => panic!("Expected Warmer with Medium intensity, got {:?}", plan.mode),
        }
    }

    #[test]
    fn test_slightly_warm_user_home() {
        // Slightly warm (25°C), user is home, no solar
        let input = PlanInput {
            current_indoor_temp: 25.0,
            solar_production: 0,
            user_is_home: true,
            current_outdoor_temp: 26.0,
            avg_next_12h_outdoor_temp: 26.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder(Intensity::Medium) => {}, // Expected: Cool with Medium
            _ => panic!("Expected Colder with Medium intensity, got {:?}", plan.mode),
        }
    }

    #[test]
    fn test_slightly_cold_user_not_home() {
        // Slightly cold (19°C), user NOT home, no solar
        let input = PlanInput {
            current_indoor_temp: 19.0,
            solar_production: 0,
            user_is_home: false,
            current_outdoor_temp: 18.0,
            avg_next_12h_outdoor_temp: 18.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::NoChange => {}, // Expected: No change when user not home
            _ => panic!("Expected NoChange, got {:?}", plan.mode),
        }
    }

    // Weather forecast tests
    #[test]
    fn test_getting_significantly_colder_boosts_intensity() {
        // Current outdoor 20°C, forecast 15°C (5° drop), moderate solar
        // Should trigger high intensity due to weather forecast
        let input = PlanInput {
            current_indoor_temp: 19.0,
            solar_production: 1200, // Below normal threshold, but above lowered threshold
            user_is_home: true,
            current_outdoor_temp: 20.0,
            avg_next_12h_outdoor_temp: 15.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer(Intensity::High) => {}, // Expected: High due to weather forecast
            _ => panic!("Expected Warmer with High intensity due to forecast, got {:?}", plan.mode),
        }
    }

    #[test]
    fn test_getting_significantly_warmer_boosts_intensity() {
        // Current outdoor 20°C, forecast 25°C (5° increase), moderate solar
        // Should trigger high intensity due to weather forecast
        let input = PlanInput {
            current_indoor_temp: 25.0,
            solar_production: 1200, // Below normal threshold, but above lowered threshold
            user_is_home: true,
            current_outdoor_temp: 20.0,
            avg_next_12h_outdoor_temp: 25.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder(Intensity::High) => {}, // Expected: High due to weather forecast
            _ => panic!("Expected Colder with High intensity due to forecast, got {:?}", plan.mode),
        }
    }

    #[test]
    fn test_zero_solar_production() {
        // Test that 0 solar production results in low intensity when user not home
        let input = PlanInput {
            current_indoor_temp: 17.0, // Too cold
            solar_production: 0,
            user_is_home: false,
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 15.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer(Intensity::Low) => {}, // Expected: Low because no solar
            _ => panic!("Expected Warmer with Low intensity, got {:?}", plan.mode),
        }
    }

    #[test]
    fn test_moderate_solar_with_forecast_change() {
        // Test that moderate solar (1200W) triggers high intensity when forecast changes significantly
        let input = PlanInput {
            current_indoor_temp: 17.0, // Too cold
            solar_production: 1200, // Moderate, above half threshold (1000W)
            user_is_home: false,
            current_outdoor_temp: 20.0,
            avg_next_12h_outdoor_temp: 16.0, // Dropping 4°C
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer(Intensity::High) => {}, // Expected: High due to forecast + moderate solar
            _ => panic!("Expected Warmer with High intensity, got {:?}", plan.mode),
        }
    }

    // Ice Exception tests
    #[test]
    fn test_ice_exception_triggers() {
        // Outdoor temp below 5°C, indoor temp above 12°C
        // Should trigger ice exception and turn AC off
        let input = PlanInput {
            current_indoor_temp: 18.0,
            solar_production: 2000,
            user_is_home: true,
            current_outdoor_temp: 3.0, // Below 5°C
            avg_next_12h_outdoor_temp: 3.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::NoChange => {}, // Expected: NoChange due to ice exception
            _ => panic!("Expected NoChange due to ice exception, got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::IceException);
    }

    #[test]
    fn test_ice_exception_override_when_too_cold_indoors() {
        // Outdoor temp below 5°C, but indoor temp below 12°C
        // Should override ice exception and allow heating
        let input = PlanInput {
            current_indoor_temp: 10.0, // Below 12°C
            solar_production: 0,
            user_is_home: false,
            current_outdoor_temp: 3.0, // Below 5°C
            avg_next_12h_outdoor_temp: 3.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer(Intensity::Low) => {}, // Expected: Heating with low intensity
            _ => panic!("Expected Warmer with Low intensity, got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::Undefined);
    }

    #[test]
    fn test_ice_exception_does_not_trigger_above_threshold() {
        // Outdoor temp above 5°C, should not trigger ice exception
        let input = PlanInput {
            current_indoor_temp: 18.0,
            solar_production: 0,
            user_is_home: false,
            current_outdoor_temp: 7.0, // Above 5°C
            avg_next_12h_outdoor_temp: 7.0,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::NoChange => {}, // Expected: NoChange but not due to ice exception
            _ => panic!("Expected NoChange, got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::Undefined);
    }

    #[test]
    fn test_ice_exception_edge_case_at_boundary() {
        // Outdoor temp exactly at 5°C, indoor temp exactly at 12°C
        // At 5°C outdoor: condition is < 5°C, so NO ice exception
        // At 12°C indoor: condition is >= 12°C, so no override
        // Result: Normal planning applies, and 12°C is too cold -> heating
        let input = PlanInput {
            current_indoor_temp: 12.0,
            solar_production: 0,
            user_is_home: false,
            current_outdoor_temp: 5.0,
            avg_next_12h_outdoor_temp: 5.0,
        };
        let plan = get_plan(&input);
        // At exactly 5°C outdoor, ice exception should NOT trigger (< 5°C, not <= 5°C)
        // Temperature is 12°C which is between TOO_COLD (18°C) and comfortable (20°C)
        // User not home, so no heating
        match plan.mode {
            RequestMode::Warmer(Intensity::Low) => {}, // Expected: Heating because too cold
            _ => panic!("Expected Warmer(Low), got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::Undefined);
    }
}
