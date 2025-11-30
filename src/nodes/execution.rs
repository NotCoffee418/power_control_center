//! Nodeset Execution Engine
//! 
//! This module provides the runtime execution of node graphs.
//! It takes a nodeset configuration (nodes and edges) and input values,
//! then evaluates the graph to produce an output result.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Node type identifiers
pub const NODE_TYPE_START: &str = "flow_start";
pub const NODE_TYPE_EXECUTE_ACTION: &str = "flow_execute_action";
pub const NODE_TYPE_DO_NOTHING: &str = "flow_do_nothing";
pub const NODE_TYPE_ACTIVE_COMMAND: &str = "flow_active_command";
pub const NODE_TYPE_LOGIC_AND: &str = "logic_and";
pub const NODE_TYPE_LOGIC_OR: &str = "logic_or";
pub const NODE_TYPE_LOGIC_NAND: &str = "logic_nand";
pub const NODE_TYPE_LOGIC_IF: &str = "logic_if";
pub const NODE_TYPE_LOGIC_NOT: &str = "logic_not";
pub const NODE_TYPE_LOGIC_EQUALS: &str = "logic_equals";
pub const NODE_TYPE_LOGIC_EVALUATE_NUMBER: &str = "logic_evaluate_number";
pub const NODE_TYPE_LOGIC_BRANCH: &str = "logic_branch";
pub const NODE_TYPE_PRIMITIVE_FLOAT: &str = "primitive_float";
pub const NODE_TYPE_PRIMITIVE_INTEGER: &str = "primitive_integer";
pub const NODE_TYPE_PRIMITIVE_BOOLEAN: &str = "primitive_boolean";
pub const NODE_TYPE_DEVICE: &str = "device";
pub const NODE_TYPE_INTENSITY: &str = "intensity";
pub const NODE_TYPE_CAUSE_REASON: &str = "cause_reason";
pub const NODE_TYPE_REQUEST_MODE: &str = "request_mode";
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
    pub outside_temperature_trend: f64,
    /// PIR detection state by device: (is_recently_triggered, minutes_ago)
    pub pir_state: HashMap<String, (bool, i64)>,
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
    /// Any error that occurred during execution
    pub error: Option<String>,
    /// Validation warnings (e.g., disconnected nodes)
    pub warnings: Vec<String>,
}

/// Action parameters when Execute Action node is reached
#[derive(Debug, Clone, Serialize)]
pub struct ActionResult {
    pub device: String,
    pub temperature: f64,
    pub mode: String,
    pub is_powerful: bool,
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
        })
    }
    
    /// Execute the nodeset and return the result
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
                error: Some(ExecutionError::MissingStartNode.to_string()),
                warnings: vec![],
            };
        }
        
        if start_nodes.len() > 1 {
            return ExecutionResult {
                completed: false,
                terminal_type: None,
                action: None,
                error: Some(ExecutionError::MultipleStartNodes.to_string()),
                warnings: vec![],
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
                error: Some(ExecutionError::MissingTerminalNode.to_string()),
                warnings: vec![],
            };
        }
        
        // Populate the Start node outputs first
        if let Err(e) = self.populate_start_node_outputs(&start_node_id) {
            return ExecutionResult {
                completed: false,
                terminal_type: None,
                action: None,
                error: Some(e.to_string()),
                warnings: vec![],
            };
        }
        
        // Execution Order Decision:
        // We prioritize Execute Action nodes over Do Nothing nodes because:
        // 1. Execute Action represents an actual AC command that should be simulated
        // 2. Do Nothing is a fallback terminal that represents "no action taken"
        // 3. In a well-formed nodeset, only one terminal should be reachable from
        //    the current execution path, but if multiple exist, we prefer action.
        //
        // Note: A future enhancement could trace the actual execution path from
        // Start to determine which terminal is reachable, rather than trying all.
        for (terminal_id, terminal_type) in &terminal_nodes {
            if terminal_type == NODE_TYPE_EXECUTE_ACTION {
                match self.evaluate_execute_action_node(terminal_id) {
                    Ok(action) => {
                        return ExecutionResult {
                            completed: true,
                            terminal_type: Some("Execute Action".to_string()),
                            action: Some(action),
                            error: None,
                            warnings: vec![],
                        };
                    }
                    Err(e) => {
                        return ExecutionResult {
                            completed: false,
                            terminal_type: None,
                            action: None,
                            error: Some(e.to_string()),
                            warnings: vec![],
                        };
                    }
                }
            }
        }
        
        // Check for Do Nothing node
        for (terminal_id, terminal_type) in &terminal_nodes {
            if terminal_type == NODE_TYPE_DO_NOTHING {
                match self.evaluate_do_nothing_node(terminal_id) {
                    Ok(()) => {
                        return ExecutionResult {
                            completed: true,
                            terminal_type: Some("Do Nothing".to_string()),
                            action: None,
                            error: None,
                            warnings: vec![],
                        };
                    }
                    Err(e) => {
                        return ExecutionResult {
                            completed: false,
                            terminal_type: None,
                            action: None,
                            error: Some(e.to_string()),
                            warnings: vec![],
                        };
                    }
                }
            }
        }
        
        ExecutionResult {
            completed: false,
            terminal_type: None,
            action: None,
            error: Some("No valid terminal node could be evaluated".to_string()),
            warnings: vec![],
        }
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
            (start_node_id.to_string(), "outside_temperature_trend".to_string()),
            RuntimeValue::Float(self.inputs.outside_temperature_trend),
        );
        
        Ok(())
    }
    
    /// Evaluate the Execute Action node and return the action parameters
    fn evaluate_execute_action_node(&mut self, node_id: &str) -> Result<ActionResult, ExecutionError> {
        let device = self.get_input_value(node_id, "device")?
            .as_string();
        let temperature = self.get_input_value(node_id, "temperature")?
            .as_f64()
            .ok_or_else(|| ExecutionError::TypeMismatch {
                expected: "Float".to_string(),
                got: "non-numeric".to_string(),
            })?;
        let mode = self.get_input_value(node_id, "mode")?
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
            is_powerful,
            cause_reason,
        })
    }
    
    /// Evaluate the Do Nothing node (just verify the input is connected)
    fn evaluate_do_nothing_node(&mut self, node_id: &str) -> Result<(), ExecutionError> {
        // Just need to verify the input can be evaluated
        let _ = self.get_input_value(node_id, "input")?;
        Ok(())
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
            NODE_TYPE_DEVICE | NODE_TYPE_INTENSITY | NODE_TYPE_CAUSE_REASON | NODE_TYPE_REQUEST_MODE => {
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
            
            NODE_TYPE_LOGIC_IF => {
                let condition = self.get_input_value(&node.id, "condition")?;
                let is_true = match condition {
                    RuntimeValue::Boolean(v) => v,
                    _ => return Err(ExecutionError::TypeMismatch {
                        expected: "Boolean".to_string(),
                        got: condition.type_name().to_string(),
                    }),
                };
                
                // If node outputs true/false based on condition
                if output_id == "true" {
                    Ok(RuntimeValue::Boolean(is_true))
                } else {
                    Ok(RuntimeValue::Boolean(!is_true))
                }
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
                        { "id": "device", "label": "Device" },
                        { "id": "temperature", "label": "Temperature" },
                        { "id": "mode", "label": "Mode" },
                        { "id": "is_powerful", "label": "Is Powerful" },
                        { "id": "cause_reason", "label": "Cause Reason" }
                    ],
                    "outputs": []
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

    #[test]
    fn test_simple_execution() {
        // Create a simple nodeset: Start -> Float(22.0) -> Execute Action
        let nodes = vec![
            create_start_node(),
            create_float_node("float-1", 22.0),
            create_boolean_node("bool-1", false),
            create_enum_node("mode-1", "request_mode", "Heat"),
            create_enum_node("cause-1", "cause_reason", "1"),
            create_execute_action_node(),
        ];
        
        let edges = vec![
            create_edge("start-1", "device", "execute-1", "device"),
            create_edge("float-1", "value", "execute-1", "temperature"),
            create_edge("mode-1", "value", "execute-1", "mode"),
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
    fn test_missing_required_input() {
        // Execute Action without required inputs connected
        let nodes = vec![
            create_start_node(),
            create_execute_action_node(),
        ];
        let edges = vec![]; // No edges - inputs not connected
        
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
            json!({
                "id": "do-nothing-1",
                "type": "custom",
                "position": { "x": 500, "y": 0 },
                "data": {
                    "definition": {
                        "node_type": "flow_do_nothing",
                        "name": "Do Nothing",
                        "category": "System"
                    }
                }
            }),
        ];
        
        let edges = vec![
            create_edge("bool-1", "value", "and-1", "input_1"),
            create_edge("bool-2", "value", "and-1", "input_2"),
            create_edge("and-1", "result", "do-nothing-1", "input"),
        ];
        
        let inputs = ExecutionInputs::default();
        let mut executor = NodesetExecutor::new(&nodes, &edges, inputs).unwrap();
        let result = executor.execute();
        
        assert!(result.completed);
        assert_eq!(result.terminal_type, Some("Do Nothing".to_string()));
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
            create_enum_node("cause-1", "cause_reason", "1"),
            create_boolean_node("powerful", false),
            create_execute_action_node(),
        ];
        
        let edges = vec![
            create_edge("start-1", "device", "execute-1", "device"),
            create_edge("condition", "value", "branch-1", "condition"),
            create_edge("true-val", "value", "branch-1", "true_value"),
            create_edge("false-val", "value", "branch-1", "false_value"),
            create_edge("branch-1", "result", "execute-1", "temperature"),
            create_edge("mode-1", "value", "execute-1", "mode"),
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
            create_enum_node("cause-1", "cause_reason", "1"),
            create_boolean_node("powerful", false),
            create_execute_action_node(),
        ];
        
        let edges = vec![
            create_edge("start-1", "device", "execute-1", "device"),
            create_edge("condition", "value", "branch-1", "condition"),
            create_edge("true-val", "value", "branch-1", "true_value"),
            create_edge("false-val", "value", "branch-1", "false_value"),
            create_edge("branch-1", "result", "execute-1", "temperature"),
            create_edge("mode-1", "value", "execute-1", "mode"),
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
}
