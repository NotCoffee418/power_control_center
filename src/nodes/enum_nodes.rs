use super::node_system::{Node, NodeDefinition, NodeInput, NodeOutput, ValueType};

/// Device node - represents an AC device enum selection
/// This node provides a dropdown/combobox for selecting an AC device
pub struct DeviceNode;

impl Node for DeviceNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "device",
            "Device",
            "Select an AC device from the list of available devices.",
            "Enums",
            vec![], // No inputs - this is a source node with enum selection
            vec![
                NodeOutput::new(
                    "device",
                    "Device",
                    "The selected AC device",
                    ValueType::Enum(vec![
                        "LivingRoom".to_string(),
                        "Veranda".to_string(),
                    ]),
                ),
            ],
        )
    }
}

/// Currently Evaluating Device node - outputs the device being evaluated
/// This is a source node that provides the device currently being processed by the planner
pub struct CurrentlyEvaluatingDeviceNode;

impl Node for CurrentlyEvaluatingDeviceNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "currently_evaluating_device",
            "Currently Evaluating Device",
            "Outputs the AC device currently being evaluated by the planner, along with its temperature and mode.",
            "System",
            vec![], // No inputs - this is a source node
            vec![
                NodeOutput::new(
                    "device",
                    "Device",
                    "The device currently being evaluated",
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

/// Intensity node - represents an intensity level enum selection
/// This node provides a dropdown for selecting intensity levels (Low, Medium, High)
pub struct IntensityNode;

impl Node for IntensityNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "intensity",
            "Intensity",
            "Select an intensity level for AC operation.",
            "Enums",
            vec![], // No inputs - this is a source node with enum selection
            vec![
                NodeOutput::new(
                    "intensity",
                    "Intensity",
                    "The selected intensity level",
                    ValueType::Enum(vec![
                        "Low".to_string(),
                        "Medium".to_string(),
                        "High".to_string(),
                    ]),
                ),
            ],
        )
    }
}

/// CauseReason node - represents a cause/reason for AC action
/// This node provides a dropdown for selecting cause reason
pub struct CauseReasonNode;

impl Node for CauseReasonNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "cause_reason",
            "Cause Reason",
            "Select a cause/reason for an AC action or decision.",
            "Enums",
            vec![], // No inputs - this is a source node with enum selection
            vec![
                NodeOutput::new(
                    "cause_reason",
                    "Cause Reason",
                    "The selected cause reason",
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
                ),
            ],
        )
    }
}

/// RequestMode node - represents a request mode for AC operation
/// This node provides a dropdown for selecting request mode (Heat, Cool, Off)
pub struct RequestModeNode;

impl Node for RequestModeNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "request_mode",
            "Request Mode",
            "Select a request mode for AC operation.",
            "Enums",
            vec![], // No inputs - this is a source node with enum selection
            vec![
                NodeOutput::new(
                    "request_mode",
                    "Request Mode",
                    "The selected request mode",
                    ValueType::Enum(vec![
                        "Heat".to_string(),
                        "Cool".to_string(),
                        "Off".to_string(),
                    ]),
                ),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_node_definition() {
        let def = DeviceNode::definition();
        
        assert_eq!(def.node_type, "device");
        assert_eq!(def.name, "Device");
        assert_eq!(def.category, "Enums");
        assert_eq!(def.inputs.len(), 0); // Source node has no inputs
        assert_eq!(def.outputs.len(), 1); // One output: device
        
        // Verify output is an enum with device values
        match &def.outputs[0].value_type {
            ValueType::Enum(values) => {
                assert!(values.contains(&"LivingRoom".to_string()));
                assert!(values.contains(&"Veranda".to_string()));
            }
            _ => panic!("Expected Enum type for device output"),
        }
    }

    #[test]
    fn test_currently_evaluating_device_node_definition() {
        let def = CurrentlyEvaluatingDeviceNode::definition();
        
        assert_eq!(def.node_type, "currently_evaluating_device");
        assert_eq!(def.name, "Currently Evaluating Device");
        assert_eq!(def.category, "System");
        assert_eq!(def.inputs.len(), 0); // Source node has no inputs
        assert_eq!(def.outputs.len(), 3); // Three outputs: device, temperature, is_auto_mode
        
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
    fn test_intensity_node_definition() {
        let def = IntensityNode::definition();
        
        assert_eq!(def.node_type, "intensity");
        assert_eq!(def.name, "Intensity");
        assert_eq!(def.category, "Enums");
        assert_eq!(def.inputs.len(), 0); // Source node has no inputs
        assert_eq!(def.outputs.len(), 1); // One output: intensity
        
        // Verify output is an enum with intensity values
        match &def.outputs[0].value_type {
            ValueType::Enum(values) => {
                assert_eq!(values.len(), 3);
                assert!(values.contains(&"Low".to_string()));
                assert!(values.contains(&"Medium".to_string()));
                assert!(values.contains(&"High".to_string()));
            }
            _ => panic!("Expected Enum type for intensity output"),
        }
    }

    #[test]
    fn test_enum_nodes_serializable() {
        let definitions = vec![
            DeviceNode::definition(),
            CurrentlyEvaluatingDeviceNode::definition(),
            IntensityNode::definition(),
            CauseReasonNode::definition(),
            RequestModeNode::definition(),
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
    fn test_cause_reason_node_definition() {
        let def = CauseReasonNode::definition();
        
        assert_eq!(def.node_type, "cause_reason");
        assert_eq!(def.name, "Cause Reason");
        assert_eq!(def.category, "Enums");
        assert_eq!(def.inputs.len(), 0); // Source node has no inputs
        assert_eq!(def.outputs.len(), 1); // One output: cause_reason
        
        // Verify output is an enum with cause reason values
        match &def.outputs[0].value_type {
            ValueType::Enum(values) => {
                assert_eq!(values.len(), 8);
                assert!(values.contains(&"Undefined".to_string()));
                assert!(values.contains(&"IceException".to_string()));
                assert!(values.contains(&"PirDetection".to_string()));
                assert!(values.contains(&"NobodyHome".to_string()));
                assert!(values.contains(&"MildTemperature".to_string()));
                assert!(values.contains(&"MajorTemperatureChangePending".to_string()));
                assert!(values.contains(&"ExcessiveSolarPower".to_string()));
                assert!(values.contains(&"ManualToAutoTransition".to_string()));
            }
            _ => panic!("Expected Enum type for cause_reason output"),
        }
    }

    #[test]
    fn test_request_mode_node_definition() {
        let def = RequestModeNode::definition();
        
        assert_eq!(def.node_type, "request_mode");
        assert_eq!(def.name, "Request Mode");
        assert_eq!(def.category, "Enums");
        assert_eq!(def.inputs.len(), 0); // Source node has no inputs
        assert_eq!(def.outputs.len(), 1); // One output: request_mode
        
        // Verify output is an enum with request mode values
        match &def.outputs[0].value_type {
            ValueType::Enum(values) => {
                assert_eq!(values.len(), 3);
                assert!(values.contains(&"Heat".to_string()));
                assert!(values.contains(&"Cool".to_string()));
                assert!(values.contains(&"Off".to_string()));
            }
            _ => panic!("Expected Enum type for request_mode output"),
        }
    }
}
