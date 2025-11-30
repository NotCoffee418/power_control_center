//! Nodeset Execution Engine
//! 
//! This module provides the runtime execution of node graphs.
//! It takes a nodeset configuration (nodes and edges) and input values,
//! then evaluates the graph to produce an output result.
//!
//! Execution Flow:
//! The execution follows "Execution" type connections from the Start node.
//! When an If node is reached, it evaluates its condition and follows either
//! the True or False execution path. When a Sequence node is reached, it tries
//! each output in order until one path leads to a terminal node (Execute Action
//! or Do Nothing). Data connections (non-Execution types) are evaluated lazily
//! when needed by nodes along the execution path.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import AC mode constants from ac_executor
use crate::ac_controller::ac_executor::{AC_MODE_HEAT, AC_MODE_COOL};

/// Node type identifiers
pub const NODE_TYPE_START: &str = "flow_start";
pub const NODE_TYPE_EXECUTE_ACTION: &str = "flow_execute_action";
pub const NODE_TYPE_DO_NOTHING: &str = "flow_do_nothing";
pub const NODE_TYPE_ACTIVE_COMMAND: &str = "flow_active_command";
pub const NODE_TYPE_RESET_ACTIVE_COMMAND: &str = "flow_reset_active_command";
pub const NODE_TYPE_LOGIC_AND: &str = "logic_and";
pub const NODE_TYPE_LOGIC_OR: &str = "logic_or";
pub const NODE_TYPE_LOGIC_NAND: &str = "logic_nand";
pub const NODE_TYPE_LOGIC_IF: &str = "logic_if";
pub const NODE_TYPE_LOGIC_NOT: &str = "logic_not";
pub const NODE_TYPE_LOGIC_EQUALS: &str = "logic_equals";
pub const NODE_TYPE_LOGIC_EVALUATE_NUMBER: &str = "logic_evaluate_number";
pub const NODE_TYPE_LOGIC_BRANCH: &str = "logic_branch";
pub const NODE_TYPE_LOGIC_SEQUENCE: &str = "logic_sequence";
pub const NODE_TYPE_PRIMITIVE_FLOAT: &str = "primitive_float";
pub const NODE_TYPE_PRIMITIVE_INTEGER: &str = "primitive_integer";
pub const NODE_TYPE_PRIMITIVE_BOOLEAN: &str = "primitive_boolean";
pub const NODE_TYPE_DEVICE: &str = "device";
pub const NODE_TYPE_INTENSITY: &str = "intensity";
pub const NODE_TYPE_CAUSE_REASON: &str = "cause_reason";
pub const NODE_TYPE_REQUEST_MODE: &str = "request_mode";
pub const NODE_TYPE_FAN_SPEED: &str = "fan_speed";
pub const NODE_TYPE_PIR_DETECTION: &str = "pir_detection";

/// Sentinel value indicating no PIR detection has ever occurred
pub const PIR_NEVER_DETECTED: i64 = -1;

/// Tolerance for floating-point comparisons (suitable for temperature values in AC control)
const FLOAT_TOLERANCE: f64 = 0.0001;

/// A runtime value that can flow through nodes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuntimeValue {
    Float(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
    /// Represents an Active Command object (the last command sent to a device)
    ActiveCommand(ActiveCommandData),
}

/// Data for the Active Command - represents the last command sent to a device
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveCommandData {
    /// Whether an active command exists (a command was previously sent)
    pub is_defined: bool,
    /// Whether the AC is currently on (based on last command)
    pub is_on: bool,
    /// Target temperature in Celsius from the last command
    pub temperature: f64,
    /// AC mode: 1 = Heat, 4 = Cool, 0 = Off
    pub mode: i32,
    /// Fan speed setting (0-5, where 0 is auto)
    pub fan_speed: i32,
    /// Swing setting (0 = off, 1 = on)
    pub swing: i32,
    /// Whether powerful/turbo mode was enabled
    pub is_powerful: bool,
}

impl Default for ActiveCommandData {
    fn default() -> Self {
        Self {
            is_defined: false,
            is_on: false,
            temperature: 0.0,
            mode: 0,
            fan_speed: 0,
            swing: 0,
            is_powerful: false,
        }
    }
}

impl RuntimeValue {
    /// Get the type name for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            RuntimeValue::Float(_) => "Float",
            RuntimeValue::Integer(_) => "Integer",
            RuntimeValue::Boolean(_) => "Boolean",
            RuntimeValue::String(_) => "String",
            RuntimeValue::ActiveCommand(_) => "ActiveCommand",
        }
    }

    /// Try to convert to f64 (for numeric comparisons)
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            RuntimeValue::Float(v) => Some(*v),
            RuntimeValue::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Try to convert to bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            RuntimeValue::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to convert to string
    pub fn as_string(&self) -> String {
        match self {
            RuntimeValue::Float(v) => v.to_string(),
            RuntimeValue::Integer(v) => v.to_string(),
            RuntimeValue::Boolean(v) => v.to_string(),
            RuntimeValue::String(v) => v.clone(),
            RuntimeValue::ActiveCommand(_) => "ActiveCommand".to_string(),
        }
    }

    /// Try to get as ActiveCommandData
    pub fn as_active_command(&self) -> Option<&ActiveCommandData> {
        match self {
            RuntimeValue::ActiveCommand(data) => Some(data),
            _ => None,
        }
    }
}

/// Input values provided to the Start node from the simulation context
#[derive(Debug, Clone, Default)]
pub struct ExecutionInputs {
    pub device: String,
    pub device_sensor_temperature: f64,
    pub is_auto_mode: bool,
    pub last_change_minutes: i64,
    pub outdoor_temperature: f64,
    pub is_user_home: bool,
    pub net_power_watt: i64,
    pub raw_solar_watt: i64,
    /// Average outdoor temperature for the next 24 hours
    pub avg_next_24h_outdoor_temp: f64,
    /// PIR detection state by device: (is_recently_triggered, minutes_ago)
    pub pir_state: HashMap<String, (bool, i64)>,
    /// Active command data (last command sent to the device)
    pub active_command: ActiveCommandData,
}

/// Result of executing a nodeset
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResult {
    /// Whether the execution reached a terminal node
    pub completed: bool,
    /// The type of terminal node reached
    pub terminal_type: Option<String>,
    /// If Execute Action, the action parameters
    pub action: Option<ActionResult>,
    /// If Do Nothing, the do_nothing parameters (for debugging/simulation)
    pub do_nothing: Option<DoNothingResult>,
    /// Any error that occurred during execution
    pub error: Option<String>,
    /// Validation warnings (e.g., disconnected nodes)
    pub warnings: Vec<String>,
    /// Whether the active command should be reset to undefined state
    pub reset_active_command: bool,
}

/// Action parameters when Execute Action node is reached
#[derive(Debug, Clone, Serialize)]
pub struct ActionResult {
    pub device: String,
    pub temperature: f64,
    pub mode: String,
    pub fan_speed: String,
    pub is_powerful: bool,
    pub cause_reason: String,
}

/// Do Nothing parameters when Do Nothing node is reached (for debugging/simulation)
#[derive(Debug, Clone, Serialize)]
pub struct DoNothingResult {
    pub device: String,
    pub cause_reason: String,
}

/// Errors that can occur during nodeset execution
#[derive(Debug, Clone)]
pub enum ExecutionError {
    /// No Start node found
    MissingStartNode,
    /// Multiple Start nodes found
    MultipleStartNodes,
    /// No terminal node found
    MissingTerminalNode,
    /// Node not found
    NodeNotFound(String),
    /// Required input not connected
    MissingInput { node_id: String, input_id: String },
    /// Type mismatch
    TypeMismatch { expected: String, got: String },
    /// Invalid node configuration
    InvalidNode { node_id: String, reason: String },
    /// Cycle detected
    CycleDetected,
    /// Generic error
    Other(String),
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::MissingStartNode => write!(f, "No Start node found in the nodeset"),
            ExecutionError::MultipleStartNodes => write!(f, "Multiple Start nodes found (expected exactly one)"),
            ExecutionError::MissingTerminalNode => write!(f, "No path to a terminal node (Execute Action or Do Nothing)"),
            ExecutionError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            ExecutionError::MissingInput { node_id, input_id } => {
                write!(f, "Required input '{}' on node '{}' is not connected", input_id, node_id)
            }
            ExecutionError::TypeMismatch { expected, got } => {
                write!(f, "Type mismatch: expected {}, got {}", expected, got)
            }
            ExecutionError::InvalidNode { node_id, reason } => {
                write!(f, "Invalid node '{}': {}", node_id, reason)
            }
            ExecutionError::CycleDetected => write!(f, "Cycle detected in node connections"),
            ExecutionError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

/// Runtime representation of a node for execution
#[derive(Debug, Clone)]
struct RuntimeNode {
    id: String,
    node_type: String,
    data: serde_json::Value,
}

/// Runtime representation of an edge for execution
#[derive(Debug, Clone)]
struct RuntimeEdge {
    source: String,
    source_handle: String,
    target: String,
    target_handle: String,
}

/// The execution engine that evaluates nodesets
pub struct NodesetExecutor {
    nodes: HashMap<String, RuntimeNode>,
    edges: Vec<RuntimeEdge>,
    /// Cache of computed output values: (node_id, output_id) -> value
    output_cache: HashMap<(String, String), RuntimeValue>,
    /// Track nodes being evaluated to detect cycles
    evaluating: std::collections::HashSet<String>,
    /// Inputs from the simulation context
    inputs: ExecutionInputs,
    /// Flag to track if reset_active_command was triggered during execution
    reset_active_command_triggered: bool,
}

impl NodesetExecutor {
    /// Create a new executor from a nodeset configuration
    pub fn new(
        nodes: &[serde_json::Value],
        edges: &[serde_json::Value],
        inputs: ExecutionInputs,
    ) -> Result<Self, ExecutionError> {
        let mut node_map = HashMap::new();
        
        for node_json in nodes {
            let id = node_json.get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ExecutionError::Other("Node missing id".to_string()))?
                .to_string();
            
            let node_type = node_json
                .get("data")
                .and_then(|d| d.get("definition"))
                .and_then(|def| def.get("node_type"))
                .and_then(|nt| nt.as_str())
                .ok_or_else(|| ExecutionError::InvalidNode { 
                    node_id: id.clone(), 
                    reason: "Missing node_type in definition".to_string() 
                })?
                .to_string();
            
            node_map.insert(id.clone(), RuntimeNode {
                id,
                node_type,
                data: node_json.clone(),
            });
        }
        
        let mut edge_list = Vec::new();
        for edge_json in edges {
            let source = edge_json.get("source")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let source_handle = edge_json.get("sourceHandle")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let target = edge_json.get("target")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let target_handle = edge_json.get("targetHandle")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            
            if !source.is_empty() && !target.is_empty() {
                edge_list.push(RuntimeEdge {
                    source,
                    source_handle,
                    target,
                    target_handle,
                });
            }
        }
        
        Ok(Self {
            nodes: node_map,
            edges: edge_list,
            output_cache: HashMap::new(),
            evaluating: std::collections::HashSet::new(),
            inputs,
            reset_active_command_triggered: false,
        })
    }
    
    /// Execute the nodeset and return the result
    /// 
    /// The execution follows the execution flow pins from Start node:
    /// 1. Start node's exec_out connects to an If, Sequence, or terminal node
    /// 2. If nodes evaluate their condition and follow either True or False exec path
    /// 3. Sequence nodes try each output in order until a terminal is reached
    /// 4. When Execute Action or Do Nothing is reached, execution completes
    pub fn execute(&mut self) -> ExecutionResult {
        // Find the Start node
        let start_nodes: Vec<_> = self.nodes.values()
            .filter(|n| n.node_type == NODE_TYPE_START)
            .collect();
        
        if start_nodes.is_empty() {
            return ExecutionResult {
                completed: false,
                terminal_type: None,
                action: None,
                do_nothing: None,
                error: Some(ExecutionError::MissingStartNode.to_string()),
                warnings: vec![],
                reset_active_command: false,
            };
        }
        
        if start_nodes.len() > 1 {
            return ExecutionResult {
                completed: false,
                terminal_type: None,
                action: None,
                do_nothing: None,
                error: Some(ExecutionError::MultipleStartNodes.to_string()),
                warnings: vec![],
                reset_active_command: false,
            };
        }
        
        let start_node_id = start_nodes[0].id.clone();
        
        // Find all terminal nodes - collect IDs and types to avoid borrow issues
        let terminal_nodes: Vec<(String, String)> = self.nodes.values()
            .filter(|n| n.node_type == NODE_TYPE_EXECUTE_ACTION || n.node_type == NODE_TYPE_DO_NOTHING)
            .map(|n| (n.id.clone(), n.node_type.clone()))
            .collect();
        
        if terminal_nodes.is_empty() {
            return ExecutionResult {
                completed: false,
                terminal_type: None,
                action: None,
                do_nothing: None,
                error: Some(ExecutionError::MissingTerminalNode.to_string()),
                warnings: vec![],
                reset_active_command: false,
            };
        }
        
        // Populate the Start node outputs first
        if let Err(e) = self.populate_start_node_outputs(&start_node_id) {
            return ExecutionResult {
                completed: false,
                terminal_type: None,
                action: None,
                do_nothing: None,
                error: Some(e.to_string()),
                warnings: vec![],
                reset_active_command: false,
            };
        }
        
        // Follow execution flow from Start node's exec_out pin
        match self.follow_execution_flow(&start_node_id, "exec_out") {
            Ok(mut result) => {
                // Propagate the reset_active_command flag from the executor
                result.reset_active_command = self.reset_active_command_triggered;
                result
            }
            Err(e) => ExecutionResult {
                completed: false,
                terminal_type: None,
                action: None,
                do_nothing: None,
                error: Some(e.to_string()),
                warnings: vec![],
                reset_active_command: self.reset_active_command_triggered,
            },
        }
    }
    
    /// Follow the execution flow from a node's execution output pin
    /// Returns the result when a terminal node is reached
    fn follow_execution_flow(&mut self, source_node_id: &str, exec_output_id: &str) -> Result<ExecutionResult, ExecutionError> {
        // Find the edge connected to this execution output
        let edge = self.edges.iter()
            .find(|e| e.source == source_node_id && e.source_handle == exec_output_id)
            .cloned();
        
        match edge {
            Some(e) => {
                // Get the target node
                let target_node = self.nodes.get(&e.target)
                    .ok_or_else(|| ExecutionError::NodeNotFound(e.target.clone()))?
                    .clone();
                
                // Execute based on target node type
                match target_node.node_type.as_str() {
                    NODE_TYPE_EXECUTE_ACTION => {
                        // Terminal node - evaluate and return action
                        let action = self.evaluate_execute_action_node(&target_node.id)?;
                        Ok(ExecutionResult {
                            completed: true,
                            terminal_type: Some("Execute Action".to_string()),
                            action: Some(action),
                            do_nothing: None,
                            error: None,
                            warnings: vec![],
                            reset_active_command: false,
                        })
                    }
                    NODE_TYPE_DO_NOTHING => {
                        // Terminal node - evaluate and return do nothing
                        let do_nothing = self.evaluate_do_nothing_node(&target_node.id)?;
                        Ok(ExecutionResult {
                            completed: true,
                            terminal_type: Some("Do Nothing".to_string()),
                            action: None,
                            do_nothing: Some(do_nothing),
                            error: None,
                            warnings: vec![],
                            reset_active_command: false,
                        })
                    }
                    NODE_TYPE_RESET_ACTIVE_COMMAND => {
                        // Pass-through node - set reset flag and continue to next node
                        self.reset_active_command_triggered = true;
                        // Continue execution from this node's exec_out
                        self.follow_execution_flow(&target_node.id, "exec_out")
                    }
                    NODE_TYPE_LOGIC_IF => {
                        // If node - evaluate condition and follow appropriate path
                        self.execute_if_node(&target_node.id)
                    }
                    NODE_TYPE_LOGIC_SEQUENCE => {
                        // Sequence node - try each output in order
                        self.execute_sequence_node(&target_node.id)
                    }
                    _ => {
                        Err(ExecutionError::InvalidNode {
                            node_id: target_node.id.clone(),
                            reason: format!("Node type '{}' cannot receive execution flow", target_node.node_type),
                        })
                    }
                }
            }
            None => {
                // No execution connection - flow ends without reaching terminal
                Err(ExecutionError::Other(format!(
                    "Execution flow from '{}' output '{}' is not connected",
                    source_node_id, exec_output_id
                )))
            }
        }
    }
    
    /// Execute an If node - evaluate condition and follow appropriate execution path
    fn execute_if_node(&mut self, node_id: &str) -> Result<ExecutionResult, ExecutionError> {
        // Evaluate the condition input
        let condition = self.get_input_value(node_id, "condition")?;
        let is_true = match condition {
            RuntimeValue::Boolean(v) => v,
            _ => return Err(ExecutionError::TypeMismatch {
                expected: "Boolean".to_string(),
                got: condition.type_name().to_string(),
            }),
        };
        
        // Follow the appropriate execution output
        if is_true {
            self.follow_execution_flow(node_id, "exec_true")
        } else {
            self.follow_execution_flow(node_id, "exec_false")
        }
    }
    
    /// Sort "then_N" output IDs by their numeric suffix
    fn sort_then_outputs(outputs: &mut Vec<String>) {
        outputs.sort_by(|a, b| {
            let a_num: usize = a.strip_prefix("then_").and_then(|s| s.parse().ok()).unwrap_or(0);
            let b_num: usize = b.strip_prefix("then_").and_then(|s| s.parse().ok()).unwrap_or(0);
            a_num.cmp(&b_num)
        });
    }
    
    /// Get dynamic outputs from a node's data, if any
    fn get_dynamic_outputs_from_node(node: &RuntimeNode) -> Vec<String> {
        let mut outputs = Vec::new();
        if let Some(dynamic_outputs) = node.data.get("data")
            .and_then(|d| d.get("dynamicOutputs"))
            .and_then(|o| o.as_array()) 
        {
            for output in dynamic_outputs {
                if let Some(id) = output.get("id").and_then(|i| i.as_str()) {
                    if id.starts_with("then_") {
                        outputs.push(id.to_string());
                    }
                }
            }
        }
        outputs
    }
    
    /// Execute a Sequence node - try each output in order until one reaches a terminal
    /// 
    /// The sequence evaluates each "then_N" output in order (then_0, then_1, etc.).
    /// Execution stops when any path successfully reaches a terminal node (Execute Action or Do Nothing).
    /// If a path is not connected or leads to an error, it continues to the next output.
    fn execute_sequence_node(&mut self, node_id: &str) -> Result<ExecutionResult, ExecutionError> {
        // Get the node to check for dynamic outputs
        let node = self.nodes.get(node_id)
            .ok_or_else(|| ExecutionError::NodeNotFound(node_id.to_string()))?
            .clone();
        
        // Collect all "then_N" outputs from the node's edges
        let mut then_outputs: Vec<String> = self.edges.iter()
            .filter(|e| e.source == node_id && e.source_handle.starts_with("then_"))
            .map(|e| e.source_handle.clone())
            .collect();
        
        // Add any dynamic outputs defined in the node data
        for output_id in Self::get_dynamic_outputs_from_node(&node) {
            if !then_outputs.contains(&output_id) {
                then_outputs.push(output_id);
            }
        }
        
        // Sort by the numeric suffix to ensure correct order
        Self::sort_then_outputs(&mut then_outputs);
        
        // If no then outputs are found, that's an error
        if then_outputs.is_empty() {
            return Err(ExecutionError::Other(format!(
                "Sequence node '{}' has no connected outputs",
                node_id
            )));
        }
        
        // Try each output in order, collecting errors for debugging
        let mut last_error: Option<ExecutionError> = None;
        for output_id in &then_outputs {
            match self.follow_execution_flow(node_id, output_id) {
                Ok(result) => {
                    // Path reached a terminal - return the result
                    return Ok(result);
                }
                Err(e) => {
                    // This path didn't work, record the error and try the next one
                    last_error = Some(e);
                    continue;
                }
            }
        }
        
        // No path reached a terminal - include the last error for debugging
        let error_info = last_error
            .map(|e| format!(": last error was '{}'", e))
            .unwrap_or_default();
        Err(ExecutionError::Other(format!(
            "Sequence node '{}': no path reached a terminal node{}",
            node_id, error_info
        )))
    }
    
    /// Populate the Start node's output values from the execution inputs
    fn populate_start_node_outputs(&mut self, start_node_id: &str) -> Result<(), ExecutionError> {
        self.output_cache.insert(
            (start_node_id.to_string(), "device".to_string()),
            RuntimeValue::String(self.inputs.device.clone()),
        );
        self.output_cache.insert(
            (start_node_id.to_string(), "device_sensor_temperature".to_string()),
            RuntimeValue::Float(self.inputs.device_sensor_temperature),
        );
        self.output_cache.insert(
            (start_node_id.to_string(), "is_auto_mode".to_string()),
            RuntimeValue::Boolean(self.inputs.is_auto_mode),
        );
        self.output_cache.insert(
            (start_node_id.to_string(), "last_change_minutes".to_string()),
            RuntimeValue::Integer(self.inputs.last_change_minutes),
        );
        self.output_cache.insert(
            (start_node_id.to_string(), "outdoor_temperature".to_string()),
            RuntimeValue::Float(self.inputs.outdoor_temperature),
        );
        self.output_cache.insert(
            (start_node_id.to_string(), "is_user_home".to_string()),
            RuntimeValue::Boolean(self.inputs.is_user_home),
        );
        self.output_cache.insert(
            (start_node_id.to_string(), "net_power_watt".to_string()),
            RuntimeValue::Integer(self.inputs.net_power_watt),
        );
        self.output_cache.insert(
            (start_node_id.to_string(), "raw_solar_watt".to_string()),
            RuntimeValue::Integer(self.inputs.raw_solar_watt),
        );
        self.output_cache.insert(
            (start_node_id.to_string(), "avg_next_24h_outdoor_temp".to_string()),
            RuntimeValue::Float(self.inputs.avg_next_24h_outdoor_temp),
        );
        self.output_cache.insert(
            (start_node_id.to_string(), "active_command".to_string()),
            RuntimeValue::ActiveCommand(self.inputs.active_command.clone()),
        );
        
        Ok(())
    }
    
    /// Evaluate the Execute Action node and return the action parameters
    /// Device is inferred from the execution context (Start node)
    fn evaluate_execute_action_node(&mut self, node_id: &str) -> Result<ActionResult, ExecutionError> {
        // Device is inferred from execution context, not from node input
        let device = self.inputs.device.clone();
        let temperature = self.get_input_value(node_id, "temperature")?
            .as_f64()
            .ok_or_else(|| ExecutionError::TypeMismatch {
                expected: "Float".to_string(),
                got: "non-numeric".to_string(),
            })?;
        let mode = self.get_input_value(node_id, "mode")?
            .as_string();
        let fan_speed = self.get_input_value(node_id, "fan_speed")?
            .as_string();
        let is_powerful = self.get_input_value(node_id, "is_powerful")?
            .as_bool()
            .ok_or_else(|| ExecutionError::TypeMismatch {
                expected: "Boolean".to_string(),
                got: "non-boolean".to_string(),
            })?;
        let cause_reason = self.get_input_value(node_id, "cause_reason")?
            .as_string();
        
        Ok(ActionResult {
            device,
            temperature,
            mode,
            fan_speed,
            is_powerful,
            cause_reason,
        })
    }
    
    /// Evaluate the Do Nothing node and return the device and cause_reason for debugging
    /// Device is inferred from the execution context (Start node)
    /// Execution flow is handled by the flow engine, so we just need to get cause_reason
    fn evaluate_do_nothing_node(&mut self, node_id: &str) -> Result<DoNothingResult, ExecutionError> {
        // Device is inferred from execution context, not from node input
        let device = self.inputs.device.clone();
        let cause_reason = self.get_input_value(node_id, "cause_reason")?
            .as_string();
        
        Ok(DoNothingResult {
            device,
            cause_reason,
        })
    }
    
    /// Get the value for a node's input by finding the connected edge and evaluating the source
    fn get_input_value(&mut self, node_id: &str, input_id: &str) -> Result<RuntimeValue, ExecutionError> {
        // Find the edge that connects to this input
        let edge = self.edges.iter()
            .find(|e| e.target == node_id && e.target_handle == input_id)
            .cloned();
        
        match edge {
            Some(e) => {
                // Evaluate the source node's output
                self.evaluate_output(&e.source, &e.source_handle)
            }
            None => {
                Err(ExecutionError::MissingInput {
                    node_id: node_id.to_string(),
                    input_id: input_id.to_string(),
                })
            }
        }
    }
    
    /// Evaluate a specific output of a node
    fn evaluate_output(&mut self, node_id: &str, output_id: &str) -> Result<RuntimeValue, ExecutionError> {
        // Check cache first
        let cache_key = (node_id.to_string(), output_id.to_string());
        if let Some(value) = self.output_cache.get(&cache_key) {
            return Ok(value.clone());
        }
        
        // Check for cycles
        if self.evaluating.contains(node_id) {
            return Err(ExecutionError::CycleDetected);
        }
        self.evaluating.insert(node_id.to_string());
        
        let node = self.nodes.get(node_id)
            .ok_or_else(|| ExecutionError::NodeNotFound(node_id.to_string()))?
            .clone();
        
        let result = self.evaluate_node_output(&node, output_id);
        
        self.evaluating.remove(node_id);
        
        // Cache the result if successful
        if let Ok(ref value) = result {
            self.output_cache.insert(cache_key, value.clone());
        }
        
        result
    }
    
    /// Evaluate a node and return the requested output value
    fn evaluate_node_output(&mut self, node: &RuntimeNode, output_id: &str) -> Result<RuntimeValue, ExecutionError> {
        match node.node_type.as_str() {
            // Source nodes that provide values directly
            NODE_TYPE_START => {
                // Start node outputs should already be in cache
                self.output_cache
                    .get(&(node.id.clone(), output_id.to_string()))
                    .cloned()
                    .ok_or_else(|| ExecutionError::InvalidNode {
                        node_id: node.id.clone(),
                        reason: format!("Start node output '{}' not found", output_id),
                    })
            }
            
            // Primitive nodes
            NODE_TYPE_PRIMITIVE_FLOAT => {
                let value = node.data
                    .get("data")
                    .and_then(|d| d.get("primitiveValue"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                Ok(RuntimeValue::Float(value))
            }
            
            NODE_TYPE_PRIMITIVE_INTEGER => {
                let value = node.data
                    .get("data")
                    .and_then(|d| d.get("primitiveValue"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                Ok(RuntimeValue::Integer(value))
            }
            
            NODE_TYPE_PRIMITIVE_BOOLEAN => {
                let value = node.data
                    .get("data")
                    .and_then(|d| d.get("primitiveValue"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Ok(RuntimeValue::Boolean(value))
            }
            
            // Enum nodes
            NODE_TYPE_DEVICE | NODE_TYPE_INTENSITY | NODE_TYPE_CAUSE_REASON | NODE_TYPE_REQUEST_MODE | NODE_TYPE_FAN_SPEED => {
                let value = node.data
                    .get("data")
                    .and_then(|d| d.get("enumValue"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                Ok(RuntimeValue::String(value))
            }
            
            // Logic nodes
            NODE_TYPE_LOGIC_AND => {
                self.evaluate_logic_and(&node.id)
            }
            
            NODE_TYPE_LOGIC_OR => {
                self.evaluate_logic_or(&node.id)
            }
            
            NODE_TYPE_LOGIC_NAND => {
                let and_result = self.evaluate_logic_and(&node.id)?;
                match and_result {
                    RuntimeValue::Boolean(v) => Ok(RuntimeValue::Boolean(!v)),
                    _ => Err(ExecutionError::TypeMismatch {
                        expected: "Boolean".to_string(),
                        got: and_result.type_name().to_string(),
                    }),
                }
            }
            
            NODE_TYPE_LOGIC_NOT => {
                let input = self.get_input_value(&node.id, "input")?;
                match input {
                    RuntimeValue::Boolean(v) => Ok(RuntimeValue::Boolean(!v)),
                    _ => Err(ExecutionError::TypeMismatch {
                        expected: "Boolean".to_string(),
                        got: input.type_name().to_string(),
                    }),
                }
            }
            
            // If node now only has execution outputs, not data outputs
            // It's handled by follow_execution_flow instead
            NODE_TYPE_LOGIC_IF => {
                Err(ExecutionError::InvalidNode {
                    node_id: node.id.clone(),
                    reason: "If node only has execution outputs, not data outputs".to_string(),
                })
            }
            
            // Sequence node only has execution outputs
            NODE_TYPE_LOGIC_SEQUENCE => {
                Err(ExecutionError::InvalidNode {
                    node_id: node.id.clone(),
                    reason: "Sequence node only has execution outputs, not data outputs".to_string(),
                })
            }
            
            NODE_TYPE_LOGIC_EQUALS => {
                let a = self.get_input_value(&node.id, "input_a")?;
                let b = self.get_input_value(&node.id, "input_b")?;
                
                // Compare values - both must be same type and equal
                let is_equal = match (&a, &b) {
                    (RuntimeValue::Float(av), RuntimeValue::Float(bv)) => (av - bv).abs() < FLOAT_TOLERANCE,
                    (RuntimeValue::Integer(av), RuntimeValue::Integer(bv)) => av == bv,
                    (RuntimeValue::Boolean(av), RuntimeValue::Boolean(bv)) => av == bv,
                    (RuntimeValue::String(av), RuntimeValue::String(bv)) => av == bv,
                    // Allow comparing Integer and Float
                    (RuntimeValue::Float(av), RuntimeValue::Integer(bv)) => (av - (*bv as f64)).abs() < FLOAT_TOLERANCE,
                    (RuntimeValue::Integer(av), RuntimeValue::Float(bv)) => ((*av as f64) - bv).abs() < FLOAT_TOLERANCE,
                    _ => false,
                };
                
                Ok(RuntimeValue::Boolean(is_equal))
            }
            
            NODE_TYPE_LOGIC_EVALUATE_NUMBER => {
                let a = self.get_input_value(&node.id, "input_a")?;
                let b = self.get_input_value(&node.id, "input_b")?;
                
                let a_num = a.as_f64().ok_or_else(|| ExecutionError::TypeMismatch {
                    expected: "Numeric".to_string(),
                    got: a.type_name().to_string(),
                })?;
                let b_num = b.as_f64().ok_or_else(|| ExecutionError::TypeMismatch {
                    expected: "Numeric".to_string(),
                    got: b.type_name().to_string(),
                })?;
                
                // Get the operator from node data
                let operator = node.data
                    .get("data")
                    .and_then(|d| d.get("operatorValue"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(">");
                
                let result = match operator {
                    ">" => a_num > b_num,
                    ">=" => a_num >= b_num,
                    "==" => (a_num - b_num).abs() < FLOAT_TOLERANCE,
                    "<=" => a_num <= b_num,
                    "<" => a_num < b_num,
                    _ => a_num > b_num, // Default to >
                };
                
                Ok(RuntimeValue::Boolean(result))
            }
            
            NODE_TYPE_LOGIC_BRANCH => {
                let condition = self.get_input_value(&node.id, "condition")?;
                let is_true = match condition {
                    RuntimeValue::Boolean(v) => v,
                    _ => return Err(ExecutionError::TypeMismatch {
                        expected: "Boolean".to_string(),
                        got: condition.type_name().to_string(),
                    }),
                };
                
                // Return the appropriate value based on condition
                if is_true {
                    self.get_input_value(&node.id, "true_value")
                } else {
                    self.get_input_value(&node.id, "false_value")
                }
            }
            
            NODE_TYPE_PIR_DETECTION => {
                self.evaluate_pir_detection(&node.id, output_id)
            }
            
            NODE_TYPE_ACTIVE_COMMAND => {
                self.evaluate_active_command(&node.id, output_id)
            }
            
            _ => Err(ExecutionError::InvalidNode {
                node_id: node.id.clone(),
                reason: format!("Unknown node type: {}", node.node_type),
            }),
        }
    }
    
    /// Evaluate AND node with dynamic inputs
    fn evaluate_logic_and(&mut self, node_id: &str) -> Result<RuntimeValue, ExecutionError> {
        // Get all inputs connected to this node
        let connected_edges: Vec<_> = self.edges.iter()
            .filter(|e| e.target == node_id)
            .cloned()
            .collect();
        
        if connected_edges.is_empty() {
            return Err(ExecutionError::MissingInput {
                node_id: node_id.to_string(),
                input_id: "input_1".to_string(),
            });
        }
        
        // All inputs must be true
        for edge in connected_edges {
            let value = self.evaluate_output(&edge.source, &edge.source_handle)?;
            match value {
                RuntimeValue::Boolean(v) => {
                    if !v {
                        return Ok(RuntimeValue::Boolean(false));
                    }
                }
                _ => return Err(ExecutionError::TypeMismatch {
                    expected: "Boolean".to_string(),
                    got: value.type_name().to_string(),
                }),
            }
        }
        
        Ok(RuntimeValue::Boolean(true))
    }
    
    /// Evaluate OR node with dynamic inputs
    fn evaluate_logic_or(&mut self, node_id: &str) -> Result<RuntimeValue, ExecutionError> {
        // Get all inputs connected to this node
        let connected_edges: Vec<_> = self.edges.iter()
            .filter(|e| e.target == node_id)
            .cloned()
            .collect();
        
        if connected_edges.is_empty() {
            return Err(ExecutionError::MissingInput {
                node_id: node_id.to_string(),
                input_id: "input_1".to_string(),
            });
        }
        
        // Any input being true returns true
        for edge in connected_edges {
            let value = self.evaluate_output(&edge.source, &edge.source_handle)?;
            match value {
                RuntimeValue::Boolean(v) => {
                    if v {
                        return Ok(RuntimeValue::Boolean(true));
                    }
                }
                _ => return Err(ExecutionError::TypeMismatch {
                    expected: "Boolean".to_string(),
                    got: value.type_name().to_string(),
                }),
            }
        }
        
        Ok(RuntimeValue::Boolean(false))
    }
    
    /// Evaluate PIR Detection node
    fn evaluate_pir_detection(&mut self, node_id: &str, output_id: &str) -> Result<RuntimeValue, ExecutionError> {
        // Get the device input
        let device = self.get_input_value(node_id, "device")?.as_string();
        let timeout_minutes = self.get_input_value(node_id, "timeout_minutes")?;
        let timeout = match timeout_minutes {
            RuntimeValue::Integer(v) => v,
            _ => return Err(ExecutionError::TypeMismatch {
                expected: "Integer".to_string(),
                got: timeout_minutes.type_name().to_string(),
            }),
        };
        
        // Look up PIR state for this device
        // PIR_NEVER_DETECTED (-1) indicates no detection has ever occurred
        let (is_triggered, minutes_ago) = self.inputs.pir_state
            .get(&device)
            .copied()
            .unwrap_or((false, PIR_NEVER_DETECTED));
        
        match output_id {
            "is_recently_triggered" => {
                let is_recent = is_triggered && minutes_ago >= 0 && minutes_ago < timeout;
                Ok(RuntimeValue::Boolean(is_recent))
            }
            "last_detection_minutes_ago" => {
                Ok(RuntimeValue::Integer(minutes_ago))
            }
            _ => Err(ExecutionError::InvalidNode {
                node_id: node_id.to_string(),
                reason: format!("Unknown output: {}", output_id),
            }),
        }
    }
    
    /// Evaluate Active Command node
    /// Extracts properties from the active command input
    fn evaluate_active_command(&mut self, node_id: &str, output_id: &str) -> Result<RuntimeValue, ExecutionError> {
        // Get the active_command input
        let active_command_input = self.get_input_value(node_id, "active_command")?;
        let active_command = match active_command_input {
            RuntimeValue::ActiveCommand(data) => data,
            _ => return Err(ExecutionError::TypeMismatch {
                expected: "ActiveCommand".to_string(),
                got: active_command_input.type_name().to_string(),
            }),
        };
        
        match output_id {
            "is_defined" => Ok(RuntimeValue::Boolean(active_command.is_defined)),
            "is_on" => Ok(RuntimeValue::Boolean(active_command.is_on)),
            "temperature" => Ok(RuntimeValue::Float(active_command.temperature)),
            "mode" => {
                // Convert mode integer to string
                let mode_str = if !active_command.is_on {
                    "Off"
                } else {
                    match active_command.mode {
                        m if m == AC_MODE_HEAT => "Heat",
                        m if m == AC_MODE_COOL => "Cool",
                        m => {
                            log::warn!("Unknown AC mode value {} in active command, defaulting to 'Off'", m);
                            "Off"
                        }
                    }
                };
                Ok(RuntimeValue::String(mode_str.to_string()))
            }
            "fan_speed" => Ok(RuntimeValue::Integer(active_command.fan_speed as i64)),
            "swing" => Ok(RuntimeValue::Integer(active_command.swing as i64)),
            "is_powerful" => Ok(RuntimeValue::Boolean(active_command.is_powerful)),
            _ => Err(ExecutionError::InvalidNode {
                node_id: node_id.to_string(),
                reason: format!("Unknown output: {}", output_id),
            }),
        }
    }
}

/// Validate a nodeset configuration and return any errors
pub fn validate_nodeset_for_execution(
    nodes: &[serde_json::Value],
    edges: &[serde_json::Value],
) -> Vec<String> {
    let mut errors = Vec::new();
    
    // Check for Start node
    let start_nodes: Vec<_> = nodes.iter()
        .filter(|n| {
            n.get("data")
                .and_then(|d| d.get("definition"))
                .and_then(|def| def.get("node_type"))
                .and_then(|nt| nt.as_str())
                == Some(NODE_TYPE_START)
        })
        .collect();
    
    if start_nodes.is_empty() {
        errors.push("Missing Start node".to_string());
    } else if start_nodes.len() > 1 {
        errors.push(format!("Multiple Start nodes found (expected 1, found {})", start_nodes.len()));
    }
    
    // Check for terminal nodes
    let terminal_nodes: Vec<_> = nodes.iter()
        .filter(|n| {
            let node_type = n.get("data")
                .and_then(|d| d.get("definition"))
                .and_then(|def| def.get("node_type"))
                .and_then(|nt| nt.as_str());
            matches!(node_type, Some(NODE_TYPE_EXECUTE_ACTION) | Some(NODE_TYPE_DO_NOTHING))
        })
        .collect();
    
    if terminal_nodes.is_empty() {
        errors.push("Missing terminal node (Execute Action or Do Nothing)".to_string());
    }
    
    // Build a map of node IDs
    let node_ids: std::collections::HashSet<_> = nodes.iter()
        .filter_map(|n| n.get("id").and_then(|id| id.as_str()))
        .collect();
    
    // Check that all edges reference valid nodes
    for (i, edge) in edges.iter().enumerate() {
        let source = edge.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let target = edge.get("target").and_then(|v| v.as_str()).unwrap_or("");
        
        if !source.is_empty() && !node_ids.contains(source) {
            errors.push(format!("Edge {} references non-existent source node: {}", i, source));
        }
        if !target.is_empty() && !node_ids.contains(target) {
            errors.push(format!("Edge {} references non-existent target node: {}", i, target));
        }
    }
    
    // Check that if Active Command node exists, its is_defined output must be connected
    let active_command_nodes: Vec<_> = nodes.iter()
        .filter(|n| {
            n.get("data")
                .and_then(|d| d.get("definition"))
                .and_then(|def| def.get("node_type"))
                .and_then(|nt| nt.as_str())
                == Some(NODE_TYPE_ACTIVE_COMMAND)
        })
        .collect();
    
    for active_command_node in active_command_nodes {
        let node_id = active_command_node.get("id").and_then(|id| id.as_str()).unwrap_or("");
        
        // Check if is_defined output is connected
        let is_defined_connected = edges.iter().any(|edge| {
            let source = edge.get("source").and_then(|v| v.as_str()).unwrap_or("");
            let source_handle = edge.get("sourceHandle").and_then(|v| v.as_str()).unwrap_or("");
            source == node_id && source_handle == "is_defined"
        });
        
        if !is_defined_connected {
            errors.push("Active Command requires Is Defined pin to be handled".to_string());
        }
    }
    
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_start_node() -> serde_json::Value {
        json!({
            "id": "start-1",
            "type": "custom",
            "position": { "x": 0, "y": 0 },
            "data": {
                "definition": {
                    "node_type": "flow_start",
                    "name": "Start",
                    "description": "Entry point",
                    "category": "System",
                    "inputs": [],
                    "outputs": [
                        { "id": "exec_out", "label": "▶" },
                        { "id": "device", "label": "Device" },
                        { "id": "device_sensor_temperature", "label": "Device Sensor Temperature" }
                    ]
                }
            }
        })
    }

    fn create_execute_action_node() -> serde_json::Value {
        json!({
            "id": "execute-1",
            "type": "custom",
            "position": { "x": 400, "y": 0 },
            "data": {
                "definition": {
                    "node_type": "flow_execute_action",
                    "name": "Execute Action",
                    "description": "Executes AC command",
                    "category": "System",
                    "inputs": [
                        { "id": "exec_in", "label": "▶" },
                        { "id": "temperature", "label": "Temperature" },
                        { "id": "mode", "label": "Mode" },
                        { "id": "fan_speed", "label": "Fan Speed" },
                        { "id": "is_powerful", "label": "Is Powerful" },
                        { "id": "cause_reason", "label": "Cause Reason" }
                    ],
                    "outputs": []
                }
            }
        })
    }

    fn create_do_nothing_node() -> serde_json::Value {
        json!({
            "id": "do-nothing-1",
            "type": "custom",
            "position": { "x": 400, "y": 100 },
            "data": {
                "definition": {
                    "node_type": "flow_do_nothing",
                    "name": "Do Nothing",
                    "description": "Does nothing",
                    "category": "System",
                    "inputs": [
                        { "id": "exec_in", "label": "▶" },
                        { "id": "cause_reason", "label": "Cause Reason" }
                    ],
                    "outputs": []
                }
            }
        })
    }

    fn create_if_node(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "type": "custom",
            "position": { "x": 200, "y": 0 },
            "data": {
                "definition": {
                    "node_type": "logic_if",
                    "name": "If",
                    "description": "Routes execution",
                    "category": "Logic",
                    "inputs": [
                        { "id": "exec_in", "label": "▶" },
                        { "id": "condition", "label": "Condition" }
                    ],
                    "outputs": [
                        { "id": "exec_true", "label": "True ▶" },
                        { "id": "exec_false", "label": "False ▶" }
                    ]
                }
            }
        })
    }

    fn create_float_node(id: &str, value: f64) -> serde_json::Value {
        json!({
            "id": id,
            "type": "custom",
            "position": { "x": 200, "y": 0 },
            "data": {
                "primitiveValue": value,
                "definition": {
                    "node_type": "primitive_float",
                    "name": "Float",
                    "description": "Float value",
                    "category": "Primitives",
                    "inputs": [],
                    "outputs": [{ "id": "value", "label": "Value" }]
                }
            }
        })
    }

    fn create_boolean_node(id: &str, value: bool) -> serde_json::Value {
        json!({
            "id": id,
            "type": "custom",
            "position": { "x": 200, "y": 100 },
            "data": {
                "primitiveValue": value,
                "definition": {
                    "node_type": "primitive_boolean",
                    "name": "Boolean",
                    "description": "Boolean value",
                    "category": "Primitives",
                    "inputs": [],
                    "outputs": [{ "id": "value", "label": "Value" }]
                }
            }
        })
    }

    fn create_enum_node(id: &str, node_type: &str, value: &str) -> serde_json::Value {
        json!({
            "id": id,
            "type": "custom",
            "position": { "x": 200, "y": 200 },
            "data": {
                "enumValue": value,
                "definition": {
                    "node_type": node_type,
                    "name": "Enum",
                    "description": "Enum value",
                    "category": "Enums",
                    "inputs": [],
                    "outputs": [{ "id": "value", "label": "Value" }]
                }
            }
        })
    }

    fn create_edge(source: &str, source_handle: &str, target: &str, target_handle: &str) -> serde_json::Value {
        json!({
            "id": format!("e{}-{}", source, target),
            "source": source,
            "sourceHandle": source_handle,
            "target": target,
            "targetHandle": target_handle
        })
    }

    fn create_do_nothing_node_with_id(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "type": "custom",
            "position": { "x": 500, "y": 0 },
            "data": {
                "definition": {
                    "node_type": "flow_do_nothing",
                    "name": "Do Nothing",
                    "category": "System",
                    "inputs": [
                        { "id": "exec_in", "label": "▶" },
                        { "id": "cause_reason", "label": "Cause Reason" }
                    ],
                    "outputs": []
                }
            }
        })
    }

    #[test]
    fn test_simple_execution() {
        // Create a simple nodeset: Start -> Execute Action
        // With execution flow and data connections
        let nodes = vec![
            create_start_node(),
            create_float_node("float-1", 22.0),
            create_boolean_node("bool-1", false),
            create_enum_node("mode-1", "request_mode", "Heat"),
            create_enum_node("fan-speed-1", "fan_speed", "Auto"),
            create_enum_node("cause-1", "cause_reason", "1"),
            create_execute_action_node(),
        ];
        
        let edges = vec![
            // Execution flow: Start -> Execute Action
            create_edge("start-1", "exec_out", "execute-1", "exec_in"),
            // Data connections
            create_edge("float-1", "value", "execute-1", "temperature"),
            create_edge("mode-1", "value", "execute-1", "mode"),
            create_edge("fan-speed-1", "value", "execute-1", "fan_speed"),
            create_edge("bool-1", "value", "execute-1", "is_powerful"),
            create_edge("cause-1", "value", "execute-1", "cause_reason"),
        ];
        
        let inputs = ExecutionInputs {
            device: "LivingRoom".to_string(),
            device_sensor_temperature: 20.0,
            ..Default::default()
        };
        
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        assert!(result.completed);
        assert_eq!(result.terminal_type, Some("Execute Action".to_string()));
        assert!(result.action.is_some());
        
        let action = result.action.unwrap();
        assert_eq!(action.device, "LivingRoom");
        assert!((action.temperature - 22.0).abs() < f64::EPSILON);
        assert_eq!(action.mode, "Heat");
        assert_eq!(action.fan_speed, "Auto");
        assert!(!action.is_powerful);
    }

    #[test]
    fn test_missing_start_node() {
        let nodes = vec![
            create_execute_action_node(),
        ];
        let edges = vec![];
        
        let inputs = ExecutionInputs::default();
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        assert!(!result.completed);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Start node"));
    }

    #[test]
    fn test_missing_terminal_node() {
        let nodes = vec![
            create_start_node(),
        ];
        let edges = vec![];
        
        let inputs = ExecutionInputs::default();
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        assert!(!result.completed);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("terminal node"));
    }

    #[test]
    fn test_missing_execution_flow() {
        // Execute Action has data connections but no execution flow
        let nodes = vec![
            create_start_node(),
            create_float_node("float-1", 22.0),
            create_boolean_node("bool-1", false),
            create_enum_node("mode-1", "request_mode", "Heat"),
            create_enum_node("fan-speed-1", "fan_speed", "Auto"),
            create_enum_node("cause-1", "cause_reason", "1"),
            create_execute_action_node(),
        ];
        let edges = vec![
            // Data connections but no execution flow
            create_edge("float-1", "value", "execute-1", "temperature"),
            create_edge("mode-1", "value", "execute-1", "mode"),
            create_edge("fan-speed-1", "value", "execute-1", "fan_speed"),
            create_edge("bool-1", "value", "execute-1", "is_powerful"),
            create_edge("cause-1", "value", "execute-1", "cause_reason"),
        ];
        
        let inputs = ExecutionInputs::default();
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        assert!(!result.completed);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("not connected"));
    }

    #[test]
    fn test_validation_errors() {
        // Test with no nodes
        let errors = validate_nodeset_for_execution(&[], &[]);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("Start")));
        assert!(errors.iter().any(|e| e.contains("terminal")));
    }

    #[test]
    fn test_and_node_evaluation() {
        let nodes = vec![
            create_start_node(),
            create_boolean_node("bool-1", true),
            create_boolean_node("bool-2", true),
            json!({
                "id": "and-1",
                "type": "custom",
                "position": { "x": 300, "y": 0 },
                "data": {
                    "definition": {
                        "node_type": "logic_and",
                        "name": "AND",
                        "category": "Logic"
                    }
                }
            }),
            // If node to route execution based on AND result
            create_if_node("if-1"),
            create_do_nothing_node_with_id("do-nothing-1"),
            create_enum_node("cause-1", "cause_reason", "1"),
        ];
        
        let edges = vec![
            // Data flow: bool-1 AND bool-2 -> if condition
            create_edge("bool-1", "value", "and-1", "input_1"),
            create_edge("bool-2", "value", "and-1", "input_2"),
            create_edge("and-1", "result", "if-1", "condition"),
            // Execution flow: Start -> If -> Do Nothing (true path)
            create_edge("start-1", "exec_out", "if-1", "exec_in"),
            create_edge("if-1", "exec_true", "do-nothing-1", "exec_in"),
            // Data flow for Do Nothing
            create_edge("cause-1", "value", "do-nothing-1", "cause_reason"),
        ];
        
        let inputs = ExecutionInputs {
            device: "LivingRoom".to_string(),
            ..Default::default()
        };
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        assert!(result.completed);
        assert_eq!(result.terminal_type, Some("Do Nothing".to_string()));
        // Verify do_nothing result has the expected values
        assert!(result.do_nothing.is_some());
        let do_nothing = result.do_nothing.unwrap();
        assert_eq!(do_nothing.device, "LivingRoom");
        assert_eq!(do_nothing.cause_reason, "1");
    }

    #[test]
    fn test_branch_node_true_path() {
        let nodes = vec![
            create_start_node(),
            create_boolean_node("condition", true),
            create_float_node("true-val", 25.0),
            create_float_node("false-val", 15.0),
            json!({
                "id": "branch-1",
                "type": "custom",
                "position": { "x": 300, "y": 0 },
                "data": {
                    "definition": {
                        "node_type": "logic_branch",
                        "name": "Branch",
                        "category": "Logic"
                    }
                }
            }),
            create_enum_node("mode-1", "request_mode", "Heat"),
            create_enum_node("fan-speed-1", "fan_speed", "Medium"),
            create_enum_node("cause-1", "cause_reason", "1"),
            create_boolean_node("powerful", false),
            create_execute_action_node(),
        ];
        
        // Execution flow + data connections
        let edges = vec![
            // Execution flow: Start -> Execute Action
            create_edge("start-1", "exec_out", "execute-1", "exec_in"),
            // Data flow
            create_edge("condition", "value", "branch-1", "condition"),
            create_edge("true-val", "value", "branch-1", "true_value"),
            create_edge("false-val", "value", "branch-1", "false_value"),
            create_edge("branch-1", "result", "execute-1", "temperature"),
            create_edge("mode-1", "value", "execute-1", "mode"),
            create_edge("fan-speed-1", "value", "execute-1", "fan_speed"),
            create_edge("powerful", "value", "execute-1", "is_powerful"),
            create_edge("cause-1", "value", "execute-1", "cause_reason"),
        ];
        
        let inputs = ExecutionInputs {
            device: "LivingRoom".to_string(),
            ..Default::default()
        };
        
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        assert!(result.completed);
        let action = result.action.unwrap();
        // Should use true path value (25.0) since condition is true
        assert!((action.temperature - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_branch_node_false_path() {
        let nodes = vec![
            create_start_node(),
            create_boolean_node("condition", false), // Condition is false
            create_float_node("true-val", 25.0),
            create_float_node("false-val", 15.0),
            json!({
                "id": "branch-1",
                "type": "custom",
                "position": { "x": 300, "y": 0 },
                "data": {
                    "definition": {
                        "node_type": "logic_branch",
                        "name": "Branch",
                        "category": "Logic"
                    }
                }
            }),
            create_enum_node("mode-1", "request_mode", "Cool"),
            create_enum_node("fan-speed-1", "fan_speed", "High"),
            create_enum_node("cause-1", "cause_reason", "1"),
            create_boolean_node("powerful", false),
            create_execute_action_node(),
        ];
        
        // Execution flow + data connections
        let edges = vec![
            // Execution flow: Start -> Execute Action
            create_edge("start-1", "exec_out", "execute-1", "exec_in"),
            // Data flow
            create_edge("condition", "value", "branch-1", "condition"),
            create_edge("true-val", "value", "branch-1", "true_value"),
            create_edge("false-val", "value", "branch-1", "false_value"),
            create_edge("branch-1", "result", "execute-1", "temperature"),
            create_edge("mode-1", "value", "execute-1", "mode"),
            create_edge("fan-speed-1", "value", "execute-1", "fan_speed"),
            create_edge("powerful", "value", "execute-1", "is_powerful"),
            create_edge("cause-1", "value", "execute-1", "cause_reason"),
        ];
        
        let inputs = ExecutionInputs {
            device: "Veranda".to_string(),
            ..Default::default()
        };
        
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        assert!(result.completed);
        let action = result.action.unwrap();
        // Should use false path value (15.0) since condition is false
        assert!((action.temperature - 15.0).abs() < f64::EPSILON);
    }

    fn create_active_command_node(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "type": "custom",
            "position": { "x": 300, "y": 0 },
            "data": {
                "definition": {
                    "node_type": "flow_active_command",
                    "name": "Active Command",
                    "description": "Gets active command properties",
                    "category": "System",
                    "inputs": [
                        { "id": "active_command", "label": "Active Command" }
                    ],
                    "outputs": [
                        { "id": "is_defined", "label": "Is Defined" },
                        { "id": "is_on", "label": "Is On" },
                        { "id": "temperature", "label": "Temperature" },
                        { "id": "mode", "label": "Mode" },
                        { "id": "fan_speed", "label": "Fan Speed" },
                        { "id": "swing", "label": "Swing" },
                        { "id": "is_powerful", "label": "Is Powerful" }
                    ]
                }
            }
        })
    }

    #[test]
    fn test_active_command_validation_missing_is_defined() {
        // Active Command node without is_defined connected should fail validation
        let nodes = vec![
            create_start_node(),
            create_active_command_node("active-cmd-1"),
            create_execute_action_node(),
        ];
        
        // Only connect active_command input, not the is_defined output
        let edges = vec![
            create_edge("start-1", "active_command", "active-cmd-1", "active_command"),
            create_edge("active-cmd-1", "temperature", "execute-1", "temperature"),
        ];
        
        let errors = validate_nodeset_for_execution(&nodes, &edges);
        
        assert!(errors.iter().any(|e| e.contains("Active Command requires Is Defined pin to be handled")));
    }

    #[test]
    fn test_active_command_validation_with_is_defined() {
        // Active Command node with is_defined connected should not produce this error
        // We use an If node to route execution based on is_defined
        let nodes = vec![
            create_start_node(),
            create_active_command_node("active-cmd-1"),
            create_if_node("if-1"),
            create_do_nothing_node_with_id("do-nothing-1"),
            create_enum_node("cause-1", "cause_reason", "1"),
        ];
        
        // Connect is_defined to If node condition
        let edges = vec![
            create_edge("start-1", "active_command", "active-cmd-1", "active_command"),
            create_edge("active-cmd-1", "is_defined", "if-1", "condition"), // is_defined is connected (handled)
            create_edge("start-1", "exec_out", "if-1", "exec_in"),
            create_edge("if-1", "exec_true", "do-nothing-1", "exec_in"),
            create_edge("cause-1", "value", "do-nothing-1", "cause_reason"),
        ];
        
        let errors = validate_nodeset_for_execution(&nodes, &edges);
        
        // Should not contain the Active Command validation error
        assert!(!errors.iter().any(|e| e.contains("Active Command requires Is Defined pin to be handled")));
    }

    #[test]
    fn test_active_command_evaluation_defined() {
        // Test evaluation of Active Command node when command is defined
        // We use If node to route execution based on is_defined
        let nodes = vec![
            create_start_node(),
            create_active_command_node("active-cmd-1"),
            create_if_node("if-1"),
            create_do_nothing_node_with_id("do-nothing-1"),
            create_enum_node("cause-1", "cause_reason", "1"),
        ];
        
        // Execution flow with If node routing based on is_defined
        let edges = vec![
            create_edge("start-1", "active_command", "active-cmd-1", "active_command"),
            create_edge("active-cmd-1", "is_defined", "if-1", "condition"),
            // Execution flow: Start -> If -> Do Nothing (true path = is_defined)
            create_edge("start-1", "exec_out", "if-1", "exec_in"),
            create_edge("if-1", "exec_true", "do-nothing-1", "exec_in"),
            create_edge("cause-1", "value", "do-nothing-1", "cause_reason"),
        ];
        
        let inputs = ExecutionInputs {
            device: "LivingRoom".to_string(),
            active_command: ActiveCommandData {
                is_defined: true,
                is_on: true,
                temperature: 22.5,
                mode: 1, // Heat
                fan_speed: 2,
                swing: 1,
                is_powerful: false,
            },
            ..Default::default()
        };
        
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        assert!(result.completed);
        assert_eq!(result.terminal_type, Some("Do Nothing".to_string()));
    }

    #[test]
    fn test_active_command_evaluation_not_defined() {
        // Test evaluation of Active Command node when command is not defined
        // When is_defined is false, the If node should take the false path
        let nodes = vec![
            create_start_node(),
            create_active_command_node("active-cmd-1"),
            create_if_node("if-1"),
            create_do_nothing_node_with_id("do-nothing-1"),
            create_enum_node("cause-1", "cause_reason", "1"),
        ];
        
        // Execution flow with If node routing based on is_defined
        let edges = vec![
            create_edge("start-1", "active_command", "active-cmd-1", "active_command"),
            create_edge("active-cmd-1", "is_defined", "if-1", "condition"),
            // Execution flow: Start -> If -> Do Nothing (false path = !is_defined)
            create_edge("start-1", "exec_out", "if-1", "exec_in"),
            create_edge("if-1", "exec_false", "do-nothing-1", "exec_in"),
            create_edge("cause-1", "value", "do-nothing-1", "cause_reason"),
        ];
        
        // Default ActiveCommandData has is_defined = false
        let inputs = ExecutionInputs {
            device: "LivingRoom".to_string(),
            ..Default::default()
        };
        
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        assert!(result.completed);
        // Do Nothing node should be reached via false path since is_defined = false
        assert_eq!(result.terminal_type, Some("Do Nothing".to_string()));
    }

    fn create_reset_active_command_node(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "type": "custom",
            "position": { "x": 200, "y": 0 },
            "data": {
                "definition": {
                    "node_type": "flow_reset_active_command",
                    "name": "Reset Active Command",
                    "description": "Resets the active command to undefined state",
                    "category": "System",
                    "inputs": [
                        { "id": "exec_in", "label": "▶" }
                    ],
                    "outputs": [
                        { "id": "exec_out", "label": "▶" }
                    ]
                }
            }
        })
    }

    #[test]
    fn test_reset_active_command_node_execution() {
        // Test that Reset Active Command node passes execution through and sets the flag
        // Flow: Start -> Reset Active Command -> Do Nothing
        let nodes = vec![
            create_start_node(),
            create_reset_active_command_node("reset-1"),
            create_do_nothing_node_with_id("do-nothing-1"),
            create_enum_node("cause-1", "cause_reason", "1"),
        ];
        
        let edges = vec![
            // Execution flow: Start -> Reset Active Command -> Do Nothing
            create_edge("start-1", "exec_out", "reset-1", "exec_in"),
            create_edge("reset-1", "exec_out", "do-nothing-1", "exec_in"),
            create_edge("cause-1", "value", "do-nothing-1", "cause_reason"),
        ];
        
        let inputs = ExecutionInputs {
            device: "LivingRoom".to_string(),
            active_command: ActiveCommandData {
                is_defined: true,
                is_on: true,
                temperature: 22.5,
                mode: 1,
                fan_speed: 2,
                swing: 1,
                is_powerful: false,
            },
            ..Default::default()
        };
        
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        assert!(result.completed);
        assert_eq!(result.terminal_type, Some("Do Nothing".to_string()));
        // The reset_active_command flag should be set
        assert!(result.reset_active_command, "Reset Active Command flag should be set");
    }

    #[test]
    fn test_reset_active_command_flag_not_set_without_node() {
        // Test that when Reset Active Command node is not used, the flag is false
        // Flow: Start -> Do Nothing (no reset node)
        let nodes = vec![
            create_start_node(),
            create_do_nothing_node_with_id("do-nothing-1"),
            create_enum_node("cause-1", "cause_reason", "1"),
        ];
        
        let edges = vec![
            // Execution flow: Start -> Do Nothing (no reset node in between)
            create_edge("start-1", "exec_out", "do-nothing-1", "exec_in"),
            create_edge("cause-1", "value", "do-nothing-1", "cause_reason"),
        ];
        
        let inputs = ExecutionInputs {
            device: "LivingRoom".to_string(),
            ..Default::default()
        };
        
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        assert!(result.completed);
        assert_eq!(result.terminal_type, Some("Do Nothing".to_string()));
        // The reset_active_command flag should NOT be set
        assert!(!result.reset_active_command, "Reset Active Command flag should NOT be set when node is not used");
    }

    #[test]
    fn test_reset_active_command_node_exec_out_not_connected() {
        // Test that when Reset Active Command node's exec_out is not connected, we get an error
        // Flow: Start -> Reset Active Command (exec_out not connected)
        let nodes = vec![
            create_start_node(),
            create_reset_active_command_node("reset-1"),
            create_do_nothing_node_with_id("do-nothing-1"), // Present but not connected
            create_enum_node("cause-1", "cause_reason", "1"),
        ];
        
        let edges = vec![
            // Execution flow: Start -> Reset Active Command (but exec_out not connected)
            create_edge("start-1", "exec_out", "reset-1", "exec_in"),
            // Missing: create_edge("reset-1", "exec_out", "do-nothing-1", "exec_in"),
            create_edge("cause-1", "value", "do-nothing-1", "cause_reason"),
        ];
        
        let inputs = ExecutionInputs {
            device: "LivingRoom".to_string(),
            ..Default::default()
        };
        
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        // Execution should fail because exec_out is not connected
        assert!(!result.completed);
        assert!(result.error.is_some());
        let error_msg = result.error.unwrap();
        assert!(error_msg.contains("not connected"), "Error should indicate exec_out is not connected, got: {}", error_msg);
        // The reset flag should still be propagated even in error case since the node was executed
        assert!(result.reset_active_command, "Reset Active Command flag should be set even when exec_out is not connected");
    }
}
