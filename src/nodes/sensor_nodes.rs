use super::node_system::{Node, NodeDefinition, NodeInput, NodeOutput, ValueType};

/// PIR Detection node - checks PIR sensor detection status
/// Takes a device and timeout, outputs detection status and time since last detection
pub struct PirDetectionNode;

impl Node for PirDetectionNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "pir_detection",
            "PIR Detection",
            "Checks PIR (motion sensor) detection status for a device. Outputs whether motion was recently detected and how many minutes ago the last detection occurred.",
            "Sensors",
            vec![
                NodeInput::new(
                    "timeout_minutes",
                    "Timeout Minutes",
                    "Number of minutes to consider a detection as 'recent'",
                    ValueType::Integer,
                    true,
                ),
                NodeInput::new(
                    "device",
                    "Device",
                    "The device to check PIR detection for",
                    ValueType::Enum(vec![
                        "LivingRoom".to_string(),
                        "Veranda".to_string(),
                    ]),
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "is_recently_triggered",
                    "Is Recently Triggered",
                    "True if PIR was triggered within the timeout period",
                    ValueType::Boolean,
                ),
                NodeOutput::new(
                    "last_detection_minutes_ago",
                    "Last Detection Minutes Ago",
                    "Number of minutes since the last PIR detection (or -1 if never detected)",
                    ValueType::Integer,
                ),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pir_detection_node_definition() {
        let def = PirDetectionNode::definition();
        
        assert_eq!(def.node_type, "pir_detection");
        assert_eq!(def.name, "PIR Detection");
        assert_eq!(def.category, "Sensors");
        assert_eq!(def.inputs.len(), 2); // timeout_minutes and device
        assert_eq!(def.outputs.len(), 2); // is_recently_triggered and last_detection_minutes_ago
        
        // Verify inputs
        let input_ids: Vec<&str> = def.inputs.iter().map(|i| i.id.as_str()).collect();
        assert!(input_ids.contains(&"timeout_minutes"));
        assert!(input_ids.contains(&"device"));
        
        // Verify outputs
        let output_ids: Vec<&str> = def.outputs.iter().map(|o| o.id.as_str()).collect();
        assert!(output_ids.contains(&"is_recently_triggered"));
        assert!(output_ids.contains(&"last_detection_minutes_ago"));
        
        // Verify input types
        let timeout_input = def.inputs.iter().find(|i| i.id == "timeout_minutes").unwrap();
        assert_eq!(timeout_input.value_type, ValueType::Integer);
        assert!(timeout_input.required);
        
        let device_input = def.inputs.iter().find(|i| i.id == "device").unwrap();
        match &device_input.value_type {
            ValueType::Enum(values) => {
                assert!(values.contains(&"LivingRoom".to_string()));
                assert!(values.contains(&"Veranda".to_string()));
            }
            _ => panic!("Expected Enum type for device input"),
        }
        assert!(device_input.required);
        
        // Verify output types
        let triggered_output = def.outputs.iter().find(|o| o.id == "is_recently_triggered").unwrap();
        assert_eq!(triggered_output.value_type, ValueType::Boolean);
        
        let minutes_output = def.outputs.iter().find(|o| o.id == "last_detection_minutes_ago").unwrap();
        assert_eq!(minutes_output.value_type, ValueType::Integer);
    }

    #[test]
    fn test_pir_detection_node_serializable() {
        let def = PirDetectionNode::definition();
        let json = serde_json::to_string(&def).unwrap();
        let deserialized: NodeDefinition = serde_json::from_str(&json).unwrap();
        
        assert_eq!(def.node_type, deserialized.node_type);
        assert_eq!(def.inputs.len(), deserialized.inputs.len());
        assert_eq!(def.outputs.len(), deserialized.outputs.len());
    }
}
