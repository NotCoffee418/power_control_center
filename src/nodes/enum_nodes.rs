use super::node_system::{Node, NodeDefinition, NodeOutput, ValueType};

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
/// NOTE: The hardcoded enum values below are deprecated defaults.
/// The actual cause reasons are loaded from the database at runtime
/// via the get_node_definitions API endpoint.
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
                    // Deprecated: These values are replaced with database values at runtime
                    ValueType::Enum(vec![]),
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
        
        // Verify output is an empty enum (actual values are loaded from database at runtime)
        match &def.outputs[0].value_type {
            ValueType::Enum(values) => {
                assert_eq!(values.len(), 0, "Cause reason values should be empty (loaded from database at runtime)");
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
