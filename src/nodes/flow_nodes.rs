use super::node_system::{Node, NodeDefinition, NodeInput, NodeOutput, ValueType};

/// Maximum value for evaluate_every_minutes (24 hours * 60 minutes = 1440)
pub const MAX_EVALUATE_EVERY_MINUTES: i32 = 1440;

/// Start Node - Entry point for the device evaluation flow
/// This node provides all the required data to start evaluating an AC device
/// Outputs device identifier, sensor temperature, environmental data, and mode status
/// 
/// The "Evaluate Every Minutes" input controls how often the AC state is reevaluated.
/// This value determines the interval in minutes between each evaluation cycle.
/// For example, if set to 5, the node logic runs every 5 minutes.
/// In the simulator, this value has no effect but is still reported.
/// 
/// The execution flow pin (▶) starts the evaluation and must be connected to control
/// when downstream nodes execute.
pub struct StartNode;

impl Node for StartNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "flow_start",
            "Start",
            "Entry point for device evaluation. Provides device data including identifier, sensor temperature, outdoor conditions, power data, and auto/manual mode status. One Start node should exist per evaluation flow.",
            "System",
            vec![
                NodeInput::new(
                    "evaluate_every_minutes",
                    "Evaluate Every Minutes",
                    "How often to reevaluate the AC state in minutes. This controls the interval between each evaluation cycle. For example, if set to 5, the node logic runs every 5 minutes. Maximum value is 1440 (24 hours). In the simulator, this has no effect but is still reported.",
                    ValueType::Integer,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "exec_out",
                    "▶",
                    "Execution flow output - connect to nodes that control the flow",
                    ValueType::Execution,
                ),
                NodeOutput::new(
                    "device",
                    "Device",
                    "The AC device being evaluated",
                    ValueType::Enum(vec![
                        "LivingRoom".to_string(),
                        "Veranda".to_string(),
                    ]),
                ),
                NodeOutput::new(
                    "device_sensor_temperature",
                    "Device Sensor Temperature",
                    "Current temperature reading from the device sensor in Celsius",
                    ValueType::Float,
                ),
                NodeOutput::new(
                    "is_auto_mode",
                    "Is Auto Mode",
                    "True if the device is in automatic mode, false if in manual mode",
                    ValueType::Boolean,
                ),
                NodeOutput::new(
                    "last_change_minutes",
                    "Last Change Minutes",
                    "Number of minutes since the AC last received a command (i32::MAX if never)",
                    ValueType::Integer,
                ),
                NodeOutput::new(
                    "outdoor_temperature",
                    "Outdoor Temperature",
                    "Current outdoor temperature in Celsius from weather API",
                    ValueType::Float,
                ),
                NodeOutput::new(
                    "is_user_home",
                    "Is User Home",
                    "True if the user is home and awake based on schedule settings",
                    ValueType::Boolean,
                ),
                NodeOutput::new(
                    "net_power_watt",
                    "Net Power Watt",
                    "Current net power in watts (positive = consuming from grid, negative = exporting to grid)",
                    ValueType::Integer,
                ),
                NodeOutput::new(
                    "raw_solar_watt",
                    "Raw Solar Watt",
                    "Current raw solar production in watts",
                    ValueType::Integer,
                ),
                NodeOutput::new(
                    "avg_next_24h_outdoor_temp",
                    "Avg Next 24h Outdoor Temp",
                    "Average outdoor temperature in Celsius forecasted for the next 24 hours. This is the absolute average temperature, not a trend or offset.",
                    ValueType::Float,
                ),
                NodeOutput::new(
                    "active_command",
                    "Active Command",
                    "The active command struct containing the last command sent to the device",
                    ValueType::Object,
                ),
            ],
        )
    }
}

/// Execute Action Node - End point that executes the command and stores to database
/// Takes raw AC control values: temperature, mode (Heat/Cool/Off), fan_speed, and isPowerful
/// This node represents the final action in the evaluation flow
/// NOTE: The device is inferred from the evaluation context (Start node) at runtime.
/// NOTE: The cause_reason input accepts a CauseReason type (connect from a Cause Reason node).
/// 
/// Requires an execution flow input to trigger - execution must be connected from
/// Start through If/Sequence nodes to reach this terminal.
pub struct ExecuteActionNode;

impl Node for ExecuteActionNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "flow_execute_action",
            "Execute Action",
            "Executes the AC command and stores the action to the database. This is the end point of an evaluation flow that results in an AC action. The device is inferred from the evaluation context.",
            "System",
            vec![
                NodeInput::new(
                    "exec_in",
                    "▶",
                    "Execution flow input - triggers this action to execute",
                    ValueType::Execution,
                    true,
                ),
                NodeInput::new(
                    "temperature",
                    "Temperature",
                    "Target temperature in Celsius for the AC",
                    ValueType::Float,
                    true,
                ),
                NodeInput::new(
                    "mode",
                    "Mode",
                    "AC operating mode: Heat, Cool, or Off",
                    ValueType::Enum(vec![
                        "Heat".to_string(),
                        "Cool".to_string(),
                        "Off".to_string(),
                    ]),
                    true,
                ),
                NodeInput::new(
                    "fan_speed",
                    "Fan Speed",
                    "Fan speed setting: 0=Auto, 1=High, 2=Medium, 3=Low, 4=Quiet",
                    ValueType::Enum(vec![
                        "Auto".to_string(),
                        "High".to_string(),
                        "Medium".to_string(),
                        "Low".to_string(),
                        "Quiet".to_string(),
                    ]),
                    true,
                ),
                NodeInput::new(
                    "is_powerful",
                    "Is Powerful",
                    "Whether to enable powerful/turbo mode for maximum heating or cooling",
                    ValueType::Boolean,
                    true,
                ),
                NodeInput::new(
                    "cause_reason",
                    "Cause Reason",
                    "The reason for this action (for logging and debugging). Connect from a Cause Reason node.",
                    ValueType::CauseReason(vec![]),
                    true,
                ),
            ],
            vec![], // No outputs - this is a terminal node
        )
    }
}

/// Active Command Node - Provides information about the previously sent command to a device
/// This node outputs the properties of the last command sent to the AC device.
/// The active command may not exist if no command has been sent yet, so the "Is Defined" 
/// output must be checked before using other output values.
pub struct ActiveCommandNode;

impl Node for ActiveCommandNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "flow_active_command",
            "Active Command",
            "Provides the properties of the last command sent to the AC device. The 'Is Defined' output must be handled to check if a command has been previously sent.",
            "System",
            vec![
                NodeInput::new(
                    "active_command",
                    "Active Command",
                    "The active command struct from the Start node",
                    ValueType::Object,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "is_defined",
                    "Is Defined",
                    "True if an active command exists (a command was previously sent to this device)",
                    ValueType::Boolean,
                ),
                NodeOutput::new(
                    "is_on",
                    "Is On",
                    "True if the AC is currently on (based on last command)",
                    ValueType::Boolean,
                ),
                NodeOutput::new(
                    "temperature",
                    "Temperature",
                    "Target temperature in Celsius from the last command",
                    ValueType::Float,
                ),
                NodeOutput::new(
                    "mode",
                    "Mode",
                    "AC operating mode from the last command: Heat, Cool, or Off",
                    ValueType::Enum(vec![
                        "Heat".to_string(),
                        "Cool".to_string(),
                        "Off".to_string(),
                    ]),
                ),
                NodeOutput::new(
                    "fan_speed",
                    "Fan Speed",
                    "Fan speed setting from the last command (0-5, where 0 is auto)",
                    ValueType::Integer,
                ),
                NodeOutput::new(
                    "swing",
                    "Swing",
                    "Swing setting from the last command (0 = off, 1 = on)",
                    ValueType::Integer,
                ),
                NodeOutput::new(
                    "is_powerful",
                    "Is Powerful",
                    "True if powerful/turbo mode was enabled in the last command",
                    ValueType::Boolean,
                ),
            ],
        )
    }
}

/// Do Nothing Node - Terminates the flow without executing any action
/// Requires an execution flow input to trigger - execution must be connected from
/// Start through If/Sequence nodes to reach this terminal.
/// Use this when the evaluation determines no action should be taken
/// NOTE: The device is inferred from the evaluation context (Start node) at runtime.
/// NOTE: The cause_reason input accepts a CauseReason type (connect from a Cause Reason node).
pub struct DoNothingNode;

impl Node for DoNothingNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "flow_do_nothing",
            "Do Nothing",
            "Terminates the evaluation flow without executing any AC action. When execution reaches this node, the flow ends for the current device. The device is inferred from the evaluation context.",
            "System",
            vec![
                NodeInput::new(
                    "exec_in",
                    "▶",
                    "Execution flow input - triggers this node to end the evaluation",
                    ValueType::Execution,
                    true,
                ),
                NodeInput::new(
                    "cause_reason",
                    "Cause Reason",
                    "The reason for not taking action (for debugging and simulation display). Connect from a Cause Reason node.",
                    ValueType::CauseReason(vec![]),
                    true,
                ),
            ],
            vec![], // No outputs - this is a terminal node that does nothing
        )
    }
}

/// Reset Active Command Node - Resets the active command to the startup state
/// This is a pass-through execution node with input and output execution pins.
/// When execution flows through this node, it resets the active command for the current
/// device to its initial/undefined state (as if no command has been sent).
/// NOTE: The device is inferred from the evaluation context (Start node) at runtime.
pub struct ResetActiveCommandNode;

impl Node for ResetActiveCommandNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "flow_reset_active_command",
            "Reset Active Command",
            "Resets the active command to undefined state (as on startup). Execution flows through this node to the next connected node. The device is inferred from the evaluation context.",
            "System",
            vec![
                NodeInput::new(
                    "exec_in",
                    "▶",
                    "Execution flow input - triggers the reset",
                    ValueType::Execution,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "exec_out",
                    "▶",
                    "Execution flow output - continues to the next node",
                    ValueType::Execution,
                ),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_node_definition() {
        let def = StartNode::definition();
        
        assert_eq!(def.node_type, "flow_start");
        assert_eq!(def.name, "Start");
        assert_eq!(def.category, "System");
        assert_eq!(def.inputs.len(), 1); // evaluate_every_minutes input
        assert_eq!(def.outputs.len(), 11); // exec_out, device, device_sensor_temperature, is_auto_mode, last_change_minutes, outdoor_temperature, is_user_home, net_power_watt, raw_solar_watt, avg_next_24h_outdoor_temp, active_command
        
        // Verify evaluate_every_minutes input
        let eval_input = def.inputs.iter().find(|i| i.id == "evaluate_every_minutes").unwrap();
        assert_eq!(eval_input.value_type, ValueType::Integer);
        assert!(eval_input.required);
        assert_eq!(eval_input.label, "Evaluate Every Minutes");
        
        // Verify exec_out output (execution flow)
        let exec_output = def.outputs.iter().find(|o| o.id == "exec_out").unwrap();
        assert_eq!(exec_output.value_type, ValueType::Execution);
        
        // Verify device output is an enum with device values
        let device_output = def.outputs.iter().find(|o| o.id == "device").unwrap();
        match &device_output.value_type {
            ValueType::Enum(values) => {
                assert!(values.contains(&"LivingRoom".to_string()));
                assert!(values.contains(&"Veranda".to_string()));
            }
            _ => panic!("Expected Enum type for device output"),
        }
        
        // Verify device_sensor_temperature output is a float
        let temp_output = def.outputs.iter().find(|o| o.id == "device_sensor_temperature").unwrap();
        assert_eq!(temp_output.value_type, ValueType::Float);
        assert_eq!(temp_output.label, "Device Sensor Temperature");
        
        // Verify is_auto_mode output is a boolean
        let auto_mode_output = def.outputs.iter().find(|o| o.id == "is_auto_mode").unwrap();
        assert_eq!(auto_mode_output.value_type, ValueType::Boolean);
        
        // Verify last_change_minutes output is an integer
        let last_change_output = def.outputs.iter().find(|o| o.id == "last_change_minutes").unwrap();
        assert_eq!(last_change_output.value_type, ValueType::Integer);
        
        // Verify outdoor_temperature output is a float
        let outdoor_temp_output = def.outputs.iter().find(|o| o.id == "outdoor_temperature").unwrap();
        assert_eq!(outdoor_temp_output.value_type, ValueType::Float);
        
        // Verify is_user_home output is a boolean
        let user_home_output = def.outputs.iter().find(|o| o.id == "is_user_home").unwrap();
        assert_eq!(user_home_output.value_type, ValueType::Boolean);
        
        // Verify net_power_watt output is an integer
        let net_power_output = def.outputs.iter().find(|o| o.id == "net_power_watt").unwrap();
        assert_eq!(net_power_output.value_type, ValueType::Integer);
        
        // Verify raw_solar_watt output is an integer
        let solar_output = def.outputs.iter().find(|o| o.id == "raw_solar_watt").unwrap();
        assert_eq!(solar_output.value_type, ValueType::Integer);
        
        // Verify avg_next_24h_outdoor_temp output is a float
        let avg_temp_output = def.outputs.iter().find(|o| o.id == "avg_next_24h_outdoor_temp").unwrap();
        assert_eq!(avg_temp_output.value_type, ValueType::Float);
    }
    
    #[test]
    fn test_max_evaluate_every_minutes_constant() {
        // Verify the constant value is 1440 (24 hours * 60 minutes)
        assert_eq!(MAX_EVALUATE_EVERY_MINUTES, 1440);
    }

    #[test]
    fn test_execute_action_node_definition() {
        let def = ExecuteActionNode::definition();
        
        assert_eq!(def.node_type, "flow_execute_action");
        assert_eq!(def.name, "Execute Action");
        assert_eq!(def.category, "System");
        assert_eq!(def.inputs.len(), 6); // exec_in, temperature, mode, fan_speed, is_powerful, cause_reason (device is inferred from context)
        assert_eq!(def.outputs.len(), 0); // Terminal node has no outputs
        
        // Verify exec_in input (execution flow)
        let exec_input = def.inputs.iter().find(|i| i.id == "exec_in").unwrap();
        assert_eq!(exec_input.value_type, ValueType::Execution);
        assert!(exec_input.required);
        
        // Verify temperature input
        let temp_input = def.inputs.iter().find(|i| i.id == "temperature").unwrap();
        assert_eq!(temp_input.value_type, ValueType::Float);
        assert!(temp_input.required);
        
        // Verify mode input (Heat/Cool/Off)
        let mode_input = def.inputs.iter().find(|i| i.id == "mode").unwrap();
        match &mode_input.value_type {
            ValueType::Enum(values) => {
                assert_eq!(values.len(), 3);
                assert!(values.contains(&"Heat".to_string()));
                assert!(values.contains(&"Cool".to_string()));
                assert!(values.contains(&"Off".to_string()));
            }
            _ => panic!("Expected Enum type for mode input"),
        }
        assert!(mode_input.required);
        
        // Verify fan_speed input (0=Auto, 1=High, 2=Medium, 3=Low, 4=Quiet)
        let fan_speed_input = def.inputs.iter().find(|i| i.id == "fan_speed").unwrap();
        match &fan_speed_input.value_type {
            ValueType::Enum(values) => {
                assert_eq!(values.len(), 5);
                assert!(values.contains(&"Auto".to_string()));
                assert!(values.contains(&"High".to_string()));
                assert!(values.contains(&"Medium".to_string()));
                assert!(values.contains(&"Low".to_string()));
                assert!(values.contains(&"Quiet".to_string()));
            }
            _ => panic!("Expected Enum type for fan_speed input"),
        }
        assert!(fan_speed_input.required);
        
        // Verify is_powerful input
        let powerful_input = def.inputs.iter().find(|i| i.id == "is_powerful").unwrap();
        assert_eq!(powerful_input.value_type, ValueType::Boolean);
        assert!(powerful_input.required);
        
        // Verify cause_reason input (CauseReason type with empty options - populated from database at runtime)
        let cause_input = def.inputs.iter().find(|i| i.id == "cause_reason").unwrap();
        match &cause_input.value_type {
            ValueType::CauseReason(options) => {
                assert_eq!(options.len(), 0, "Cause reason options should be empty (loaded from database at runtime)");
            }
            _ => panic!("Expected CauseReason type for cause_reason input"),
        }
        assert!(cause_input.required);
        
        // Verify no device input (device is inferred from context)
        assert!(def.inputs.iter().find(|i| i.id == "device").is_none());
    }

    #[test]
    fn test_do_nothing_node_definition() {
        let def = DoNothingNode::definition();
        
        assert_eq!(def.node_type, "flow_do_nothing");
        assert_eq!(def.name, "Do Nothing");
        assert_eq!(def.category, "System");
        assert_eq!(def.inputs.len(), 2); // exec_in and cause_reason inputs
        assert_eq!(def.outputs.len(), 0); // Terminal node has no outputs
        
        // Verify exec_in input (execution flow)
        let exec_input = def.inputs.iter().find(|i| i.id == "exec_in").unwrap();
        assert_eq!(exec_input.value_type, ValueType::Execution);
        assert!(exec_input.required);
        
        // Verify cause_reason input (CauseReason type with empty options - populated from database at runtime)
        let cause_input = def.inputs.iter().find(|i| i.id == "cause_reason").unwrap();
        match &cause_input.value_type {
            ValueType::CauseReason(options) => {
                assert_eq!(options.len(), 0, "Cause reason options should be empty (loaded from database at runtime)");
            }
            _ => panic!("Expected CauseReason type for cause_reason input"),
        }
        assert!(cause_input.required);
        
        // Verify no device input (device is inferred from context)
        assert!(def.inputs.iter().find(|i| i.id == "device").is_none());
    }

    #[test]
    fn test_flow_nodes_serializable() {
        let definitions = vec![
            StartNode::definition(),
            ExecuteActionNode::definition(),
            DoNothingNode::definition(),
            ActiveCommandNode::definition(),
            ResetActiveCommandNode::definition(),
        ];
        
        for def in definitions {
            let json = serde_json::to_string(&def).unwrap();
            let deserialized: NodeDefinition = serde_json::from_str(&json).unwrap();
            assert_eq!(def.node_type, deserialized.node_type);
            assert_eq!(def.inputs.len(), deserialized.inputs.len());
            assert_eq!(def.outputs.len(), deserialized.outputs.len());
        }
    }

    #[test]
    fn test_start_node_is_source_node() {
        let def = StartNode::definition();
        // Start node now has 1 input (evaluate_every_minutes) but still acts as a source
        // because it doesn't require connections from other nodes for its output data
        assert_eq!(def.inputs.len(), 1, "Start node should have 1 input (evaluate_every_minutes)");
        assert!(def.outputs.len() > 0, "Start node should have outputs");
    }

    #[test]
    fn test_terminal_nodes_have_no_outputs() {
        let execute_def = ExecuteActionNode::definition();
        assert_eq!(execute_def.outputs.len(), 0, "Execute Action should have no outputs (terminal)");
        
        let do_nothing_def = DoNothingNode::definition();
        assert_eq!(do_nothing_def.outputs.len(), 0, "Do Nothing should have no outputs (terminal)");
    }

    #[test]
    fn test_active_command_node_definition() {
        let def = ActiveCommandNode::definition();
        
        assert_eq!(def.node_type, "flow_active_command");
        assert_eq!(def.name, "Active Command");
        assert_eq!(def.category, "System");
        assert_eq!(def.inputs.len(), 1); // active_command input
        assert_eq!(def.outputs.len(), 7); // is_defined, is_on, temperature, mode, fan_speed, swing, is_powerful
        
        // Verify input
        let input = &def.inputs[0];
        assert_eq!(input.id, "active_command");
        assert_eq!(input.value_type, ValueType::Object);
        assert!(input.required);
        
        // Verify is_defined output
        let is_defined_output = def.outputs.iter().find(|o| o.id == "is_defined").unwrap();
        assert_eq!(is_defined_output.value_type, ValueType::Boolean);
        
        // Verify is_on output
        let is_on_output = def.outputs.iter().find(|o| o.id == "is_on").unwrap();
        assert_eq!(is_on_output.value_type, ValueType::Boolean);
        
        // Verify temperature output
        let temp_output = def.outputs.iter().find(|o| o.id == "temperature").unwrap();
        assert_eq!(temp_output.value_type, ValueType::Float);
        
        // Verify mode output
        let mode_output = def.outputs.iter().find(|o| o.id == "mode").unwrap();
        match &mode_output.value_type {
            ValueType::Enum(values) => {
                assert_eq!(values.len(), 3);
                assert!(values.contains(&"Heat".to_string()));
                assert!(values.contains(&"Cool".to_string()));
                assert!(values.contains(&"Off".to_string()));
            }
            _ => panic!("Expected Enum type for mode output"),
        }
        
        // Verify fan_speed output
        let fan_speed_output = def.outputs.iter().find(|o| o.id == "fan_speed").unwrap();
        assert_eq!(fan_speed_output.value_type, ValueType::Integer);
        
        // Verify swing output
        let swing_output = def.outputs.iter().find(|o| o.id == "swing").unwrap();
        assert_eq!(swing_output.value_type, ValueType::Integer);
        
        // Verify is_powerful output
        let is_powerful_output = def.outputs.iter().find(|o| o.id == "is_powerful").unwrap();
        assert_eq!(is_powerful_output.value_type, ValueType::Boolean);
    }

    #[test]
    fn test_active_command_node_serializable() {
        let def = ActiveCommandNode::definition();
        let json = serde_json::to_string(&def).unwrap();
        let deserialized: NodeDefinition = serde_json::from_str(&json).unwrap();
        
        assert_eq!(def.node_type, deserialized.node_type);
        assert_eq!(def.inputs.len(), deserialized.inputs.len());
        assert_eq!(def.outputs.len(), deserialized.outputs.len());
    }

    #[test]
    fn test_reset_active_command_node_definition() {
        let def = ResetActiveCommandNode::definition();
        
        assert_eq!(def.node_type, "flow_reset_active_command");
        assert_eq!(def.name, "Reset Active Command");
        assert_eq!(def.category, "System");
        assert_eq!(def.inputs.len(), 1); // exec_in
        assert_eq!(def.outputs.len(), 1); // exec_out
        
        // Verify exec_in input (execution flow)
        let exec_input = def.inputs.iter().find(|i| i.id == "exec_in").unwrap();
        assert_eq!(exec_input.value_type, ValueType::Execution);
        assert!(exec_input.required);
        
        // Verify exec_out output (execution flow)
        let exec_output = def.outputs.iter().find(|o| o.id == "exec_out").unwrap();
        assert_eq!(exec_output.value_type, ValueType::Execution);
    }

    #[test]
    fn test_reset_active_command_node_serializable() {
        let def = ResetActiveCommandNode::definition();
        let json = serde_json::to_string(&def).unwrap();
        let deserialized: NodeDefinition = serde_json::from_str(&json).unwrap();
        
        assert_eq!(def.node_type, deserialized.node_type);
        assert_eq!(def.inputs.len(), deserialized.inputs.len());
        assert_eq!(def.outputs.len(), deserialized.outputs.len());
    }
}
