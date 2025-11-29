use super::node_system::{Node, NodeDefinition, NodeInput, NodeOutput, ValueType};

/// Start Node - Entry point for the device evaluation flow
/// This node provides all the required data to start evaluating an AC device
/// Outputs device identifier, current temperature, and auto mode status
pub struct StartNode;

impl Node for StartNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "flow_start",
            "Start",
            "Entry point for device evaluation. Provides device data including identifier, temperature reading, and auto/manual mode status. One Start node should exist per evaluation flow.",
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
                    "temperature",
                    "Temperature",
                    "Current temperature reading from the device sensor in Celsius",
                    ValueType::Float,
                ),
                NodeOutput::new(
                    "is_auto_mode",
                    "Is Auto Mode",
                    "True if the device is in automatic mode, false if in manual mode",
                    ValueType::Boolean,
                ),
            ],
        )
    }
}

/// Execute Action Node - End point that executes the command and stores to database
/// Takes raw AC control values: temperature, mode (Heat/Cool/Off), and isPowerful
/// This node represents the final action in the evaluation flow
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
                    ValueType::Enum(vec![
                        "Undefined".to_string(),
                        "IceException".to_string(),
                        "PirDetection".to_string(),
                        "NobodyHome".to_string(),
                        "MildTemperature".to_string(),
                        "MajorTemperatureChangePending".to_string(),
                        "ExcessiveSolarPower".to_string(),
                        "ManualToAutoTransition".to_string(),
                    ]),
                    true,
                ),
            ],
            vec![], // No outputs - this is a terminal node
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
        assert_eq!(def.outputs.len(), 3); // device, temperature, is_auto_mode
        
        // Verify device output is an enum with device values
        let device_output = def.outputs.iter().find(|o| o.id == "device").unwrap();
        match &device_output.value_type {
            ValueType::Enum(values) => {
                assert!(values.contains(&"LivingRoom".to_string()));
                assert!(values.contains(&"Veranda".to_string()));
            }
            _ => panic!("Expected Enum type for device output"),
        }
        
        // Verify temperature output is a float
        let temp_output = def.outputs.iter().find(|o| o.id == "temperature").unwrap();
        assert_eq!(temp_output.value_type, ValueType::Float);
        
        // Verify is_auto_mode output is a boolean
        let auto_mode_output = def.outputs.iter().find(|o| o.id == "is_auto_mode").unwrap();
        assert_eq!(auto_mode_output.value_type, ValueType::Boolean);
    }

    #[test]
    fn test_execute_action_node_definition() {
        let def = ExecuteActionNode::definition();
        
        assert_eq!(def.node_type, "flow_execute_action");
        assert_eq!(def.name, "Execute Action");
        assert_eq!(def.category, "System");
        assert_eq!(def.inputs.len(), 5); // device, temperature, mode, is_powerful, cause_reason
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
        
        // Verify is_powerful input
        let powerful_input = def.inputs.iter().find(|i| i.id == "is_powerful").unwrap();
        assert_eq!(powerful_input.value_type, ValueType::Boolean);
        assert!(powerful_input.required);
        
        // Verify cause_reason input
        let cause_input = def.inputs.iter().find(|i| i.id == "cause_reason").unwrap();
        match &cause_input.value_type {
            ValueType::Enum(values) => {
                assert_eq!(values.len(), 8);
                assert!(values.contains(&"Undefined".to_string()));
                assert!(values.contains(&"IceException".to_string()));
                assert!(values.contains(&"PirDetection".to_string()));
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
}
