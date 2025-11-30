use super::node_system::{Node, NodeDefinition, NodeInput, NodeOutput, ValueType};

/// Add node - adds two numeric values
/// 
/// Type Constraint Behavior (handled by frontend):
/// - Initial state: Both inputs accept Float or Integer types
/// - When one input is connected to a specific type (Float or Integer), 
///   the other input's type constraint is updated to match that type
/// - When all pins are disconnected, constraints reset to accept Float/Integer
/// - Output type matches the input types (Float if any input is Float, Integer if both are Integer)
pub struct AddNode;

impl Node for AddNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "math_add",
            "Add",
            "Adds two numeric values. When one input is connected, the other input's type constraint matches that input's type (Float or Integer). Resets to Float/Integer constraint when all pins are disconnected.",
            "Logic",
            vec![
                NodeInput::new(
                    "input_a",
                    "A",
                    "First numeric value (accepts Float or Integer)",
                    ValueType::Any, // Uses Any for flexible type matching, but frontend constrains to Float/Integer
                    true,
                ),
                NodeInput::new(
                    "input_b",
                    "B",
                    "Second numeric value (accepts Float or Integer)",
                    ValueType::Any, // Uses Any for flexible type matching, but frontend constrains to Float/Integer
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "result",
                    "Result",
                    "The sum of the two input values",
                    ValueType::Any, // Output type matches input types
                ),
            ],
        )
    }
}

/// Subtract node - subtracts the second value from the first
/// 
/// Type Constraint Behavior (handled by frontend):
/// - Initial state: Both inputs accept Float or Integer types
/// - When one input is connected to a specific type (Float or Integer), 
///   the other input's type constraint is updated to match that type
/// - When all pins are disconnected, constraints reset to accept Float/Integer
/// - Output type matches the input types (Float if any input is Float, Integer if both are Integer)
pub struct SubtractNode;

impl Node for SubtractNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "math_subtract",
            "Subtract",
            "Subtracts the second value from the first. When one input is connected, the other input's type constraint matches that input's type (Float or Integer). Resets to Float/Integer constraint when all pins are disconnected.",
            "Logic",
            vec![
                NodeInput::new(
                    "input_a",
                    "A",
                    "First numeric value (accepts Float or Integer)",
                    ValueType::Any, // Uses Any for flexible type matching, but frontend constrains to Float/Integer
                    true,
                ),
                NodeInput::new(
                    "input_b",
                    "B",
                    "Second numeric value to subtract (accepts Float or Integer)",
                    ValueType::Any, // Uses Any for flexible type matching, but frontend constrains to Float/Integer
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "result",
                    "Result",
                    "The difference (A - B)",
                    ValueType::Any, // Output type matches input types
                ),
            ],
        )
    }
}

/// Multiply node - multiplies two float values
/// 
/// This node only accepts Float type inputs to ensure precision in multiplication.
pub struct MultiplyNode;

impl Node for MultiplyNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "math_multiply",
            "Multiply",
            "Multiplies two float values. Only accepts Float type inputs.",
            "Logic",
            vec![
                NodeInput::new(
                    "input_a",
                    "A",
                    "First float value",
                    ValueType::Float,
                    true,
                ),
                NodeInput::new(
                    "input_b",
                    "B",
                    "Second float value",
                    ValueType::Float,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "result",
                    "Result",
                    "The product of the two input values",
                    ValueType::Float,
                ),
            ],
        )
    }
}

/// Divide node - divides the first value by the second
/// 
/// This node only accepts Float type inputs to ensure precision in division.
/// Note: Division by zero will return 0.0 (handled by execution engine).
pub struct DivideNode;

impl Node for DivideNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "math_divide",
            "Divide",
            "Divides the first value by the second. Only accepts Float type inputs. Division by zero returns 0.",
            "Logic",
            vec![
                NodeInput::new(
                    "input_a",
                    "A",
                    "Dividend (value to divide)",
                    ValueType::Float,
                    true,
                ),
                NodeInput::new(
                    "input_b",
                    "B",
                    "Divisor (value to divide by)",
                    ValueType::Float,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "result",
                    "Result",
                    "The quotient (A / B)",
                    ValueType::Float,
                ),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node_definition() {
        let def = AddNode::definition();
        
        assert_eq!(def.node_type, "math_add");
        assert_eq!(def.name, "Add");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 2);
        assert_eq!(def.outputs.len(), 1);
        
        // Verify inputs are Any type for flexible matching between Float/Integer
        let input_a = def.inputs.iter().find(|i| i.id == "input_a").unwrap();
        assert_eq!(input_a.value_type, ValueType::Any);
        assert!(input_a.required);
        
        let input_b = def.inputs.iter().find(|i| i.id == "input_b").unwrap();
        assert_eq!(input_b.value_type, ValueType::Any);
        assert!(input_b.required);
        
        // Verify output is Any type (matches input types)
        assert_eq!(def.outputs[0].id, "result");
        assert_eq!(def.outputs[0].value_type, ValueType::Any);
    }

    #[test]
    fn test_subtract_node_definition() {
        let def = SubtractNode::definition();
        
        assert_eq!(def.node_type, "math_subtract");
        assert_eq!(def.name, "Subtract");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 2);
        assert_eq!(def.outputs.len(), 1);
        
        // Verify inputs are Any type for flexible matching between Float/Integer
        let input_a = def.inputs.iter().find(|i| i.id == "input_a").unwrap();
        assert_eq!(input_a.value_type, ValueType::Any);
        assert!(input_a.required);
        
        let input_b = def.inputs.iter().find(|i| i.id == "input_b").unwrap();
        assert_eq!(input_b.value_type, ValueType::Any);
        assert!(input_b.required);
        
        // Verify output is Any type (matches input types)
        assert_eq!(def.outputs[0].id, "result");
        assert_eq!(def.outputs[0].value_type, ValueType::Any);
    }

    #[test]
    fn test_multiply_node_definition() {
        let def = MultiplyNode::definition();
        
        assert_eq!(def.node_type, "math_multiply");
        assert_eq!(def.name, "Multiply");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 2);
        assert_eq!(def.outputs.len(), 1);
        
        // Verify inputs are Float type only
        let input_a = def.inputs.iter().find(|i| i.id == "input_a").unwrap();
        assert_eq!(input_a.value_type, ValueType::Float);
        assert!(input_a.required);
        
        let input_b = def.inputs.iter().find(|i| i.id == "input_b").unwrap();
        assert_eq!(input_b.value_type, ValueType::Float);
        assert!(input_b.required);
        
        // Verify output is Float type
        assert_eq!(def.outputs[0].id, "result");
        assert_eq!(def.outputs[0].value_type, ValueType::Float);
    }

    #[test]
    fn test_divide_node_definition() {
        let def = DivideNode::definition();
        
        assert_eq!(def.node_type, "math_divide");
        assert_eq!(def.name, "Divide");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 2);
        assert_eq!(def.outputs.len(), 1);
        
        // Verify inputs are Float type only
        let input_a = def.inputs.iter().find(|i| i.id == "input_a").unwrap();
        assert_eq!(input_a.value_type, ValueType::Float);
        assert!(input_a.required);
        
        let input_b = def.inputs.iter().find(|i| i.id == "input_b").unwrap();
        assert_eq!(input_b.value_type, ValueType::Float);
        assert!(input_b.required);
        
        // Verify output is Float type
        assert_eq!(def.outputs[0].id, "result");
        assert_eq!(def.outputs[0].value_type, ValueType::Float);
    }

    #[test]
    fn test_math_nodes_serializable() {
        let definitions = vec![
            AddNode::definition(),
            SubtractNode::definition(),
            MultiplyNode::definition(),
            DivideNode::definition(),
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
