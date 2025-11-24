/// Integration test to verify that node definitions work correctly
/// This test verifies the complete flow from node definitions to JSON serialization
#[cfg(test)]
mod integration_tests {
    use crate::nodes;
    
    #[test]
    fn test_get_all_node_definitions() {
        let definitions = nodes::get_all_node_definitions();
        
        // Verify we have 12 node definitions (5 original + 4 logic + 3 primitives)
        assert_eq!(definitions.len(), 12);
        
        // Verify original node types
        let node_types: Vec<&str> = definitions.iter().map(|d| d.node_type.as_str()).collect();
        assert!(node_types.contains(&"on_evaluate"));
        assert!(node_types.contains(&"ac_plan_input"));
        assert!(node_types.contains(&"ac_planner"));
        assert!(node_types.contains(&"ac_plan_result"));
        assert!(node_types.contains(&"execute_plan"));
        
        // Verify logic node types
        assert!(node_types.contains(&"logic_and"));
        assert!(node_types.contains(&"logic_or"));
        assert!(node_types.contains(&"logic_nand"));
        assert!(node_types.contains(&"logic_if"));
        
        // Verify primitive node types
        assert!(node_types.contains(&"primitive_float"));
        assert!(node_types.contains(&"primitive_integer"));
        assert!(node_types.contains(&"primitive_boolean"));
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
    fn test_ac_plan_input_has_correct_outputs() {
        let definitions = nodes::get_all_node_definitions();
        let ac_input = definitions.iter().find(|d| d.node_type == "ac_plan_input").unwrap();
        
        // Should have 5 outputs matching PlanInput struct fields
        assert_eq!(ac_input.outputs.len(), 5);
        
        let output_ids: Vec<&str> = ac_input.outputs.iter().map(|o| o.id.as_str()).collect();
        assert!(output_ids.contains(&"current_indoor_temp"));
        assert!(output_ids.contains(&"solar_production"));
        assert!(output_ids.contains(&"user_is_home"));
        assert!(output_ids.contains(&"current_outdoor_temp"));
        assert!(output_ids.contains(&"avg_next_12h_outdoor_temp"));
    }
    
    #[test]
    fn test_ac_planner_matches_plan_input_structure() {
        let definitions = nodes::get_all_node_definitions();
        let ac_input = definitions.iter().find(|d| d.node_type == "ac_plan_input").unwrap();
        let ac_planner = definitions.iter().find(|d| d.node_type == "ac_planner").unwrap();
        
        // AC planner inputs should match AC input outputs
        assert_eq!(ac_planner.inputs.len(), 5);
        assert_eq!(ac_input.outputs.len(), 5);
        
        // Verify all output IDs from ac_input match input IDs in ac_planner
        for output in &ac_input.outputs {
            let matching_input = ac_planner.inputs.iter().find(|i| i.id == output.id);
            assert!(
                matching_input.is_some(),
                "Expected planner to have input '{}' matching output from plan_input",
                output.id
            );
        }
    }
    
    #[test]
    fn test_node_definitions_have_categories() {
        let definitions = nodes::get_all_node_definitions();
        
        // Verify categories are assigned appropriately
        for def in &definitions {
            match def.node_type.as_str() {
                "on_evaluate" | "execute_plan" => {
                    assert_eq!(def.category, "System", "System nodes should be in 'System' category");
                }
                "ac_plan_input" | "ac_planner" | "ac_plan_result" => {
                    assert_eq!(def.category, "AC Controller", "AC nodes should be in 'AC Controller' category");
                }
                "logic_and" | "logic_or" | "logic_nand" | "logic_if" => {
                    assert_eq!(def.category, "Logic", "Logic nodes should be in 'Logic' category");
                }
                "primitive_float" | "primitive_integer" | "primitive_boolean" => {
                    assert_eq!(def.category, "Primitives", "Primitive nodes should be in 'Primitives' category");
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
}
