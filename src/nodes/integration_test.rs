/// Integration test to verify that node definitions work correctly
/// This test verifies the complete flow from node definitions to JSON serialization
#[cfg(test)]
mod integration_tests {
    use crate::nodes;
    
    #[test]
    fn test_get_all_node_definitions() {
        let definitions = nodes::get_all_node_definitions();
        
        // Verify we have 20 node definitions:
        // System: 4 (flow_start, currently_evaluating_device, flow_execute_action, flow_do_nothing)
        // Sensors: 1 (pir_detection)
        // Logic: 8 (and, or, nand, if, not, equals, evaluate_number, branch)
        // Primitives: 3 (float, integer, boolean)
        // Enums: 4 (device, intensity, cause_reason, request_mode)
        assert_eq!(definitions.len(), 20);
        
        // Verify system node types
        let node_types: Vec<&str> = definitions.iter().map(|d| d.node_type.as_str()).collect();
        assert!(node_types.contains(&"flow_start"));
        assert!(node_types.contains(&"currently_evaluating_device"));
        assert!(node_types.contains(&"flow_execute_action"));
        assert!(node_types.contains(&"flow_do_nothing"));
        
        // Verify sensor node types
        assert!(node_types.contains(&"pir_detection"));
        
        // Verify logic node types
        assert!(node_types.contains(&"logic_and"));
        assert!(node_types.contains(&"logic_or"));
        assert!(node_types.contains(&"logic_nand"));
        assert!(node_types.contains(&"logic_if"));
        assert!(node_types.contains(&"logic_not"));
        assert!(node_types.contains(&"logic_equals"));
        assert!(node_types.contains(&"logic_evaluate_number"));
        assert!(node_types.contains(&"logic_branch"));
        
        // Verify primitive node types
        assert!(node_types.contains(&"primitive_float"));
        assert!(node_types.contains(&"primitive_integer"));
        assert!(node_types.contains(&"primitive_boolean"));
        
        // Verify enum node types
        assert!(node_types.contains(&"device"));
        assert!(node_types.contains(&"intensity"));
        assert!(node_types.contains(&"cause_reason"));
        assert!(node_types.contains(&"request_mode"));
    }
    
    #[test]
    fn test_node_definitions_json_serialization() {
        let definitions = nodes::get_all_node_definitions();
        
        // Verify each definition can be serialized to JSON
        for def in definitions {
            let json = serde_json::to_string(&def).unwrap();
            assert!(!json.is_empty());
            
            // Verify it can be deserialized back
            let deserialized: nodes::NodeDefinition = serde_json::from_str(&json).unwrap();
            assert_eq!(def.node_type, deserialized.node_type);
            assert_eq!(def.name, deserialized.name);
        }
    }
    
    #[test]
    fn test_node_definitions_have_categories() {
        let definitions = nodes::get_all_node_definitions();
        
        // Verify categories are assigned appropriately
        for def in &definitions {
            match def.node_type.as_str() {
                "flow_start" | "currently_evaluating_device" | "flow_execute_action" | "flow_do_nothing" => {
                    assert_eq!(def.category, "System", "System nodes should be in 'System' category");
                }
                "pir_detection" => {
                    assert_eq!(def.category, "Sensors", "Sensor nodes should be in 'Sensors' category");
                }
                "logic_and" | "logic_or" | "logic_nand" | "logic_if" | "logic_not" | "logic_equals" | "logic_evaluate_number" | "logic_branch" => {
                    assert_eq!(def.category, "Logic", "Logic nodes should be in 'Logic' category");
                }
                "primitive_float" | "primitive_integer" | "primitive_boolean" => {
                    assert_eq!(def.category, "Primitives", "Primitive nodes should be in 'Primitives' category");
                }
                "device" | "intensity" | "cause_reason" | "request_mode" => {
                    assert_eq!(def.category, "Enums", "Enum nodes should be in 'Enums' category");
                }
                _ => panic!("Unexpected node type: {}", def.node_type),
            }
        }
    }
    
    #[test]
    fn test_logic_nodes_have_correct_io() {
        let definitions = nodes::get_all_node_definitions();
        
        // AND, OR, NAND nodes should have 2 boolean inputs and 1 boolean output
        for node_type in &["logic_and", "logic_or", "logic_nand"] {
            let node = definitions.iter().find(|d| d.node_type == *node_type).unwrap();
            assert_eq!(node.inputs.len(), 2, "{} should have 2 inputs", node_type);
            assert_eq!(node.outputs.len(), 1, "{} should have 1 output", node_type);
            
            // Verify all inputs are boolean
            for input in &node.inputs {
                assert_eq!(input.value_type, nodes::ValueType::Boolean);
            }
            // Verify output is boolean
            assert_eq!(node.outputs[0].value_type, nodes::ValueType::Boolean);
        }
        
        // If node should have 1 boolean input and 2 boolean outputs
        let if_node = definitions.iter().find(|d| d.node_type == "logic_if").unwrap();
        assert_eq!(if_node.inputs.len(), 1);
        assert_eq!(if_node.outputs.len(), 2);
        assert_eq!(if_node.inputs[0].value_type, nodes::ValueType::Boolean);
        for output in &if_node.outputs {
            assert_eq!(output.value_type, nodes::ValueType::Boolean);
        }
        
        // NOT node should have 1 boolean input and 1 boolean output
        let not_node = definitions.iter().find(|d| d.node_type == "logic_not").unwrap();
        assert_eq!(not_node.inputs.len(), 1);
        assert_eq!(not_node.outputs.len(), 1);
        assert_eq!(not_node.inputs[0].value_type, nodes::ValueType::Boolean);
        assert_eq!(not_node.outputs[0].value_type, nodes::ValueType::Boolean);
        
        // Equals node should have 2 Any inputs and 1 boolean output
        let equals_node = definitions.iter().find(|d| d.node_type == "logic_equals").unwrap();
        assert_eq!(equals_node.inputs.len(), 2);
        assert_eq!(equals_node.outputs.len(), 1);
        for input in &equals_node.inputs {
            assert_eq!(input.value_type, nodes::ValueType::Any);
        }
        assert_eq!(equals_node.outputs[0].value_type, nodes::ValueType::Boolean);
    }
    
    #[test]
    fn test_primitive_nodes_are_source_nodes() {
        let definitions = nodes::get_all_node_definitions();
        
        // Primitive nodes should have no inputs (they are source nodes)
        for node_type in &["primitive_float", "primitive_integer", "primitive_boolean"] {
            let node = definitions.iter().find(|d| d.node_type == *node_type).unwrap();
            assert_eq!(node.inputs.len(), 0, "{} should have no inputs", node_type);
            assert_eq!(node.outputs.len(), 1, "{} should have 1 output", node_type);
        }
        
        // Verify correct output types
        let float_node = definitions.iter().find(|d| d.node_type == "primitive_float").unwrap();
        assert_eq!(float_node.outputs[0].value_type, nodes::ValueType::Float);
        
        let int_node = definitions.iter().find(|d| d.node_type == "primitive_integer").unwrap();
        assert_eq!(int_node.outputs[0].value_type, nodes::ValueType::Integer);
        
        let bool_node = definitions.iter().find(|d| d.node_type == "primitive_boolean").unwrap();
        assert_eq!(bool_node.outputs[0].value_type, nodes::ValueType::Boolean);
    }
    
    #[test]
    fn test_enum_nodes_are_source_nodes() {
        let definitions = nodes::get_all_node_definitions();
        
        // Device and Intensity nodes should have no inputs (they are source nodes with enum selection)
        let device_node = definitions.iter().find(|d| d.node_type == "device").unwrap();
        assert_eq!(device_node.inputs.len(), 0, "Device node should have no inputs");
        assert_eq!(device_node.outputs.len(), 1, "Device node should have 1 output");
        match &device_node.outputs[0].value_type {
            nodes::ValueType::Enum(values) => {
                assert!(values.contains(&"LivingRoom".to_string()));
                assert!(values.contains(&"Veranda".to_string()));
            }
            _ => panic!("Expected Enum type for device output"),
        }
        
        let intensity_node = definitions.iter().find(|d| d.node_type == "intensity").unwrap();
        assert_eq!(intensity_node.inputs.len(), 0, "Intensity node should have no inputs");
        assert_eq!(intensity_node.outputs.len(), 1, "Intensity node should have 1 output");
        match &intensity_node.outputs[0].value_type {
            nodes::ValueType::Enum(values) => {
                assert!(values.contains(&"Low".to_string()));
                assert!(values.contains(&"Medium".to_string()));
                assert!(values.contains(&"High".to_string()));
            }
            _ => panic!("Expected Enum type for intensity output"),
        }
    }
    
    #[test]
    fn test_pir_detection_node_has_correct_io() {
        let definitions = nodes::get_all_node_definitions();
        let pir_node = definitions.iter().find(|d| d.node_type == "pir_detection").unwrap();
        
        assert_eq!(pir_node.inputs.len(), 2, "PIR node should have 2 inputs");
        assert_eq!(pir_node.outputs.len(), 2, "PIR node should have 2 outputs");
        
        // Verify inputs
        let timeout_input = pir_node.inputs.iter().find(|i| i.id == "timeout_minutes").unwrap();
        assert_eq!(timeout_input.value_type, nodes::ValueType::Integer);
        
        let device_input = pir_node.inputs.iter().find(|i| i.id == "device").unwrap();
        match &device_input.value_type {
            nodes::ValueType::Enum(_) => {}
            _ => panic!("Expected Enum type for device input"),
        }
        
        // Verify outputs
        let triggered_output = pir_node.outputs.iter().find(|o| o.id == "is_recently_triggered").unwrap();
        assert_eq!(triggered_output.value_type, nodes::ValueType::Boolean);
        
        let minutes_output = pir_node.outputs.iter().find(|o| o.id == "last_detection_minutes_ago").unwrap();
        assert_eq!(minutes_output.value_type, nodes::ValueType::Integer);
    }
    
    #[test]
    fn test_currently_evaluating_device_node() {
        let definitions = nodes::get_all_node_definitions();
        let current_device_node = definitions.iter().find(|d| d.node_type == "currently_evaluating_device").unwrap();
        
        assert_eq!(current_device_node.inputs.len(), 0, "Currently Evaluating Device should have no inputs");
        assert_eq!(current_device_node.outputs.len(), 3, "Currently Evaluating Device should have 3 outputs");
        assert_eq!(current_device_node.category, "System");
        
        // Verify device output
        let device_output = current_device_node.outputs.iter().find(|o| o.id == "device").unwrap();
        match &device_output.value_type {
            nodes::ValueType::Enum(values) => {
                assert!(values.contains(&"LivingRoom".to_string()));
                assert!(values.contains(&"Veranda".to_string()));
            }
            _ => panic!("Expected Enum type for device output"),
        }
        
        // Verify temperature output
        let temp_output = current_device_node.outputs.iter().find(|o| o.id == "temperature").unwrap();
        assert_eq!(temp_output.value_type, nodes::ValueType::Float);
        
        // Verify is_auto_mode output
        let auto_mode_output = current_device_node.outputs.iter().find(|o| o.id == "is_auto_mode").unwrap();
        assert_eq!(auto_mode_output.value_type, nodes::ValueType::Boolean);
    }
    
    #[test]
    fn test_flow_start_node() {
        let definitions = nodes::get_all_node_definitions();
        let start_node = definitions.iter().find(|d| d.node_type == "flow_start").unwrap();
        
        assert_eq!(start_node.inputs.len(), 0, "Start node should have no inputs");
        assert_eq!(start_node.outputs.len(), 3, "Start node should have 3 outputs");
        assert_eq!(start_node.category, "System");
        
        // Verify device output
        let device_output = start_node.outputs.iter().find(|o| o.id == "device").unwrap();
        match &device_output.value_type {
            nodes::ValueType::Enum(values) => {
                assert!(values.contains(&"LivingRoom".to_string()));
                assert!(values.contains(&"Veranda".to_string()));
            }
            _ => panic!("Expected Enum type for device output"),
        }
        
        // Verify temperature output
        let temp_output = start_node.outputs.iter().find(|o| o.id == "temperature").unwrap();
        assert_eq!(temp_output.value_type, nodes::ValueType::Float);
        
        // Verify is_auto_mode output
        let auto_mode_output = start_node.outputs.iter().find(|o| o.id == "is_auto_mode").unwrap();
        assert_eq!(auto_mode_output.value_type, nodes::ValueType::Boolean);
    }
    
    #[test]
    fn test_flow_execute_action_node() {
        let definitions = nodes::get_all_node_definitions();
        let execute_node = definitions.iter().find(|d| d.node_type == "flow_execute_action").unwrap();
        
        assert_eq!(execute_node.inputs.len(), 4, "Execute Action node should have 4 inputs");
        assert_eq!(execute_node.outputs.len(), 0, "Execute Action node should have no outputs (terminal)");
        assert_eq!(execute_node.category, "System");
        
        // Verify all inputs exist and are required
        let input_ids: Vec<&str> = execute_node.inputs.iter().map(|i| i.id.as_str()).collect();
        assert!(input_ids.contains(&"device"));
        assert!(input_ids.contains(&"request_mode"));
        assert!(input_ids.contains(&"intensity"));
        assert!(input_ids.contains(&"cause_reason"));
        
        for input in &execute_node.inputs {
            assert!(input.required, "All Execute Action inputs should be required");
        }
    }
    
    #[test]
    fn test_flow_do_nothing_node() {
        let definitions = nodes::get_all_node_definitions();
        let do_nothing_node = definitions.iter().find(|d| d.node_type == "flow_do_nothing").unwrap();
        
        assert_eq!(do_nothing_node.inputs.len(), 1, "Do Nothing node should have 1 input");
        assert_eq!(do_nothing_node.outputs.len(), 0, "Do Nothing node should have no outputs (terminal)");
        assert_eq!(do_nothing_node.category, "System");
        
        // Verify input is Any type
        let input = &do_nothing_node.inputs[0];
        assert_eq!(input.id, "input");
        assert_eq!(input.value_type, nodes::ValueType::Any);
        assert!(input.required);
    }
}
