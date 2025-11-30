use axum::{
    Json, Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::{
    ac_controller::{
        AcDevices,
        ac_executor::{AC_MODE_HEAT, AC_MODE_COOL, get_state_manager},
    },
    config,
    db,
    device_requests,
    nodes::{ExecutionInputs, NodesetExecutor, validate_nodeset_for_execution, ActiveCommandData},
    types::ApiResponse,
};

use super::nodes::{validate_nodeset, NodeConfiguration, get_active_nodeset_id, DEFAULT_NODESET_ID};

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
    /// Net power in watts (optional, positive = consuming, negative = exporting)
    pub net_power_watt: Option<i32>,
    /// Outside temperature trend (optional, positive = warming, negative = cooling)
    pub outside_temperature_trend: Option<f64>,
    /// Nodeset ID to evaluate (optional, uses active nodeset if not provided)
    /// Use -1 for new unsaved nodesets
    pub nodeset_id: Option<i64>,
    /// Nodes configuration for unsaved/new nodesets (when nodeset_id is -1)
    pub nodes: Option<Vec<serde_json::Value>>,
    /// Edges configuration for unsaved/new nodesets (when nodeset_id is -1)
    pub edges: Option<Vec<serde_json::Value>>,
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
    pub net_power_watt: i32,
    pub outside_temperature_trend: f64,
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
            net_power_watt: inputs.net_power_watt.unwrap_or(0),
            outside_temperature_trend: inputs.outside_temperature_trend.unwrap_or(0.0),
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
    let pool = db::get_pool().await;
    
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
    
    let temperature_trend = match inputs.outside_temperature_trend {
        Some(t) => t,
        None => get_temperature_trend().await.unwrap_or(0.0),
    };
    
    let avg_next_12h_outdoor_temp = match inputs.avg_next_12h_outdoor_temp {
        Some(t) => t,
        None => outdoor_temp + temperature_trend,
    };
    
    let user_is_home = inputs.user_is_home.unwrap_or_else(|| {
        crate::ac_controller::plan_helpers::is_user_home_and_awake()
    });
    
    let pir_detected = inputs.pir_detected.unwrap_or(false);
    let pir_minutes_ago = inputs.pir_minutes_ago.unwrap_or(0) as i64;
    let last_change_minutes = inputs.last_change_minutes.unwrap_or(60);
    
    let net_power_watt = match inputs.net_power_watt {
        Some(n) => n,
        None => {
            match device_requests::meter::get_latest_reading_cached().await {
                Ok(reading) => {
                    ((reading.current_consumption_kw - reading.current_production_kw) * KW_TO_W_MULTIPLIER) as i32
                }
                Err(_) => 0,
            }
        }
    };
    
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
        net_power_watt,
        outside_temperature_trend: temperature_trend,
    };
    
    // Get the nodeset to evaluate
    let (nodes, edges) = match get_nodeset_to_evaluate(&inputs, pool).await {
        Ok((n, e)) => (n, e),
        Err(error_msg) => {
            let error_result = SimulatorResult {
                success: false,
                plan: None,
                ac_state: None,
                error: Some(error_msg),
                inputs_used,
            };
            let response = ApiResponse::success(error_result);
            return (StatusCode::OK, Json(response)).into_response();
        }
    };
    
    // Validate the nodeset before execution
    let validation_errors = validate_nodeset_for_execution(&nodes, &edges);
    if !validation_errors.is_empty() {
        let error_result = SimulatorResult {
            success: false,
            plan: None,
            ac_state: None,
            error: Some(format!("Nodeset validation failed: {}", validation_errors.join("; "))),
            inputs_used,
        };
        let response = ApiResponse::success(error_result);
        return (StatusCode::OK, Json(response)).into_response();
    }
    
    // Also run the basic structural validation
    let structural_validation = validate_nodeset(&nodes);
    if !structural_validation.is_valid {
        let error_result = SimulatorResult {
            success: false,
            plan: None,
            ac_state: None,
            error: Some(format!("Profile structure invalid: {}", structural_validation.errors.join("; "))),
            inputs_used,
        };
        let response = ApiResponse::success(error_result);
        return (StatusCode::OK, Json(response)).into_response();
    }
    
    // Build PIR state for the execution context
    let mut pir_state = HashMap::new();
    pir_state.insert(inputs.device.clone(), (pir_detected, pir_minutes_ago));
    
    // Get active command from the AC state manager
    // The state manager tracks the last known state of each device.
    // A command is considered "defined" if we have any meaningful state tracked.
    // Note: The state manager's is_device_initialized() is private, but we can infer
    // initialization from whether mode or other optional fields have values.
    let state_manager = get_state_manager();
    let ac_state = state_manager.get_state(&inputs.device);
    
    // Determine if an active command exists:
    // - If device is currently on, we definitely have an active command
    // - If mode has a value, we've sent a command at some point
    // This aligns with how AcState tracks device state (mode is Some only after sending a command)
    let is_defined = ac_state.is_on || ac_state.mode.is_some();
    
    let active_command = ActiveCommandData {
        is_defined,
        is_on: ac_state.is_on,
        temperature: ac_state.temperature.unwrap_or(0.0),
        mode: ac_state.mode.unwrap_or(0),
        fan_speed: ac_state.fan_speed.unwrap_or(0),
        swing: ac_state.swing.unwrap_or(0),
        is_powerful: ac_state.powerful_mode,
    };
    
    // Build execution inputs
    let execution_inputs = ExecutionInputs {
        device: inputs.device.clone(),
        device_sensor_temperature: inputs.temperature,
        is_auto_mode: inputs.is_auto_mode,
        last_change_minutes: last_change_minutes as i64,
        outdoor_temperature: outdoor_temp,
        is_user_home: user_is_home,
        net_power_watt: net_power_watt as i64,
        raw_solar_watt: solar_production as i64,
        outside_temperature_trend: temperature_trend,
        pir_state,
        active_command,
    };
    
    // Create and execute the nodeset
    let mut executor = match NodesetExecutor::new(&nodes, &edges, execution_inputs) {
        Ok(e) => e,
        Err(e) => {
            let error_result = SimulatorResult {
                success: false,
                plan: None,
                ac_state: None,
                error: Some(format!("Failed to create executor: {}", e)),
                inputs_used,
            };
            let response = ApiResponse::success(error_result);
            return (StatusCode::OK, Json(response)).into_response();
        }
    };
    
    let execution_result = executor.execute();
    
    // Convert execution result to simulator result
    if let Some(error) = execution_result.error {
        let error_result = SimulatorResult {
            success: false,
            plan: None,
            ac_state: None,
            error: Some(error),
            inputs_used,
        };
        let response = ApiResponse::success(error_result);
        return (StatusCode::OK, Json(response)).into_response();
    }
    
    // Check terminal type and build appropriate response
    match execution_result.terminal_type.as_deref() {
        Some("Do Nothing") => {
            let result = SimulatorResult {
                success: true,
                plan: Some(SimulatorPlanResult {
                    mode: "NoChange".to_string(),
                    intensity: "Low".to_string(),
                    cause_label: "Node Workflow".to_string(),
                    cause_description: "Do Nothing node reached - no AC action will be taken.".to_string(),
                }),
                ac_state: None,
                error: None,
                inputs_used,
            };
            let response = ApiResponse::success(result);
            return (StatusCode::OK, Json(response)).into_response();
        }
        Some("Execute Action") => {
            if let Some(action) = execution_result.action {
                // Build the plan result from the action
                let plan_result = SimulatorPlanResult {
                    mode: action.mode.clone(),
                    intensity: if action.is_powerful { "High".to_string() } else { "Medium".to_string() },
                    cause_label: get_cause_reason_label(&action.cause_reason).await,
                    cause_description: format!("Action from nodeset: {} mode at {}°C", action.mode, action.temperature),
                };
                
                // Build AC state from action
                let ac_state = action_to_simulator_state(&action);
                
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
        }
        _ => {}
    }
    
    // No valid terminal reached
    let error_result = SimulatorResult {
        success: false,
        plan: None,
        ac_state: None,
        error: Some("Workflow did not reach a valid terminal node".to_string()),
        inputs_used,
    };
    let response = ApiResponse::success(error_result);
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

/// Get minutes since the last AC command for a specific device
/// Returns i32::MAX if no actions have been recorded
async fn get_last_change_minutes_for_device(device_name: &str) -> Option<i32> {
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

/// Get the nodeset to evaluate based on input parameters
/// If nodeset_id is provided and is -1, uses the nodes/edges from input
/// If nodeset_id is provided and >= 0, fetches from database
/// Otherwise uses the active nodeset
async fn get_nodeset_to_evaluate(
    inputs: &SimulatorInputs,
    pool: &sqlx::SqlitePool,
) -> Result<(Vec<serde_json::Value>, Vec<serde_json::Value>), String> {
    // Check if we should use the provided nodes/edges (for new/unsaved nodesets)
    if let Some(nodeset_id) = inputs.nodeset_id {
        if nodeset_id == -1 {
            // Use nodes/edges from input (new unsaved nodeset)
            let nodes = inputs.nodes.clone().unwrap_or_default();
            let edges = inputs.edges.clone().unwrap_or_default();
            return Ok((nodes, edges));
        }
        
        // Fetch specific nodeset from database
        let result = sqlx::query_as::<_, (String,)>(
            "SELECT node_json FROM nodesets WHERE id = ?"
        )
        .bind(nodeset_id)
        .fetch_optional(pool)
        .await;
        
        return match result {
            Ok(Some((node_json,))) => {
                match serde_json::from_str::<NodeConfiguration>(&node_json) {
                    Ok(config) => Ok((config.nodes, config.edges)),
                    Err(e) => Err(format!("Failed to parse nodeset configuration: {}", e)),
                }
            }
            Ok(None) => Err(format!("Nodeset with id {} not found", nodeset_id)),
            Err(e) => Err(format!("Failed to fetch nodeset: {}", e)),
        };
    }
    
    // Use active nodeset
    let active_id = match get_active_nodeset_id(pool).await {
        Ok(id) => id,
        Err(e) => return Err(format!("Failed to get active nodeset: {}", e)),
    };
    
    let result = sqlx::query_as::<_, (String,)>(
        "SELECT node_json FROM nodesets WHERE id = ?"
    )
    .bind(active_id)
    .fetch_optional(pool)
    .await;
    
    match result {
        Ok(Some((node_json,))) => {
            match serde_json::from_str::<NodeConfiguration>(&node_json) {
                Ok(config) => Ok((config.nodes, config.edges)),
                Err(e) => Err(format!("Failed to parse active nodeset configuration: {}", e)),
            }
        }
        Ok(None) => {
            if active_id == DEFAULT_NODESET_ID {
                // Default nodeset can be empty
                Ok((vec![], vec![]))
            } else {
                Err("Active nodeset not found".to_string())
            }
        }
        Err(e) => Err(format!("Failed to fetch active nodeset: {}", e)),
    }
}

/// Get the cause reason label from an ID
/// Looks up the cause reason in the database
async fn get_cause_reason_label(cause_id: &str) -> String {
    // Try to parse as i32 for database lookup
    if let Ok(id) = cause_id.parse::<i32>() {
        if let Ok(cause_reasons) = db::cause_reasons::get_all(false).await {
            if let Some(cr) = cause_reasons.iter().find(|cr| cr.id == id) {
                return cr.label.clone();
            }
        }
    }
    
    // Fallback to the raw ID
    cause_id.to_string()
}

/// Convert an ActionResult to a SimulatorAcState
fn action_to_simulator_state(action: &crate::nodes::ActionResult) -> SimulatorAcState {
    let mode_str = match action.mode.as_str() {
        "Heat" => Some("Heat".to_string()),
        "Cool" => Some("Cool".to_string()),
        "Off" => None,
        _ => Some(action.mode.clone()),
    };
    
    let is_on = action.mode != "Off";
    
    // Convert fan_speed string to i32 (0=Auto, 1=High, 2=Medium, 3=Low, 4=Quiet)
    let fan_speed = match action.fan_speed.as_str() {
        "Auto" => 0,
        "High" => 1,
        "Medium" => 2,
        "Low" => 3,
        "Quiet" => 4,
        _ => 0, // Default to Auto if unknown
    };
    
    SimulatorAcState {
        is_on,
        mode: mode_str,
        fan_speed: Some(fan_speed),
        temperature: Some(action.temperature),
        swing: Some(0), // Swing off
        powerful_mode: action.is_powerful,
    }
}
