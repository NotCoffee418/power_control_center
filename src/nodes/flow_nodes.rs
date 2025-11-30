use super::node_system::{Node, NodeDefinition, NodeInput, NodeOutput, ValueType};

/// Start Node - Entry point for the device evaluation flow
/// This node provides all the required data to start evaluating an AC device
/// Outputs device identifier, sensor temperature, environmental data, and mode status
pub struct StartNode;

impl Node for StartNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "flow_start",
            "Start",
            "Entry point for device evaluation. Provides device data including identifier, sensor temperature, outdoor conditions, power data, and auto/manual mode status. One Start node should exist per evaluation flow.",
            "System",
            vec![], // No inputs - this is the entry point
            vec![
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
/// NOTE: The cause_reason input's hardcoded enum values are deprecated.
/// The actual cause reasons are loaded from the database at runtime
/// via the get_node_definitions API endpoint.
pub struct ExecuteActionNode;

impl Node for ExecuteActionNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "flow_execute_action",
            "Execute Action",
            "Executes the AC command and stores the action to the database. This is the end point of an evaluation flow that results in an AC action. Accepts raw AC control values.",
            "System",
            vec![
                NodeInput::new(
                    "device",
                    "Device",
                    "The AC device to control",
                    ValueType::Enum(vec![
                        "LivingRoom".to_string(),
                        "Veranda".to_string(),
                    ]),
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
                    "The reason for this action (for logging and debugging)",
                    // Deprecated: These values are replaced with database values at runtime
                    ValueType::Enum(vec![]),
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
/// Takes any input and produces no output, similar to RequestMode::NoChange
/// Use this when the evaluation determines no action should be taken
pub struct DoNothingNode;

impl Node for DoNothingNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "flow_do_nothing",
            "Do Nothing",
            "Terminates the evaluation flow without executing any AC action. Use this when conditions determine that no change to the AC state is needed.",
            "System",
            vec![
                NodeInput::new(
                    "input",
                    "Input",
                    "Any value that triggers this node. The value itself is discarded.",
                    ValueType::Any,
                    true,
                ),
            ],
            vec![], // No outputs - this is a terminal node that does nothing
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
        assert_eq!(def.inputs.len(), 0); // Source node has no inputs
        assert_eq!(def.outputs.len(), 10); // device, device_sensor_temperature, is_auto_mode, last_change_minutes, outdoor_temperature, is_user_home, net_power_watt, raw_solar_watt, avg_next_24h_outdoor_temp, active_command
        
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
    fn test_execute_action_node_definition() {
        let def = ExecuteActionNode::definition();
        
        assert_eq!(def.node_type, "flow_execute_action");
        assert_eq!(def.name, "Execute Action");
        assert_eq!(def.category, "System");
        assert_eq!(def.inputs.len(), 6); // device, temperature, mode, fan_speed, is_powerful, cause_reason
        assert_eq!(def.outputs.len(), 0); // Terminal node has no outputs
        
        // Verify device input
        let device_input = def.inputs.iter().find(|i| i.id == "device").unwrap();
        match &device_input.value_type {
            ValueType::Enum(values) => {
                assert!(values.contains(&"LivingRoom".to_string()));
                assert!(values.contains(&"Veranda".to_string()));
            }
            _ => panic!("Expected Enum type for device input"),
        }
        assert!(device_input.required);
        
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
        
        // Verify cause_reason input (actual values loaded from database at runtime)
        let cause_input = def.inputs.iter().find(|i| i.id == "cause_reason").unwrap();
        match &cause_input.value_type {
            ValueType::Enum(values) => {
                assert_eq!(values.len(), 0, "Cause reason values should be empty (loaded from database at runtime)");
            }
            _ => panic!("Expected Enum type for cause_reason input"),
        }
        assert!(cause_input.required);
    }

    #[test]
    fn test_do_nothing_node_definition() {
        let def = DoNothingNode::definition();
        
        assert_eq!(def.node_type, "flow_do_nothing");
        assert_eq!(def.name, "Do Nothing");
        assert_eq!(def.category, "System");
        assert_eq!(def.inputs.len(), 1); // Single Any input
        assert_eq!(def.outputs.len(), 0); // Terminal node has no outputs
        
        // Verify input is Any type
        let input = &def.inputs[0];
        assert_eq!(input.id, "input");
        assert_eq!(input.value_type, ValueType::Any);
        assert!(input.required);
    }

    #[test]
    fn test_flow_nodes_serializable() {
        let definitions = vec![
            StartNode::definition(),
            ExecuteActionNode::definition(),
            DoNothingNode::definition(),
            ActiveCommandNode::definition(),
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
        assert_eq!(def.inputs.len(), 0, "Start node should have no inputs (source node)");
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
}
