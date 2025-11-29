use axum::{
    Json, Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Serialize, Deserialize};

use crate::{
    ac_controller::{
        AcDevices, RequestMode, Intensity, PlanInput, get_plan,
        ac_executor::{plan_to_state, AcState, AC_MODE_HEAT, AC_MODE_COOL},
    },
    config,
    device_requests,
    types::{ApiResponse, CauseReason},
};

const KW_TO_W_MULTIPLIER: f64 = 1000.0;

pub fn simulator_routes() -> Router {
    Router::new()
        .route("/evaluate", post(evaluate_workflow))
        .route("/live-inputs", get(get_live_inputs))
}

/// Input parameters for the simulator
#[derive(Debug, Clone, Deserialize)]
pub struct SimulatorInputs {
    /// Device name (e.g., "LivingRoom", "Veranda")
    pub device: String,
    /// Current indoor temperature
    pub temperature: f64,
    /// Whether the device is in auto mode
    pub is_auto_mode: bool,
    /// Solar production in watts (optional, fetched if not provided)
    pub solar_production: Option<u32>,
    /// Outdoor temperature (optional, fetched if not provided)
    pub outdoor_temp: Option<f64>,
    /// Average outdoor temperature in next 12 hours (optional, fetched if not provided)
    pub avg_next_12h_outdoor_temp: Option<f64>,
    /// Whether user is home (optional, calculated if not provided)
    pub user_is_home: Option<bool>,
    /// PIR detection status for this device (optional, defaults to false)
    pub pir_detected: Option<bool>,
    /// PIR detection minutes ago (optional, used if pir_detected is true)
    pub pir_minutes_ago: Option<u32>,
    /// Minutes since last AC command (optional, defaults to 60)
    pub last_change_minutes: Option<i32>,
}

/// Result of simulating a workflow
#[derive(Debug, Clone, Serialize)]
pub struct SimulatorResult {
    /// Whether the simulation was successful
    pub success: bool,
    /// The plan result (mode, intensity, cause)
    pub plan: Option<SimulatorPlanResult>,
    /// The AC state that would be set
    pub ac_state: Option<SimulatorAcState>,
    /// Error message if simulation failed
    pub error: Option<String>,
    /// Input values used for the simulation (including fetched defaults)
    pub inputs_used: SimulatorInputsUsed,
}

/// The plan result from simulation
#[derive(Debug, Clone, Serialize)]
pub struct SimulatorPlanResult {
    /// The request mode (Colder, Warmer, Off, NoChange)
    pub mode: String,
    /// The intensity (Low, Medium, High)
    pub intensity: String,
    /// The cause reason label
    pub cause_label: String,
    /// The cause reason description
    pub cause_description: String,
}

/// The AC state that would be set
#[derive(Debug, Clone, Serialize)]
pub struct SimulatorAcState {
    /// Whether the AC would be on
    pub is_on: bool,
    /// AC mode description (Heat/Cool/Off)
    pub mode: Option<String>,
    /// Fan speed (0 = auto, 1-5 = manual)
    pub fan_speed: Option<i32>,
    /// Target temperature in Celsius
    pub temperature: Option<f64>,
    /// Swing setting (0 = off, 1 = on)
    pub swing: Option<i32>,
    /// Whether powerful mode would be active
    pub powerful_mode: bool,
}

/// Input values used for the simulation (including fetched defaults)
#[derive(Debug, Clone, Serialize)]
pub struct SimulatorInputsUsed {
    pub device: String,
    pub temperature: f64,
    pub is_auto_mode: bool,
    pub solar_production: u32,
    pub outdoor_temp: f64,
    pub avg_next_12h_outdoor_temp: f64,
    pub user_is_home: bool,
    pub pir_detected: bool,
    pub last_change_minutes: i32,
}

impl SimulatorInputsUsed {
    /// Create SimulatorInputsUsed from SimulatorInputs with default values for optional fields
    fn from_inputs_with_defaults(inputs: &SimulatorInputs) -> Self {
        Self {
            device: inputs.device.clone(),
            temperature: inputs.temperature,
            is_auto_mode: inputs.is_auto_mode,
            solar_production: inputs.solar_production.unwrap_or(0),
            outdoor_temp: inputs.outdoor_temp.unwrap_or(20.0),
            avg_next_12h_outdoor_temp: inputs.avg_next_12h_outdoor_temp.unwrap_or(20.0),
            user_is_home: inputs.user_is_home.unwrap_or(false),
            pir_detected: inputs.pir_detected.unwrap_or(false),
            last_change_minutes: inputs.last_change_minutes.unwrap_or(60),
        }
    }
}

/// Live inputs from the current environment
#[derive(Debug, Clone, Serialize)]
pub struct LiveInputs {
    /// All configured devices
    pub devices: Vec<LiveDeviceInput>,
    /// Current solar production in watts (raw solar)
    pub solar_production: Option<u32>,
    /// Current outdoor temperature
    pub outdoor_temp: Option<f64>,
    /// Average outdoor temperature in next 12 hours
    pub avg_next_12h_outdoor_temp: Option<f64>,
    /// Whether user is home
    pub user_is_home: bool,
    /// Current net power in watts (positive = consuming, negative = exporting)
    pub net_power_watt: Option<i32>,
    /// Outside temperature trend (positive = warming, negative = cooling)
    pub outside_temperature_trend: Option<f64>,
}

/// Live inputs for a specific device
#[derive(Debug, Clone, Serialize)]
pub struct LiveDeviceInput {
    pub name: String,
    pub temperature: Option<f64>,
    pub is_auto_mode: bool,
    pub pir_recently_triggered: bool,
    pub pir_minutes_ago: Option<u32>,
    pub last_change_minutes: Option<i32>,
}

/// POST /api/simulator/evaluate
/// Evaluates the workflow with the provided inputs without executing any actions
async fn evaluate_workflow(Json(inputs): Json<SimulatorInputs>) -> Response {
    let cfg = config::get_config();
    
    // Validate device
    let _device = match AcDevices::from_str(&inputs.device) {
        Some(d) => d,
        None => {
            // Return a simulation result with an error - API call succeeded but simulation has invalid input
            let error_result = SimulatorResult {
                success: false,
                plan: None,
                ac_state: None,
                error: Some(format!("Unknown device: {}", inputs.device)),
                inputs_used: SimulatorInputsUsed::from_inputs_with_defaults(&inputs),
            };
            let response = ApiResponse::success(error_result);
            return (StatusCode::OK, Json(response)).into_response();
        }
    };
    
    // Check if device is in manual mode
    if !inputs.is_auto_mode {
        let result = SimulatorResult {
            success: true,
            plan: Some(SimulatorPlanResult {
                mode: "NoChange".to_string(),
                intensity: "Low".to_string(),
                cause_label: "Manual Mode".to_string(),
                cause_description: "Device is in manual mode - automatic control is disabled.".to_string(),
            }),
            ac_state: None,
            error: None,
            inputs_used: SimulatorInputsUsed::from_inputs_with_defaults(&inputs),
        };
        let response = ApiResponse::success(result);
        return (StatusCode::OK, Json(response)).into_response();
    }
    
    // Fetch missing input values
    let solar_production = match inputs.solar_production {
        Some(s) => s,
        None => get_solar_production().await.unwrap_or(0),
    };
    
    let outdoor_temp = match inputs.outdoor_temp {
        Some(t) => t,
        None => get_outdoor_temp().await.unwrap_or(20.0),
    };
    
    let avg_next_12h_outdoor_temp = match inputs.avg_next_12h_outdoor_temp {
        Some(t) => t,
        None => {
            // Calculate from outdoor temp and trend
            let trend = get_temperature_trend().await.unwrap_or(0.0);
            outdoor_temp + trend
        },
    };
    
    let user_is_home = inputs.user_is_home.unwrap_or_else(|| {
        crate::ac_controller::plan_helpers::is_user_home_and_awake()
    });
    
    let pir_detected = inputs.pir_detected.unwrap_or(false);
    let pir_minutes_ago = inputs.pir_minutes_ago.unwrap_or(0);
    let last_change_minutes = inputs.last_change_minutes.unwrap_or(60);
    
    // Build inputs used struct
    let inputs_used = SimulatorInputsUsed {
        device: inputs.device.clone(),
        temperature: inputs.temperature,
        is_auto_mode: inputs.is_auto_mode,
        solar_production,
        outdoor_temp,
        avg_next_12h_outdoor_temp,
        user_is_home,
        pir_detected,
        last_change_minutes,
    };
    
    // Check for PIR detection first
    if pir_detected && pir_minutes_ago < cfg.pir_timeout_minutes {
        let plan_result = SimulatorPlanResult {
            mode: "Off".to_string(),
            intensity: "Low".to_string(),
            cause_label: CauseReason::PirDetection.label().to_string(),
            cause_description: CauseReason::PirDetection.description().to_string(),
        };
        
        let ac_state = ac_state_to_simulator_state(&AcState::new_off());
        
        let result = SimulatorResult {
            success: true,
            plan: Some(plan_result),
            ac_state: Some(ac_state),
            error: None,
            inputs_used,
        };
        let response = ApiResponse::success(result);
        return (StatusCode::OK, Json(response)).into_response();
    }
    
    // Build the plan input
    let plan_input = PlanInput {
        current_indoor_temp: inputs.temperature,
        solar_production,
        user_is_home,
        current_outdoor_temp: outdoor_temp,
        avg_next_12h_outdoor_temp,
        current_ac_mode: None, // Simulation doesn't know current AC mode
    };
    
    // Get the plan using the pure function
    let plan = get_plan(&plan_input);
    
    // Convert plan to AC state
    let ac_state = plan_to_state(&plan.mode, &plan.intensity, &inputs.device);
    
    // Build result
    let plan_result = SimulatorPlanResult {
        mode: request_mode_to_string(&plan.mode),
        intensity: intensity_to_string(&plan.intensity),
        cause_label: plan.cause.label().to_string(),
        cause_description: plan.cause.description().to_string(),
    };
    
    let simulator_ac_state = ac_state_to_simulator_state(&ac_state);
    
    let result = SimulatorResult {
        success: true,
        plan: Some(plan_result),
        ac_state: Some(simulator_ac_state),
        error: None,
        inputs_used,
    };
    
    let response = ApiResponse::success(result);
    (StatusCode::OK, Json(response)).into_response()
}

/// GET /api/simulator/live-inputs
/// Returns live input values from the current environment
async fn get_live_inputs() -> Response {
    let cfg = config::get_config();
    let pir_state = crate::ac_controller::pir_state::get_pir_state();
    
    // Gather device data
    let mut devices = Vec::new();
    for device_name in cfg.ac_controller_endpoints.keys() {
        let (temp, is_auto) = match device_requests::ac::get_sensors_cached(device_name).await {
            Ok(sensor_data) => (Some(sensor_data.temperature), sensor_data.is_automatic_mode),
            Err(_) => (None, false),
        };
        
        let pir_recently_triggered = pir_state.has_recent_detection(device_name, cfg.pir_timeout_minutes);
        let pir_minutes_ago = pir_state.get_last_detection(device_name).map(|dt| {
            let now = chrono::Utc::now();
            let minutes = now.signed_duration_since(dt).num_minutes();
            // Convert to u32, clamping negative values to 0
            if minutes >= 0 { minutes as u32 } else { 0 }
        });
        
        // Get last change minutes from database
        let last_change_minutes = get_last_change_minutes_for_device(device_name).await;
        
        devices.push(LiveDeviceInput {
            name: device_name.clone(),
            temperature: temp,
            is_auto_mode: is_auto,
            pir_recently_triggered,
            pir_minutes_ago,
            last_change_minutes,
        });
    }
    
    // Get environmental data
    let solar_production = match device_requests::meter::get_solar_production_cached().await {
        Ok(production) => {
            // Convert i32 to u32, treating negative values as 0
            if production.current_production >= 0 {
                Some(production.current_production as u32)
            } else {
                Some(0)
            }
        },
        Err(_) => None,
    };
    
    let outdoor_temp = get_outdoor_temp().await.ok();
    
    // Get temperature trend
    let outside_temperature_trend = get_temperature_trend().await.ok();
    
    let avg_next_12h_outdoor_temp = match (outdoor_temp, outside_temperature_trend) {
        (Some(temp), Some(trend)) => Some(temp + trend),
        _ => None,
    };
    
    // Get net power from meter reading
    let net_power_watt = match device_requests::meter::get_latest_reading_cached().await {
        Ok(reading) => {
            // Calculate net power: negative means producing more than consuming
            let net = ((reading.current_consumption_kw - reading.current_production_kw) * KW_TO_W_MULTIPLIER) as i32;
            Some(net)
        },
        Err(_) => None,
    };
    
    let user_is_home = crate::ac_controller::plan_helpers::is_user_home_and_awake();
    
    let live_inputs = LiveInputs {
        devices,
        solar_production,
        outdoor_temp,
        avg_next_12h_outdoor_temp,
        user_is_home,
        net_power_watt,
        outside_temperature_trend,
    };
    
    let response = ApiResponse::success(live_inputs);
    (StatusCode::OK, Json(response)).into_response()
}

// Helper functions

async fn get_solar_production() -> Result<u32, ()> {
    match device_requests::meter::get_solar_production_cached().await {
        Ok(production) => {
            if production.current_production >= 0 {
                Ok(production.current_production as u32)
            } else {
                Ok(0)
            }
        },
        Err(_) => Err(()),
    }
}

async fn get_outdoor_temp() -> Result<f64, ()> {
    let cfg = config::get_config();
    device_requests::weather::get_current_outdoor_temp_cached(cfg.latitude, cfg.longitude)
        .await
        .map_err(|_| ())
}

async fn get_temperature_trend() -> Result<f64, ()> {
    let cfg = config::get_config();
    device_requests::weather::compute_temperature_trend_cached(cfg.latitude, cfg.longitude)
        .await
        .map_err(|_| ())
}

fn request_mode_to_string(mode: &RequestMode) -> String {
    match mode {
        RequestMode::Colder => "Colder".to_string(),
        RequestMode::Warmer => "Warmer".to_string(),
        RequestMode::Off => "Off".to_string(),
        RequestMode::NoChange => "NoChange".to_string(),
    }
}

fn intensity_to_string(intensity: &Intensity) -> String {
    match intensity {
        Intensity::Low => "Low".to_string(),
        Intensity::Medium => "Medium".to_string(),
        Intensity::High => "High".to_string(),
    }
}

fn ac_state_to_simulator_state(state: &AcState) -> SimulatorAcState {
    let mode = state.mode.map(|m| match m {
        AC_MODE_HEAT => "Heat".to_string(),
        AC_MODE_COOL => "Cool".to_string(),
        _ => format!("Unknown ({})", m),
    });
    
    SimulatorAcState {
        is_on: state.is_on,
        mode,
        fan_speed: state.fan_speed,
        temperature: state.temperature,
        swing: state.swing,
        powerful_mode: state.powerful_mode,
    }
}

/// Get minutes since the last AC command for a specific device
/// Returns i32::MAX if no actions have been recorded
async fn get_last_change_minutes_for_device(device_name: &str) -> Option<i32> {
    use crate::db;
    
    match db::ac_actions::get_last_action_timestamp(device_name).await {
        Ok(Some(timestamp)) => {
            let now = chrono::Utc::now().timestamp() as i32;
            let minutes_ago = (now - timestamp) / 60;
            // Clamp to valid i32 range (should always be positive)
            Some(minutes_ago.max(0))
        }
        Ok(None) => {
            // No actions recorded - return max i32
            Some(i32::MAX)
        }
        Err(_) => {
            // Database error - return None to indicate unavailable
            None
        }
    }
}
