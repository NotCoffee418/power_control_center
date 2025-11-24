use super::node_system::{Node, NodeDefinition, NodeOutput, ValueType};

/// Float primitive node - provides a user-editable float value
/// Has a textbox for user input with client-side validation (colored red if not a float, defaults to 0)
pub struct FloatNode;

impl Node for FloatNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "primitive_float",
            "Float",
            "A floating-point number input. Enter a decimal value in the textbox.",
            "Primitives",
            vec![], // No inputs - this is a source node
            vec![
                NodeOutput::new(
                    "value",
                    "Value",
                    "The user-specified float value",
                    ValueType::Float,
                ),
            ],
        )
    }
}

/// Integer primitive node - provides a user-editable integer value
/// Has a textbox for user input with client-side validation (colored red if not an integer, defaults to 0)
pub struct IntegerNode;

impl Node for IntegerNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "primitive_integer",
            "Integer",
            "An integer number input. Enter a whole number in the textbox.",
            "Primitives",
            vec![], // No inputs - this is a source node
            vec![
                NodeOutput::new(
                    "value",
                    "Value",
                    "The user-specified integer value",
                    ValueType::Integer,
                ),
            ],
        )
    }
}

/// Boolean primitive node - provides a user-toggleable boolean value
/// Has a checkbox for user input
pub struct BooleanNode;

impl Node for BooleanNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "primitive_boolean",
            "Boolean",
            "A boolean value input. Toggle the checkbox to set true or false.",
            "Primitives",
            vec![], // No inputs - this is a source node
            vec![
                NodeOutput::new(
                    "value",
                    "Value",
                    "The user-specified boolean value",
                    ValueType::Boolean,
                ),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float_node_definition() {
        let def = FloatNode::definition();
        
        assert_eq!(def.node_type, "primitive_float");
        assert_eq!(def.name, "Float");
        assert_eq!(def.category, "Primitives");
        assert_eq!(def.inputs.len(), 0); // Source node has no inputs
        assert_eq!(def.outputs.len(), 1); // Single float output
        
        // Verify output type
        assert_eq!(def.outputs[0].id, "value");
        assert_eq!(def.outputs[0].value_type, ValueType::Float);
    }

    #[test]
    fn test_integer_node_definition() {
        let def = IntegerNode::definition();
        
        assert_eq!(def.node_type, "primitive_integer");
        assert_eq!(def.name, "Integer");
        assert_eq!(def.category, "Primitives");
        assert_eq!(def.inputs.len(), 0); // Source node has no inputs
        assert_eq!(def.outputs.len(), 1); // Single integer output
        
        // Verify output type
        assert_eq!(def.outputs[0].id, "value");
        assert_eq!(def.outputs[0].value_type, ValueType::Integer);
    }

    #[test]
    fn test_boolean_node_definition() {
        let def = BooleanNode::definition();
        
        assert_eq!(def.node_type, "primitive_boolean");
        assert_eq!(def.name, "Boolean");
        assert_eq!(def.category, "Primitives");
        assert_eq!(def.inputs.len(), 0); // Source node has no inputs
        assert_eq!(def.outputs.len(), 1); // Single boolean output
        
        // Verify output type
        assert_eq!(def.outputs[0].id, "value");
        assert_eq!(def.outputs[0].value_type, ValueType::Boolean);
    }

    #[test]
    fn test_primitive_nodes_serializable() {
        let definitions = vec![
            FloatNode::definition(),
            IntegerNode::definition(),
            BooleanNode::definition(),
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
    fn test_primitive_nodes_have_no_required_inputs() {
        let definitions = vec![
            FloatNode::definition(),
            IntegerNode::definition(),
            BooleanNode::definition(),
        ];
        
        for def in definitions {
            // Primitives are source nodes with user input, no inputs from other nodes
            assert_eq!(def.inputs.len(), 0, "{} should have no inputs", def.name);
        }
    }
}
