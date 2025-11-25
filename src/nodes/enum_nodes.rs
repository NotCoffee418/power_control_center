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
            "Outputs the AC device currently being evaluated by the planner.",
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
            CurrentlyEvaluatingDeviceNode::definition(),
            IntensityNode::definition(),
        ];
        
        for def in definitions {
            let json = serde_json::to_string(&def).unwrap();
            let deserialized: NodeDefinition = serde_json::from_str(&json).unwrap();
            assert_eq!(def.node_type, deserialized.node_type);
            assert_eq!(def.inputs.len(), deserialized.inputs.len());
            assert_eq!(def.outputs.len(), deserialized.outputs.len());
        }
    }
}
