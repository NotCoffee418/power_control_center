use super::node_system::{Node, NodeDefinition, NodeInput, NodeOutput, ValueType};

/// AND logic node - outputs true only when all inputs are true
/// Has dynamic number of boolean input pins (minimum 2)
pub struct AndNode;

impl Node for AndNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "logic_and",
            "AND",
            "Outputs true only when ALL inputs are true. Add or remove input pins with + and - buttons.",
            "Logic",
            vec![
                NodeInput::new(
                    "input_1",
                    "Input 1",
                    "First boolean input",
                    ValueType::Boolean,
                    true,
                ),
                NodeInput::new(
                    "input_2",
                    "Input 2",
                    "Second boolean input",
                    ValueType::Boolean,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "result",
                    "Result",
                    "True only when all inputs are true",
                    ValueType::Boolean,
                ),
            ],
        )
    }
}

/// OR logic node - outputs true when at least one input is true
/// Has dynamic number of boolean input pins (minimum 2)
pub struct OrNode;

impl Node for OrNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "logic_or",
            "OR",
            "Outputs true when ANY input is true. Add or remove input pins with + and - buttons.",
            "Logic",
            vec![
                NodeInput::new(
                    "input_1",
                    "Input 1",
                    "First boolean input",
                    ValueType::Boolean,
                    true,
                ),
                NodeInput::new(
                    "input_2",
                    "Input 2",
                    "Second boolean input",
                    ValueType::Boolean,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "result",
                    "Result",
                    "True when at least one input is true",
                    ValueType::Boolean,
                ),
            ],
        )
    }
}

/// NAND logic node - outputs false only when all inputs are true
/// Has dynamic number of boolean input pins (minimum 2)
pub struct NandNode;

impl Node for NandNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "logic_nand",
            "NAND",
            "Outputs false only when ALL inputs are true (inverse of AND). Add or remove input pins with + and - buttons.",
            "Logic",
            vec![
                NodeInput::new(
                    "input_1",
                    "Input 1",
                    "First boolean input",
                    ValueType::Boolean,
                    true,
                ),
                NodeInput::new(
                    "input_2",
                    "Input 2",
                    "Second boolean input",
                    ValueType::Boolean,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "result",
                    "Result",
                    "False only when all inputs are true",
                    ValueType::Boolean,
                ),
            ],
        )
    }
}

/// If node - routes execution based on boolean condition
/// Input: one boolean, Output: two execution paths (true/false)
pub struct IfNode;

impl Node for IfNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "logic_if",
            "If",
            "Routes based on boolean condition. One output fires for true, the other for false.",
            "Logic",
            vec![
                NodeInput::new(
                    "condition",
                    "Condition",
                    "The boolean condition to evaluate",
                    ValueType::Boolean,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "true",
                    "True",
                    "Output when condition is true",
                    ValueType::Boolean,
                ),
                NodeOutput::new(
                    "false",
                    "False",
                    "Output when condition is false",
                    ValueType::Boolean,
                ),
            ],
        )
    }
}

/// NOT logic node - inverts a boolean value
/// Takes one boolean input and outputs its inverse
pub struct NotNode;

impl Node for NotNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "logic_not",
            "NOT",
            "Inverts a boolean value. True becomes false, false becomes true.",
            "Logic",
            vec![
                NodeInput::new(
                    "input",
                    "Input",
                    "Boolean value to invert",
                    ValueType::Boolean,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "result",
                    "Result",
                    "Inverted boolean value",
                    ValueType::Boolean,
                ),
            ],
        )
    }
}

/// Equals logic node - compares two values for equality
/// Takes two inputs of the same type and outputs true if they are equal
/// Uses Any type for flexible type matching - the frontend handles type constraints
pub struct EqualsNode;

impl Node for EqualsNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "logic_equals",
            "Equals",
            "Compares two values for equality. Outputs true if both inputs have the same value. When one input is connected, the other input's type constraint matches that input's type.",
            "Logic",
            vec![
                NodeInput::new(
                    "input_a",
                    "Input A",
                    "First value to compare",
                    ValueType::Any,
                    true,
                ),
                NodeInput::new(
                    "input_b",
                    "Input B",
                    "Second value to compare",
                    ValueType::Any,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "result",
                    "Result",
                    "True if both inputs are equal",
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
    fn test_and_node_definition() {
        let def = AndNode::definition();
        
        assert_eq!(def.node_type, "logic_and");
        assert_eq!(def.name, "AND");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 2); // Minimum 2 inputs
        assert_eq!(def.outputs.len(), 1); // Single boolean output
        
        // Verify input types
        for input in &def.inputs {
            assert_eq!(input.value_type, ValueType::Boolean);
            assert!(input.required);
        }
        
        // Verify output type
        assert_eq!(def.outputs[0].id, "result");
        assert_eq!(def.outputs[0].value_type, ValueType::Boolean);
    }

    #[test]
    fn test_or_node_definition() {
        let def = OrNode::definition();
        
        assert_eq!(def.node_type, "logic_or");
        assert_eq!(def.name, "OR");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 2); // Minimum 2 inputs
        assert_eq!(def.outputs.len(), 1); // Single boolean output
        
        // Verify input types
        for input in &def.inputs {
            assert_eq!(input.value_type, ValueType::Boolean);
            assert!(input.required);
        }
        
        // Verify output type
        assert_eq!(def.outputs[0].id, "result");
        assert_eq!(def.outputs[0].value_type, ValueType::Boolean);
    }

    #[test]
    fn test_nand_node_definition() {
        let def = NandNode::definition();
        
        assert_eq!(def.node_type, "logic_nand");
        assert_eq!(def.name, "NAND");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 2); // Minimum 2 inputs
        assert_eq!(def.outputs.len(), 1); // Single boolean output
        
        // Verify input types
        for input in &def.inputs {
            assert_eq!(input.value_type, ValueType::Boolean);
            assert!(input.required);
        }
        
        // Verify output type
        assert_eq!(def.outputs[0].id, "result");
        assert_eq!(def.outputs[0].value_type, ValueType::Boolean);
    }

    #[test]
    fn test_if_node_definition() {
        let def = IfNode::definition();
        
        assert_eq!(def.node_type, "logic_if");
        assert_eq!(def.name, "If");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 1); // Single boolean input
        assert_eq!(def.outputs.len(), 2); // Two outputs: true and false
        
        // Verify input
        assert_eq!(def.inputs[0].id, "condition");
        assert_eq!(def.inputs[0].value_type, ValueType::Boolean);
        assert!(def.inputs[0].required);
        
        // Verify outputs
        let output_ids: Vec<&str> = def.outputs.iter().map(|o| o.id.as_str()).collect();
        assert!(output_ids.contains(&"true"));
        assert!(output_ids.contains(&"false"));
        
        for output in &def.outputs {
            assert_eq!(output.value_type, ValueType::Boolean);
        }
    }

    #[test]
    fn test_not_node_definition() {
        let def = NotNode::definition();
        
        assert_eq!(def.node_type, "logic_not");
        assert_eq!(def.name, "NOT");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 1); // Single boolean input
        assert_eq!(def.outputs.len(), 1); // Single boolean output
        
        // Verify input
        assert_eq!(def.inputs[0].id, "input");
        assert_eq!(def.inputs[0].value_type, ValueType::Boolean);
        assert!(def.inputs[0].required);
        
        // Verify output
        assert_eq!(def.outputs[0].id, "result");
        assert_eq!(def.outputs[0].value_type, ValueType::Boolean);
    }

    #[test]
    fn test_equals_node_definition() {
        let def = EqualsNode::definition();
        
        assert_eq!(def.node_type, "logic_equals");
        assert_eq!(def.name, "Equals");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 2); // Two inputs
        assert_eq!(def.outputs.len(), 1); // Single boolean output
        
        // Verify inputs are Any type for dynamic matching
        let input_a = def.inputs.iter().find(|i| i.id == "input_a").unwrap();
        assert_eq!(input_a.value_type, ValueType::Any);
        assert!(input_a.required);
        
        let input_b = def.inputs.iter().find(|i| i.id == "input_b").unwrap();
        assert_eq!(input_b.value_type, ValueType::Any);
        assert!(input_b.required);
        
        // Verify output is boolean
        assert_eq!(def.outputs[0].id, "result");
        assert_eq!(def.outputs[0].value_type, ValueType::Boolean);
    }

    #[test]
    fn test_logical_nodes_serializable() {
        let definitions = vec![
            AndNode::definition(),
            OrNode::definition(),
            NandNode::definition(),
            IfNode::definition(),
            NotNode::definition(),
            EqualsNode::definition(),
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
