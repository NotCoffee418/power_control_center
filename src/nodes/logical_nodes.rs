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
/// Input: one execution flow and one boolean condition
/// Output: two execution paths (true/false)
/// The execution flows to either the True or False output based on the condition
pub struct IfNode;

impl Node for IfNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "logic_if",
            "If",
            "Routes execution based on boolean condition. When triggered, evaluates the condition and fires either the True or False execution output.",
            "Logic",
            vec![
                NodeInput::new(
                    "exec_in",
                    "▶",
                    "Execution flow input - triggers this node to evaluate",
                    ValueType::Execution,
                    true,
                ),
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
                    "exec_true",
                    "True ▶",
                    "Execution output when condition is true",
                    ValueType::Execution,
                ),
                NodeOutput::new(
                    "exec_false",
                    "False ▶",
                    "Execution output when condition is false",
                    ValueType::Execution,
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

/// Evaluate Number node - compares two numeric values
/// Has a built-in combobox for selecting comparison operator and two numeric inputs
/// 
/// Type Constraint Behavior (handled by frontend):
/// - Initial state: Both inputs accept Float or Integer types
/// - When one input is connected to a specific type (Float or Integer), 
///   the other input's type constraint is updated to match that type
/// - When all pins are disconnected, constraints reset to accept Float/Integer
/// 
/// The operator selection (>, >=, ==, <=, <) is a built-in combobox on the node,
/// not an input pin. This differs from the Equals node which can compare any types,
/// while this node is specifically designed for numeric comparisons.
pub struct EvaluateNumberNode;

impl Node for EvaluateNumberNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "logic_evaluate_number",
            "Evaluate Number",
            "Compares two numeric values using the selected comparison operator. When one input is connected, the other input's type constraint matches that input's type (Float or Integer). Resets to Float/Integer constraint when all pins are disconnected.",
            "Logic",
            vec![
                NodeInput::new(
                    "input_a",
                    "Input A",
                    "First numeric value to compare (accepts Float or Integer)",
                    ValueType::Any, // Uses Any for flexible type matching, but frontend constrains to Float/Integer
                    true,
                ),
                NodeInput::new(
                    "input_b",
                    "Input B",
                    "Second numeric value to compare (accepts Float or Integer)",
                    ValueType::Any, // Uses Any for flexible type matching, but frontend constrains to Float/Integer
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "result",
                    "Result",
                    "True if the comparison is satisfied",
                    ValueType::Boolean,
                ),
            ],
        )
    }
}

/// Branch node - selects between two values based on a boolean condition
/// Takes a boolean condition and two "Any" type inputs (True and False)
/// Outputs the value from the True input when condition is true, or False input otherwise
/// 
/// Type Constraint Behavior (handled by frontend):
/// - Initial state: All Any-type inputs and output accept any type
/// - When any input is connected to a specific type, all other Any-type
///   inputs and the output are constrained to match that type
/// - When all pins are disconnected, constraints reset to accept any type
pub struct BranchNode;

impl Node for BranchNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "logic_branch",
            "Branch",
            "Selects between two values based on a boolean condition. When condition is true, outputs the True input value; otherwise outputs the False input value. All value inputs and output are constrained to the same type.",
            "Logic",
            vec![
                NodeInput::new(
                    "condition",
                    "Condition",
                    "Boolean condition to evaluate",
                    ValueType::Boolean,
                    true,
                ),
                NodeInput::new(
                    "true_value",
                    "True",
                    "Value to output when condition is true",
                    ValueType::Any,
                    true,
                ),
                NodeInput::new(
                    "false_value",
                    "False",
                    "Value to output when condition is false",
                    ValueType::Any,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "result",
                    "Output",
                    "The selected value based on condition",
                    ValueType::Any,
                ),
            ],
        )
    }
}

/// Sequence node - executes outputs in order until one path completes
/// Has dynamic number of execution output pins (minimum 2)
/// When triggered, it evaluates each output path in order (Then 0, Then 1, etc.)
/// If a path leads to Execute Action or Do Nothing, the sequence stops there.
/// Otherwise, it continues to the next output.
pub struct SequenceNode;

impl Node for SequenceNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "logic_sequence",
            "Sequence",
            "Executes outputs in order. When triggered, evaluates each 'Then' output in sequence. If a path leads to Execute Action or Do Nothing, the sequence stops. Otherwise continues to the next output.",
            "Logic",
            vec![
                NodeInput::new(
                    "exec_in",
                    "▶",
                    "Execution flow input - triggers the sequence to start",
                    ValueType::Execution,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "then_0",
                    "Then 0 ▶",
                    "First execution path to try",
                    ValueType::Execution,
                ),
                NodeOutput::new(
                    "then_1",
                    "Then 1 ▶",
                    "Second execution path to try",
                    ValueType::Execution,
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
        assert_eq!(def.inputs.len(), 2); // exec_in and condition
        assert_eq!(def.outputs.len(), 2); // Two execution outputs: exec_true and exec_false
        
        // Verify exec_in input
        let exec_input = def.inputs.iter().find(|i| i.id == "exec_in").unwrap();
        assert_eq!(exec_input.value_type, ValueType::Execution);
        assert!(exec_input.required);
        
        // Verify condition input
        let condition_input = def.inputs.iter().find(|i| i.id == "condition").unwrap();
        assert_eq!(condition_input.value_type, ValueType::Boolean);
        assert!(condition_input.required);
        
        // Verify execution outputs
        let output_ids: Vec<&str> = def.outputs.iter().map(|o| o.id.as_str()).collect();
        assert!(output_ids.contains(&"exec_true"));
        assert!(output_ids.contains(&"exec_false"));
        
        for output in &def.outputs {
            assert_eq!(output.value_type, ValueType::Execution);
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
    fn test_evaluate_number_node_definition() {
        let def = EvaluateNumberNode::definition();
        
        assert_eq!(def.node_type, "logic_evaluate_number");
        assert_eq!(def.name, "Evaluate Number");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 2); // input_a, input_b (operator is a built-in combobox, not an input)
        assert_eq!(def.outputs.len(), 1); // Single boolean output
        
        // Verify inputs are Any type for flexible matching between Float/Integer
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
    fn test_branch_node_definition() {
        let def = BranchNode::definition();
        
        assert_eq!(def.node_type, "logic_branch");
        assert_eq!(def.name, "Branch");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 3); // condition, true_value, false_value
        assert_eq!(def.outputs.len(), 1); // Single output
        
        // Verify condition input is Boolean
        let condition = def.inputs.iter().find(|i| i.id == "condition").unwrap();
        assert_eq!(condition.value_type, ValueType::Boolean);
        assert!(condition.required);
        
        // Verify true_value and false_value inputs are Any type for dynamic matching
        let true_value = def.inputs.iter().find(|i| i.id == "true_value").unwrap();
        assert_eq!(true_value.value_type, ValueType::Any);
        assert!(true_value.required);
        
        let false_value = def.inputs.iter().find(|i| i.id == "false_value").unwrap();
        assert_eq!(false_value.value_type, ValueType::Any);
        assert!(false_value.required);
        
        // Verify output is Any type
        assert_eq!(def.outputs[0].id, "result");
        assert_eq!(def.outputs[0].value_type, ValueType::Any);
    }

    #[test]
    fn test_sequence_node_definition() {
        let def = SequenceNode::definition();
        
        assert_eq!(def.node_type, "logic_sequence");
        assert_eq!(def.name, "Sequence");
        assert_eq!(def.category, "Logic");
        assert_eq!(def.inputs.len(), 1); // exec_in
        assert_eq!(def.outputs.len(), 2); // Minimum 2 outputs: then_0, then_1
        
        // Verify exec_in input
        let exec_input = def.inputs.iter().find(|i| i.id == "exec_in").unwrap();
        assert_eq!(exec_input.value_type, ValueType::Execution);
        assert!(exec_input.required);
        
        // Verify outputs are execution type
        let output_ids: Vec<&str> = def.outputs.iter().map(|o| o.id.as_str()).collect();
        assert!(output_ids.contains(&"then_0"));
        assert!(output_ids.contains(&"then_1"));
        
        for output in &def.outputs {
            assert_eq!(output.value_type, ValueType::Execution);
        }
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
            EvaluateNumberNode::definition(),
            BranchNode::definition(),
            SequenceNode::definition(),
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
