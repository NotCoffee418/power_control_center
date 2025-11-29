use super::plan_helpers;
use crate::device_requests;
use crate::config;
use crate::types::CauseReason;

// Configurable parameters for AC behavior plans
/// Minimum active solar power to consider Powerful (High Intensity) mode
const SOLAR_HIGH_INTENSITY_WATT_THRESHOLD: u32 = 2000; // Watts

/// Minimum active solar power to consider Medium Intensity mode when user not home
const SOLAR_MEDIUM_INTENSITY_WATT_THRESHOLD: u32 = 1000; // Watts

/// Temperature thresholds for comfort
const COMFORTABLE_TEMP_MIN: f64 = 20.0; // °C
const COMFORTABLE_TEMP_MAX: f64 = 24.0; // °C

/// Temperature thresholds for extreme discomfort
const TOO_COLD_THRESHOLD: f64 = 18.0; // °C
const TOO_HOT_THRESHOLD: f64 = 27.0; // °C

/// Significant temperature change threshold (for weather forecasting logic)
const SIGNIFICANT_TEMP_CHANGE: f64 = 3.0; // °C

/// Ice Exception: Outdoor temperature threshold below which AC should be off
const ICE_EXCEPTION_OUTDOOR_THRESHOLD: f64 = 2.0; // °C

/// Ice Exception: Indoor temperature threshold below which we override the ice exception
const ICE_EXCEPTION_INDOOR_OVERRIDE: f64 = 12.0; // °C

/// Ice Exception: Solar production threshold above which we ignore the ice exception (in Watts)
const ICE_EXCEPTION_SOLAR_BYPASS_THRESHOLD: u32 = 1000; // Watts

/// Buffer around comfortable temperature range for "mild temperature" classification
/// Outdoor temperatures within this buffer of the comfortable range are considered mild
const MILD_TEMPERATURE_BUFFER: f64 = 2.0; // °C

/// Temperature overshoot for heating hysteresis
/// When heating, only turn off when temperature exceeds target by this amount
/// Example: if target is 21°C, turn off at 22°C
const HEATING_TURN_OFF_OVERSHOOT: f64 = 1.0; // °C

/// Temperature overshoot for cooling hysteresis
/// When cooling, only turn off when temperature goes below target by this amount
/// Example: if target is 24°C, turn off at 23°C
const COOLING_TURN_OFF_OVERSHOOT: f64 = 1.0; // °C

/// Input parameters for AC planning
#[derive(Debug, Clone)]
pub(super) struct PlanInput {
    pub current_indoor_temp: f64,
    pub solar_production: u32,
    pub user_is_home: bool,
    pub current_outdoor_temp: f64,
    pub avg_next_12h_outdoor_temp: f64,
    /// Current AC operating mode: Some(true) = heating, Some(false) = cooling, None = off
    /// Used for temperature hysteresis to prevent rapid on/off cycling
    pub current_ac_mode: Option<bool>,
}


/// Result of AC planning including the mode, intensity, and the cause/reason
#[derive(Debug, PartialEq)]
pub struct PlanResult {
    pub mode: RequestMode,
    pub intensity: Intensity,
    pub cause: CauseReason,
}

impl PlanResult {
    pub fn new(mode: RequestMode, intensity: Intensity, cause: CauseReason) -> Self {
        Self { mode, intensity, cause }
    }
}

/// Vague request for changing temperature
/// To be specified by settings
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RequestMode {
    Colder,
    Warmer,
    /// Explicitly turn the AC off (e.g., due to IceException, PIR detection)
    Off,
    /// No change needed - keep current state (e.g., comfortable temperature)
    NoChange,
}

/// Intensity levels of desired temperature change
#[derive(Debug, PartialEq, Clone, Copy)]
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
        // Return Off to explicitly turn off or keep off the AC
        // The actual turn-off is handled by the PIR detect endpoint
        // This prevents the AC from being turned back on during normal evaluation
        return PlanResult::new(RequestMode::Off, Intensity::Low, CauseReason::PirDetection);
    }

    // Get current conditions
    let current_indoor_temp = match get_current_temperature(device).await {
        Some(temp) => temp,
        None => {
            // If we can't get temperature, default to no change
            return PlanResult::new(RequestMode::NoChange, Intensity::Low, CauseReason::Undefined);
        }
    };

    let solar_production = get_solar_production_watts().await.unwrap_or(0);
    let user_is_home = plan_helpers::is_user_home_and_awake();
    let current_outdoor_temp = get_current_outdoor_temp().await;
    let avg_next_12h_outdoor_temp = get_avg_next_12h_outdoor_temp().await;

    // Get current AC state to implement temperature hysteresis
    // Some(true) = heating, Some(false) = cooling, None = off
    let current_ac_mode = get_current_ac_mode(device);

    // Build input struct
    let input = PlanInput {
        current_indoor_temp,
        solar_production,
        user_is_home,
        current_outdoor_temp,
        avg_next_12h_outdoor_temp,
        current_ac_mode,
    };

    // Call the pure function with fetched data
    get_plan(&input)
}

/// Get the desired AC plan based on provided conditions
/// This is a pure function that can be easily unit tested
pub(super) fn get_plan(input: &PlanInput) -> PlanResult {
    // Ice Exception: If outdoor temp is below 2°C, turn AC OFF to prevent ice formation
    // UNLESS:
    // - indoor temp is below 12°C, then we continue with normal planning
    // - solar production is above 1000W, which indicates sufficient energy/warmth to bypass the exception
    if input.current_outdoor_temp < ICE_EXCEPTION_OUTDOOR_THRESHOLD 
        && input.current_indoor_temp >= ICE_EXCEPTION_INDOOR_OVERRIDE 
        && input.solar_production < ICE_EXCEPTION_SOLAR_BYPASS_THRESHOLD {
        log::info!(
            "Ice Exception triggered: outdoor temp {:.1}°C < {:.1}°C, indoor temp {:.1}°C >= {:.1}°C, solar {:.0}W < {}W. AC will be OFF.",
            input.current_outdoor_temp,
            ICE_EXCEPTION_OUTDOOR_THRESHOLD,
            input.current_indoor_temp,
            ICE_EXCEPTION_INDOOR_OVERRIDE,
            input.solar_production,
            ICE_EXCEPTION_SOLAR_BYPASS_THRESHOLD
        );
        return PlanResult::new(RequestMode::Off, Intensity::Low, CauseReason::IceException);
    }
    
    calculate_request_mode_with_cause(input)
}

/// Calculate the request mode and cause based on temperature and other conditions
fn calculate_request_mode_with_cause(input: &PlanInput) -> PlanResult {
    // Calculate temperature forecast trend
    let temp_trend = input.avg_next_12h_outdoor_temp - input.current_outdoor_temp;
    let getting_significantly_colder = temp_trend < -SIGNIFICANT_TEMP_CHANGE;
    let getting_significantly_warmer = temp_trend > SIGNIFICANT_TEMP_CHANGE;
    let significant_temp_change_pending = getting_significantly_colder || getting_significantly_warmer;
    
    // Check if outdoor temp is close to comfortable range (mild temperature)
    let outdoor_temp_near_comfortable = 
        input.current_outdoor_temp >= COMFORTABLE_TEMP_MIN - MILD_TEMPERATURE_BUFFER 
        && input.current_outdoor_temp <= COMFORTABLE_TEMP_MAX + MILD_TEMPERATURE_BUFFER;
    
    // Adjust solar threshold based on weather forecast
    // If it's getting significantly colder or warmer, we want to use excess capacity now
    let effective_solar_threshold = if significant_temp_change_pending {
        // Lower threshold = easier to trigger high intensity
        SOLAR_HIGH_INTENSITY_WATT_THRESHOLD / 2
    } else {
        SOLAR_HIGH_INTENSITY_WATT_THRESHOLD
    };

    // Determine intensity and cause based on solar production, user presence, and weather forecast
    let (intensity, intensity_cause) = if input.solar_production >= effective_solar_threshold {
        // High intensity due to either major temp change or excessive solar
        if significant_temp_change_pending {
            (Intensity::High, CauseReason::MajorTemperatureChangePending)
        } else {
            (Intensity::High, CauseReason::ExcessiveSolarPower)
        }
    } else if input.solar_production >= SOLAR_MEDIUM_INTENSITY_WATT_THRESHOLD {
        // Medium solar production (1000W+) allows medium intensity even when user not home
        (Intensity::Medium, CauseReason::ExcessiveSolarPower)
    } else if input.user_is_home {
        (Intensity::Medium, CauseReason::Undefined)
    } else {
        // Low intensity - either nobody home or mild temperature
        let cause = if outdoor_temp_near_comfortable {
            CauseReason::MildTemperature
        } else {
            CauseReason::NobodyHome
        };
        (Intensity::Low, cause)
    };

    // Decide on the mode based on temperature
    // First determine if we need heating or cooling based on indoor temp
    let needs_heating = input.current_indoor_temp < TOO_COLD_THRESHOLD ||
                       (input.current_indoor_temp < COMFORTABLE_TEMP_MIN && input.user_is_home);
    let needs_cooling = input.current_indoor_temp > TOO_HOT_THRESHOLD ||
                       (input.current_indoor_temp > COMFORTABLE_TEMP_MAX && input.user_is_home);
    
    // Temperature trend validation: prevent counterproductive operations at high intensity
    // If we're about to do high intensity heating/cooling, check if weather trend makes it wasteful
    let (final_intensity, final_cause) = if intensity == Intensity::High {
        // Check if high intensity heating is counterproductive
        if needs_heating && getting_significantly_warmer {
            // Outside is getting hotter than inside - don't use high intensity heating
            // Use medium intensity instead to keep temperature reasonable
            (Intensity::Medium, CauseReason::ExcessiveSolarPower)
        }
        // Check if high intensity cooling is counterproductive  
        else if needs_cooling && getting_significantly_colder {
            // Outside is getting colder than inside - don't use high intensity cooling
            // Use medium intensity instead to keep temperature reasonable
            (Intensity::Medium, CauseReason::ExcessiveSolarPower)
        } else {
            (intensity, intensity_cause)
        }
    } else {
        (intensity, intensity_cause)
    };
    
    // Apply temperature hysteresis to prevent rapid on/off cycling
    // If currently heating, only stop when temperature reaches target + overshoot
    // If currently cooling, only stop when temperature reaches target - overshoot
    let (heating_turn_off_temp, cooling_turn_off_temp) = match input.current_ac_mode {
        Some(true) => {
            // Currently heating: require higher temperature to turn off
            // When user is home, turn off at COMFORTABLE_TEMP_MIN + overshoot
            // When too cold, turn off at TOO_COLD_THRESHOLD + overshoot
            (
                COMFORTABLE_TEMP_MIN + HEATING_TURN_OFF_OVERSHOOT,
                COMFORTABLE_TEMP_MAX - COOLING_TURN_OFF_OVERSHOOT,
            )
        }
        Some(false) => {
            // Currently cooling: require lower temperature to turn off
            (
                COMFORTABLE_TEMP_MIN + HEATING_TURN_OFF_OVERSHOOT,
                COMFORTABLE_TEMP_MAX - COOLING_TURN_OFF_OVERSHOOT,
            )
        }
        None => {
            // Not currently running, use normal thresholds to turn on
            (COMFORTABLE_TEMP_MIN, COMFORTABLE_TEMP_MAX)
        }
    };

    let mode = if input.current_indoor_temp < TOO_COLD_THRESHOLD {
        // Too cold - need heating
        RequestMode::Warmer
    } else if input.current_indoor_temp > TOO_HOT_THRESHOLD {
        // Too hot - need cooling
        RequestMode::Colder
    } else if input.current_ac_mode == Some(true) {
        // Currently heating - continue until we reach the hysteresis turn-off point
        if input.current_indoor_temp < heating_turn_off_temp {
            RequestMode::Warmer
        } else {
            // Reached the turn-off temperature, stop heating
            RequestMode::NoChange
        }
    } else if input.current_ac_mode == Some(false) {
        // Currently cooling - continue until we reach the hysteresis turn-off point
        if input.current_indoor_temp > cooling_turn_off_temp {
            RequestMode::Colder
        } else {
            // Reached the turn-off temperature, stop cooling
            RequestMode::NoChange
        }
    } else if input.current_indoor_temp < COMFORTABLE_TEMP_MIN && input.user_is_home {
        // A bit cold and user is home - start heating
        RequestMode::Warmer
    } else if input.current_indoor_temp > COMFORTABLE_TEMP_MAX && input.user_is_home {
        // A bit warm and user is home - start cooling
        RequestMode::Colder
    } else {
        // Temperature is comfortable or user is not home
        RequestMode::NoChange
    };

    PlanResult::new(mode, final_intensity, final_cause)
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

/// Get the current AC operating mode from the state manager
/// Returns Some(true) if heating, Some(false) if cooling, None if off
fn get_current_ac_mode(device: &AcDevices) -> Option<bool> {
    let state_manager = super::ac_executor::get_state_manager();
    let current_state = state_manager.get_state(device.as_str());
    
    if !current_state.is_on {
        return None;
    }
    
    // Check the mode: 1 = Heat, 4 = Cool
    match current_state.mode {
        Some(super::ac_executor::AC_MODE_HEAT) => Some(true),  // Heating
        Some(super::ac_executor::AC_MODE_COOL) => Some(false), // Cooling
        _ => None, // Unknown mode, treat as off
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
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Medium => {}, // Expected: Medium because user is home
            _ => panic!("Expected Warmer with Medium intensity, got {:?}", plan.mode),
        }
        // Medium intensity = user is home = Undefined cause
        assert_eq!(plan.cause, CauseReason::Undefined);
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
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::High => {}, // Expected: High because high solar
            _ => panic!("Expected Warmer with High intensity, got {:?}", plan.mode),
        }
        // High intensity, no significant temp change = ExcessiveSolarPower
        assert_eq!(plan.cause, CauseReason::ExcessiveSolarPower);
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
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder if plan.intensity == Intensity::Medium => {}, // Expected: Medium because user is home
            _ => panic!("Expected Colder with Medium intensity, got {:?}", plan.mode),
        }
        // Medium intensity = user is home = Undefined cause
        assert_eq!(plan.cause, CauseReason::Undefined);
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
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder if plan.intensity == Intensity::High => {}, // Expected: High because high solar
            _ => panic!("Expected Colder with High intensity, got {:?}", plan.mode),
        }
        // High intensity, no significant temp change = ExcessiveSolarPower
        assert_eq!(plan.cause, CauseReason::ExcessiveSolarPower);
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
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::NoChange => {}, // Expected: No change
            _ => panic!("Expected NoChange, got {:?}", plan.mode),
        }
        // Medium intensity = user is home = Undefined cause
        assert_eq!(plan.cause, CauseReason::Undefined);
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
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Medium => {}, // Expected: Warm with Medium
            _ => panic!("Expected Warmer with Medium intensity, got {:?}", plan.mode),
        }
        // Medium intensity = user is home = Undefined cause
        assert_eq!(plan.cause, CauseReason::Undefined);
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
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder if plan.intensity == Intensity::Medium => {}, // Expected: Cool with Medium
            _ => panic!("Expected Colder with Medium intensity, got {:?}", plan.mode),
        }
        // Medium intensity = user is home = Undefined cause
        assert_eq!(plan.cause, CauseReason::Undefined);
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
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::NoChange => {}, // Expected: No change when user not home
            _ => panic!("Expected NoChange, got {:?}", plan.mode),
        }
        // Low intensity, outdoor temp (18) is at edge of comfortable range (18-26) = MildTemperature
        assert_eq!(plan.cause, CauseReason::MildTemperature);
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
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::High => {}, // Expected: High due to weather forecast
            _ => panic!("Expected Warmer with High intensity due to forecast, got {:?}", plan.mode),
        }
        // High intensity with significant temp change = MajorTemperatureChangePending
        assert_eq!(plan.cause, CauseReason::MajorTemperatureChangePending);
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
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder if plan.intensity == Intensity::High => {}, // Expected: High due to weather forecast
            _ => panic!("Expected Colder with High intensity due to forecast, got {:?}", plan.mode),
        }
        // High intensity with significant temp change = MajorTemperatureChangePending
        assert_eq!(plan.cause, CauseReason::MajorTemperatureChangePending);
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
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Low => {}, // Expected: Low because no solar
            _ => panic!("Expected Warmer with Low intensity, got {:?}", plan.mode),
        }
        // Low intensity, outdoor temp (15) not near comfortable range = NobodyHome
        assert_eq!(plan.cause, CauseReason::NobodyHome);
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
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::High => {}, // Expected: High due to forecast + moderate solar
            _ => panic!("Expected Warmer with High intensity, got {:?}", plan.mode),
        }
        // High intensity with significant temp change = MajorTemperatureChangePending
        assert_eq!(plan.cause, CauseReason::MajorTemperatureChangePending);
    }

    // Ice Exception tests
    #[test]
    fn test_ice_exception_triggers() {
        // Outdoor temp below 2°C, indoor temp above 12°C, low solar
        // Should trigger ice exception and turn AC off
        let input = PlanInput {
            current_indoor_temp: 18.0,
            solar_production: 500, // Low solar, below bypass threshold
            user_is_home: true,
            current_outdoor_temp: 1.0, // Below 2°C
            avg_next_12h_outdoor_temp: 1.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Off => {}, // Expected: Off due to ice exception
            _ => panic!("Expected Off due to ice exception, got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::IceException);
    }

    #[test]
    fn test_ice_exception_override_when_too_cold_indoors() {
        // Outdoor temp below 2°C, but indoor temp below 12°C
        // Should override ice exception and allow heating
        let input = PlanInput {
            current_indoor_temp: 10.0, // Below 12°C
            solar_production: 0,
            user_is_home: false,
            current_outdoor_temp: 1.0, // Below 2°C
            avg_next_12h_outdoor_temp: 1.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Low => {}, // Expected: Heating with low intensity
            _ => panic!("Expected Warmer with Low intensity, got {:?}", plan.mode),
        }
        // With new cause system: user not home and outdoor temp far from comfortable = NobodyHome
        assert_eq!(plan.cause, CauseReason::NobodyHome);
    }

    #[test]
    fn test_ice_exception_does_not_trigger_above_threshold() {
        // Outdoor temp above 2°C, should not trigger ice exception
        let input = PlanInput {
            current_indoor_temp: 18.0,
            solar_production: 0,
            user_is_home: false,
            current_outdoor_temp: 4.0, // Above 2°C
            avg_next_12h_outdoor_temp: 4.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::NoChange => {}, // Expected: NoChange but not due to ice exception
            _ => panic!("Expected NoChange, got {:?}", plan.mode),
        }
        // With new cause system: user not home and outdoor temp far from comfortable = NobodyHome
        assert_eq!(plan.cause, CauseReason::NobodyHome);
    }

    #[test]
    fn test_ice_exception_edge_case_at_boundary() {
        // Outdoor temp exactly at 2°C, indoor temp exactly at 12°C
        // At 2°C outdoor: condition is < 2°C, so NO ice exception
        // At 12°C indoor: condition is >= 12°C, so no override
        // Result: Normal planning applies, and 12°C is too cold -> heating
        let input = PlanInput {
            current_indoor_temp: 12.0,
            solar_production: 0,
            user_is_home: false,
            current_outdoor_temp: 2.0,
            avg_next_12h_outdoor_temp: 2.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        // At exactly 2°C outdoor, ice exception should NOT trigger (< 2°C, not <= 2°C)
        // Temperature is 12°C which is between TOO_COLD (18°C) and comfortable (20°C)
        // User not home, so no heating
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Low => {}, // Expected: Heating because too cold
            _ => panic!("Expected Warmer(Low), got {:?}", plan.mode),
        }
        // With new cause system: user not home and outdoor temp far from comfortable = NobodyHome
        assert_eq!(plan.cause, CauseReason::NobodyHome);
    }

    #[test]
    fn test_ice_exception_bypassed_by_high_solar() {
        // Ice exception should be bypassed when solar production is high (> 1000W)
        // Outdoor temp below 2°C, indoor temp above 12°C, high solar
        let input = PlanInput {
            current_indoor_temp: 20.0,
            solar_production: 3000, // High solar, above bypass threshold
            user_is_home: true,
            current_outdoor_temp: 1.0, // Below 2°C
            avg_next_12h_outdoor_temp: 1.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        // Should NOT be off, ice exception bypassed due to high solar
        match plan.mode {
            RequestMode::Off => panic!("Ice exception should be bypassed with high solar, but got Off"),
            _ => {}, // Expected: Any other mode is fine (normal planning)
        }
        // Should not have IceException cause
        assert_ne!(plan.cause, CauseReason::IceException);
    }

    // Tests for new CauseReason assignments
    #[test]
    fn test_cause_nobody_home() {
        // User not home, outdoor temp not near comfortable, low intensity
        let input = PlanInput {
            current_indoor_temp: 17.5, // Too cold
            solar_production: 0,
            user_is_home: false,
            current_outdoor_temp: 10.0, // Not near comfortable (18-26)
            avg_next_12h_outdoor_temp: 10.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Low => {},
            _ => panic!("Expected Warmer(Low), got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::NobodyHome);
    }

    #[test]
    fn test_cause_mild_temperature() {
        // User not home, outdoor temp near comfortable range, low intensity
        let input = PlanInput {
            current_indoor_temp: 17.5, // Too cold
            solar_production: 0,
            user_is_home: false,
            current_outdoor_temp: 21.0, // Near comfortable range, within 2°C buffer
            avg_next_12h_outdoor_temp: 21.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Low => {},
            _ => panic!("Expected Warmer(Low), got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::MildTemperature);
    }

    #[test]
    fn test_cause_excessive_solar_power() {
        // High solar production, no significant temp change
        let input = PlanInput {
            current_indoor_temp: 28.0, // Too hot
            solar_production: 2500, // Above threshold
            user_is_home: false,
            current_outdoor_temp: 30.0,
            avg_next_12h_outdoor_temp: 30.5, // Small change, not significant
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder if plan.intensity == Intensity::High => {},
            _ => panic!("Expected Colder(High), got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::ExcessiveSolarPower);
    }

    #[test]
    fn test_cause_major_temp_change_pending() {
        // High solar production AND significant temperature change
        let input = PlanInput {
            current_indoor_temp: 28.0, // Too hot
            solar_production: 2500, // Above threshold
            user_is_home: false,
            current_outdoor_temp: 30.0,
            avg_next_12h_outdoor_temp: 35.0, // +5°C = significant change
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder if plan.intensity == Intensity::High => {},
            _ => panic!("Expected Colder(High), got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::MajorTemperatureChangePending);
    }

    #[test]
    fn test_cause_undefined_for_medium_intensity() {
        // User is home, medium intensity should have Undefined cause
        let input = PlanInput {
            current_indoor_temp: 19.0, // Slightly cold
            solar_production: 500, // Below high threshold
            user_is_home: true,
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 15.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Medium => {},
            _ => panic!("Expected Warmer(Medium), got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::Undefined);
    }

    #[test]
    fn test_cause_mild_temperature_with_cooling() {
        // Test mild temperature cause with cooling mode
        let input = PlanInput {
            current_indoor_temp: 27.5, // Too hot
            solar_production: 0,
            user_is_home: false,
            current_outdoor_temp: 23.0, // Near comfortable range, within buffer
            avg_next_12h_outdoor_temp: 23.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder if plan.intensity == Intensity::Low => {},
            _ => panic!("Expected Colder(Low), got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::MildTemperature);
    }

    #[test]
    fn test_ice_exception_converts_to_off_state() {
        // Verify that IceException plan results in device being off
        let input = PlanInput {
            current_indoor_temp: 18.0,
            solar_production: 0,
            user_is_home: true,
            current_outdoor_temp: 1.0, // Below 2°C
            avg_next_12h_outdoor_temp: 1.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        
        // Verify the plan is Off with IceException cause
        assert_eq!(plan.mode, RequestMode::Off);
        assert_eq!(plan.cause, CauseReason::IceException);
        
        // Verify that this converts to an off state
        use super::super::ac_executor::plan_to_state;
        let state = plan_to_state(&plan.mode, &plan.intensity, "LivingRoom");
        assert!(!state.is_on, "Device should be off when IceException triggers");
    }

    #[test]
    fn test_ice_exception_just_below_threshold() {
        // Test with outdoor temp just below threshold
        let input = PlanInput {
            current_indoor_temp: 15.0,
            solar_production: 500, // Below bypass threshold
            user_is_home: true,
            current_outdoor_temp: 1.9, // Just below 2°C
            avg_next_12h_outdoor_temp: 1.5,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Off => {}, // Expected: Off due to ice exception
            _ => panic!("Expected Off due to ice exception at 1.9°C outdoor, got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::IceException);
    }

    #[test]
    fn test_ice_exception_solar_bypass_at_boundary() {
        // Test solar bypass at exactly 1000W - should NOT bypass (needs > 1000W, i.e., < 1000W triggers exception)
        let input = PlanInput {
            current_indoor_temp: 18.0,
            solar_production: 1000, // Exactly at threshold
            user_is_home: true,
            current_outdoor_temp: 1.0, // Below 2°C
            avg_next_12h_outdoor_temp: 1.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        // At exactly 1000W, the condition is `solar_production < 1000`, which is false
        // So ice exception is bypassed and normal planning applies
        match plan.mode {
            RequestMode::Off => panic!("Ice exception should be bypassed at exactly 1000W solar, got Off"),
            _ => {}, // Expected: Normal planning applies
        }
        assert_ne!(plan.cause, CauseReason::IceException);
    }

    #[test]
    fn test_ice_exception_solar_bypass_just_below_threshold() {
        // Test with solar just below 1000W - should trigger ice exception
        let input = PlanInput {
            current_indoor_temp: 18.0,
            solar_production: 999, // Just below threshold
            user_is_home: true,
            current_outdoor_temp: 1.0, // Below 2°C
            avg_next_12h_outdoor_temp: 1.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        // Should be off, ice exception not bypassed
        match plan.mode {
            RequestMode::Off => {}, // Expected: Off due to ice exception
            _ => panic!("Expected Off at 999W solar, got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::IceException);
    }

    #[test]
    fn test_ice_exception_solar_bypass_does_not_affect_indoor_override() {
        // Solar bypass should not matter when indoor temp is too cold (< 12°C)
        // Indoor override takes precedence
        let input = PlanInput {
            current_indoor_temp: 10.0, // Below 12°C, should allow heating regardless
            solar_production: 500, // Low solar
            user_is_home: false,
            current_outdoor_temp: 1.0, // Below 2°C
            avg_next_12h_outdoor_temp: 1.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        // Should allow heating because indoor is too cold
        match plan.mode {
            RequestMode::Warmer => {}, // Expected: Heating allowed
            _ => panic!("Expected Warmer mode when indoor < 12°C, got {:?}", plan.mode),
        }
        // Should not have IceException cause
        assert_ne!(plan.cause, CauseReason::IceException);
    }

    // Tests for new solar-based intensity rules when user not home
    #[test]
    fn test_high_solar_allows_high_intensity_when_user_not_home() {
        // High solar (2000W+) should allow high intensity even when user not home
        let input = PlanInput {
            current_indoor_temp: 17.0, // Too cold
            solar_production: 2500, // High solar
            user_is_home: false,
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 15.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::High => {},
            _ => panic!("Expected Warmer(High) with high solar and user not home, got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::ExcessiveSolarPower);
    }

    #[test]
    fn test_medium_solar_allows_medium_intensity_when_user_not_home() {
        // Medium solar (1000W+) should allow medium intensity even when user not home
        let input = PlanInput {
            current_indoor_temp: 17.0, // Too cold
            solar_production: 1500, // Medium solar, between 1000W and 2000W
            user_is_home: false,
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 15.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Medium => {},
            _ => panic!("Expected Warmer(Medium) with medium solar and user not home, got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::ExcessiveSolarPower);
    }

    #[test]
    fn test_medium_solar_at_threshold_allows_medium_intensity_when_user_not_home() {
        // Exactly 1000W solar should allow medium intensity when user not home
        let input = PlanInput {
            current_indoor_temp: 28.0, // Too hot
            solar_production: 1000, // Exactly at threshold
            user_is_home: false,
            current_outdoor_temp: 30.0,
            avg_next_12h_outdoor_temp: 30.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder if plan.intensity == Intensity::Medium => {},
            _ => panic!("Expected Colder(Medium) with 1000W solar and user not home, got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::ExcessiveSolarPower);
    }

    #[test]
    fn test_low_solar_keeps_low_intensity_when_user_not_home() {
        // Low solar (< 1000W) should remain low intensity when user not home
        let input = PlanInput {
            current_indoor_temp: 17.0, // Too cold
            solar_production: 900, // Below medium threshold
            user_is_home: false,
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 15.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Low => {},
            _ => panic!("Expected Warmer(Low) with low solar and user not home, got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::NobodyHome);
    }

    // Tests for temperature trend validation
    #[test]
    fn test_high_intensity_heating_downgraded_when_outside_getting_hotter() {
        // High solar + heating needed, but outside is getting significantly warmer
        // Should downgrade from high to medium intensity
        let input = PlanInput {
            current_indoor_temp: 17.0, // Too cold
            solar_production: 2500, // High solar would normally give high intensity
            user_is_home: false,
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 22.0, // Getting 7°C warmer (> 3°C threshold)
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Medium => {},
            _ => panic!("Expected Warmer(Medium) due to counterproductive trend, got {:?}", plan.mode),
        }
        // Cause should still be solar-related since that's the underlying reason
        assert_eq!(plan.cause, CauseReason::ExcessiveSolarPower);
    }

    #[test]
    fn test_high_intensity_cooling_downgraded_when_outside_getting_colder() {
        // High solar + cooling needed, but outside is getting significantly colder
        // Should downgrade from high to medium intensity
        let input = PlanInput {
            current_indoor_temp: 28.0, // Too hot
            solar_production: 2500, // High solar would normally give high intensity
            user_is_home: false,
            current_outdoor_temp: 30.0,
            avg_next_12h_outdoor_temp: 22.0, // Getting 8°C colder (> 3°C threshold)
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder if plan.intensity == Intensity::Medium => {},
            _ => panic!("Expected Colder(Medium) due to counterproductive trend, got {:?}", plan.mode),
        }
        // Cause should still be solar-related
        assert_eq!(plan.cause, CauseReason::ExcessiveSolarPower);
    }

    #[test]
    fn test_high_intensity_heating_allowed_when_outside_getting_colder() {
        // High solar + heating needed, and outside is getting colder
        // Should keep high intensity (trend supports heating)
        let input = PlanInput {
            current_indoor_temp: 17.0, // Too cold
            solar_production: 2500, // High solar
            user_is_home: false,
            current_outdoor_temp: 20.0,
            avg_next_12h_outdoor_temp: 15.0, // Getting 5°C colder
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::High => {},
            _ => panic!("Expected Warmer(High) when trend supports heating, got {:?}", plan.mode),
        }
        // This should be MajorTemperatureChangePending since we have both high solar and significant temp change
        assert_eq!(plan.cause, CauseReason::MajorTemperatureChangePending);
    }

    #[test]
    fn test_high_intensity_cooling_allowed_when_outside_getting_warmer() {
        // High solar + cooling needed, and outside is getting warmer
        // Should keep high intensity (trend supports cooling)
        let input = PlanInput {
            current_indoor_temp: 28.0, // Too hot
            solar_production: 2500, // High solar
            user_is_home: false,
            current_outdoor_temp: 25.0,
            avg_next_12h_outdoor_temp: 32.0, // Getting 7°C warmer
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder if plan.intensity == Intensity::High => {},
            _ => panic!("Expected Colder(High) when trend supports cooling, got {:?}", plan.mode),
        }
        // This should be MajorTemperatureChangePending since we have both high solar and significant temp change
        assert_eq!(plan.cause, CauseReason::MajorTemperatureChangePending);
    }

    #[test]
    fn test_medium_intensity_not_affected_by_trend_validation() {
        // Medium intensity should not be affected by trend validation
        // (only high intensity gets downgraded)
        let input = PlanInput {
            current_indoor_temp: 17.0, // Too cold
            solar_production: 1500, // Medium solar
            user_is_home: false,
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 22.0, // Getting warmer
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Medium => {},
            _ => panic!("Expected Warmer(Medium), trend validation only applies to high intensity, got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::ExcessiveSolarPower);
    }

    #[test]
    fn test_combined_user_home_with_high_solar() {
        // When user is home AND high solar, should still use high intensity
        let input = PlanInput {
            current_indoor_temp: 17.0, // Too cold
            solar_production: 2500, // High solar
            user_is_home: true, // User is home
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 15.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::High => {},
            _ => panic!("Expected Warmer(High) with user home and high solar, got {:?}", plan.mode),
        }
        assert_eq!(plan.cause, CauseReason::ExcessiveSolarPower);
    }

    #[test]
    fn test_combined_user_home_with_medium_solar() {
        // When user is home AND medium solar (1000-2000W), solar takes priority for medium intensity
        let input = PlanInput {
            current_indoor_temp: 17.0, // Too cold
            solar_production: 1500, // Medium solar
            user_is_home: true, // User is home
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 15.0,
            current_ac_mode: None,
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer if plan.intensity == Intensity::Medium => {},
            _ => panic!("Expected Warmer(Medium) with user home and medium solar, got {:?}", plan.mode),
        }
        // With medium solar, the cause should be ExcessiveSolarPower
        assert_eq!(plan.cause, CauseReason::ExcessiveSolarPower);
    }

    // Tests for temperature hysteresis
    #[test]
    fn test_hysteresis_heating_continues_until_overshoot() {
        // When currently heating, should continue heating until target + overshoot
        // COMFORTABLE_TEMP_MIN (20) + HEATING_TURN_OFF_OVERSHOOT (1) = 21
        let input = PlanInput {
            current_indoor_temp: 20.5, // Above COMFORTABLE_TEMP_MIN but below overshoot
            solar_production: 500,
            user_is_home: true,
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 15.0,
            current_ac_mode: Some(true), // Currently heating
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer => {}, // Should continue heating
            _ => panic!("Expected Warmer to continue heating below overshoot point, got {:?}", plan.mode),
        }
    }

    #[test]
    fn test_hysteresis_heating_stops_at_overshoot() {
        // When currently heating and temperature reaches target + overshoot, should stop
        // COMFORTABLE_TEMP_MIN (20) + HEATING_TURN_OFF_OVERSHOOT (1) = 21
        let input = PlanInput {
            current_indoor_temp: 21.5, // Above COMFORTABLE_TEMP_MIN + overshoot
            solar_production: 500,
            user_is_home: true,
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 15.0,
            current_ac_mode: Some(true), // Currently heating
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::NoChange => {}, // Should stop heating
            _ => panic!("Expected NoChange when temperature exceeds overshoot point, got {:?}", plan.mode),
        }
    }

    #[test]
    fn test_hysteresis_cooling_continues_until_overshoot() {
        // When currently cooling, should continue cooling until target - overshoot
        // COMFORTABLE_TEMP_MAX (24) - COOLING_TURN_OFF_OVERSHOOT (1) = 23
        let input = PlanInput {
            current_indoor_temp: 23.5, // Below COMFORTABLE_TEMP_MAX but above overshoot point
            solar_production: 500,
            user_is_home: true,
            current_outdoor_temp: 30.0,
            avg_next_12h_outdoor_temp: 30.0,
            current_ac_mode: Some(false), // Currently cooling
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Colder => {}, // Should continue cooling
            _ => panic!("Expected Colder to continue cooling above overshoot point, got {:?}", plan.mode),
        }
    }

    #[test]
    fn test_hysteresis_cooling_stops_at_overshoot() {
        // When currently cooling and temperature drops below target - overshoot, should stop
        // COMFORTABLE_TEMP_MAX (24) - COOLING_TURN_OFF_OVERSHOOT (1) = 23
        let input = PlanInput {
            current_indoor_temp: 22.5, // Below COMFORTABLE_TEMP_MAX - overshoot
            solar_production: 500,
            user_is_home: true,
            current_outdoor_temp: 30.0,
            avg_next_12h_outdoor_temp: 30.0,
            current_ac_mode: Some(false), // Currently cooling
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::NoChange => {}, // Should stop cooling
            _ => panic!("Expected NoChange when temperature below overshoot point, got {:?}", plan.mode),
        }
    }

    #[test]
    fn test_no_hysteresis_when_not_running() {
        // When AC is off, should use normal thresholds (no hysteresis)
        let input = PlanInput {
            current_indoor_temp: 19.5, // Below COMFORTABLE_TEMP_MIN
            solar_production: 500,
            user_is_home: true,
            current_outdoor_temp: 15.0,
            avg_next_12h_outdoor_temp: 15.0,
            current_ac_mode: None, // AC is off
        };
        let plan = get_plan(&input);
        match plan.mode {
            RequestMode::Warmer => {}, // Should start heating
            _ => panic!("Expected Warmer to start heating when AC off and temp below min, got {:?}", plan.mode),
        }
    }
}
