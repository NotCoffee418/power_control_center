use super::node_system::{Node, NodeDefinition, NodeInput, NodeOutput, ValueType};

/// OnEvaluate trigger node - fires every 5 minutes to trigger AC evaluation
/// This is a special trigger node that should be placed by default and cannot be deleted
pub struct OnEvaluateNode;

impl Node for OnEvaluateNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "on_evaluate",
            "On Evaluate Event",
            "Triggers every 5 minutes to evaluate and potentially adjust AC settings. This node cannot be deleted but can be moved.",
            "System",
            vec![], // No inputs - this is a trigger node
            vec![
                NodeOutput::new(
                    "trigger",
                    "Trigger",
                    "Fires when evaluation cycle begins",
                    ValueType::Boolean,
                ),
            ],
        )
    }
}

/// ExecutePlan node - executes an AC plan
/// Takes a plan result and sends it to the AC system
pub struct ExecutePlanNode;

impl Node for ExecutePlanNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "execute_plan",
            "Execute Plan",
            "Executes the AC control plan by sending commands to the AC system",
            "System",
            vec![
                NodeInput::new(
                    "plan",
                    "AC Plan",
                    "The complete AC plan to execute (from AC Plan Result node)",
                    ValueType::Object,
                    true,
                ),
            ],
            vec![], // No outputs - this is a terminal action node
        )
    }
}

/// Node representing the input data for AC planning
pub struct AcPlanInputNode;

impl Node for AcPlanInputNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "ac_plan_input",
            "AC Plan Input",
            "Provides all input data needed for AC planning decisions",
            "AC Controller",
            vec![], // No inputs - this is a source node
            vec![
                NodeOutput::new(
                    "current_indoor_temp",
                    "Indoor Temperature",
                    "Current indoor temperature in Celsius",
                    ValueType::Float,
                ),
                NodeOutput::new(
                    "solar_production",
                    "Solar Production",
                    "Current solar power production in Watts",
                    ValueType::Integer,
                ),
                NodeOutput::new(
                    "user_is_home",
                    "User Is Home",
                    "Whether the user is currently home and awake",
                    ValueType::Boolean,
                ),
                NodeOutput::new(
                    "current_outdoor_temp",
                    "Outdoor Temperature",
                    "Current outdoor temperature in Celsius",
                    ValueType::Float,
                ),
                NodeOutput::new(
                    "avg_next_12h_outdoor_temp",
                    "12h Avg Outdoor Temp",
                    "Average outdoor temperature forecast for next 12 hours in Celsius",
                    ValueType::Float,
                ),
            ],
        )
    }
}

/// Node representing the AC planning logic
pub struct AcPlannerNode;

impl Node for AcPlannerNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "ac_planner",
            "AC Planner Logic",
            "Determines the desired AC mode and intensity based on inputs. Includes Ice Exception, temperature checks, solar power evaluation, and user presence logic.",
            "AC Controller",
            vec![
                NodeInput::new(
                    "current_indoor_temp",
                    "Indoor Temperature",
                    "Current indoor temperature in Celsius",
                    ValueType::Float,
                    true,
                ),
                NodeInput::new(
                    "solar_production",
                    "Solar Production",
                    "Current solar power production in Watts",
                    ValueType::Integer,
                    true,
                ),
                NodeInput::new(
                    "user_is_home",
                    "User Is Home",
                    "Whether the user is currently home and awake",
                    ValueType::Boolean,
                    true,
                ),
                NodeInput::new(
                    "current_outdoor_temp",
                    "Outdoor Temperature",
                    "Current outdoor temperature in Celsius",
                    ValueType::Float,
                    true,
                ),
                NodeInput::new(
                    "avg_next_12h_outdoor_temp",
                    "12h Avg Outdoor Temp",
                    "Average outdoor temperature forecast for next 12 hours in Celsius",
                    ValueType::Float,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "plan_result",
                    "Plan Result",
                    "The computed AC plan with mode and cause",
                    ValueType::Object,
                ),
            ],
        )
    }
}

/// Node representing the output of AC planning
/// 
/// Note: This node flattens the RequestMode enum structure for visual programming convenience.
/// In Rust, RequestMode::Colder and RequestMode::Warmer contain an Intensity variant,
/// but for node-based visual programming, separating mode and intensity into distinct outputs
/// is more intuitive and allows independent connections. The intensity output is only meaningful
/// when mode is Colder or Warmer.
pub struct AcPlanResultNode;

impl Node for AcPlanResultNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "ac_plan_result",
            "AC Plan Result",
            "Final AC control plan with mode (Colder/Warmer/Off/NoChange), intensity (Low/Medium/High), and the reason for the decision",
            "AC Controller",
            vec![
                NodeInput::new(
                    "plan_result",
                    "Plan Result",
                    "The AC plan result object",
                    ValueType::Object,
                    true,
                ),
            ],
            vec![
                NodeOutput::new(
                    "mode",
                    "Mode",
                    "Desired AC mode: Colder, Warmer, Off, or NoChange",
                    ValueType::Enum(vec![
                        "Colder".to_string(),
                        "Warmer".to_string(),
                        "Off".to_string(),
                        "NoChange".to_string(),
                    ]),
                ),
                NodeOutput::new(
                    "intensity",
                    "Intensity",
                    "Desired intensity level: Low, Medium, or High (only meaningful for Colder/Warmer modes)",
                    ValueType::Enum(vec![
                        "Low".to_string(),
                        "Medium".to_string(),
                        "High".to_string(),
                    ]),
                ),
                NodeOutput::new(
                    "cause",
                    "Cause/Reason",
                    "The reason for this decision (e.g., IceException, ExcessiveSolarPower, etc.)",
                    ValueType::String,
                ),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_on_evaluate_node_definition() {
        let def = OnEvaluateNode::definition();
        
        assert_eq!(def.node_type, "on_evaluate");
        assert_eq!(def.name, "On Evaluate Event");
        assert_eq!(def.category, "System");
        assert_eq!(def.inputs.len(), 0); // Trigger node has no inputs
        assert_eq!(def.outputs.len(), 1); // One trigger output
        
        // Verify output
        assert_eq!(def.outputs[0].id, "trigger");
    }

    #[test]
    fn test_execute_plan_node_definition() {
        let def = ExecutePlanNode::definition();
        
        assert_eq!(def.node_type, "execute_plan");
        assert_eq!(def.name, "Execute Plan");
        assert_eq!(def.category, "System");
        assert_eq!(def.inputs.len(), 1); // One input: plan
        assert_eq!(def.outputs.len(), 0); // Terminal node has no outputs
        
        // Verify input
        assert_eq!(def.inputs[0].id, "plan");
        assert!(def.inputs[0].required);
    }

    #[test]
    fn test_ac_plan_input_node_definition() {
        let def = AcPlanInputNode::definition();
        
        assert_eq!(def.node_type, "ac_plan_input");
        assert_eq!(def.name, "AC Plan Input");
        assert_eq!(def.category, "AC Controller");
        assert_eq!(def.inputs.len(), 0); // Source node has no inputs
        assert_eq!(def.outputs.len(), 5); // Five data outputs
        
        // Verify output IDs
        let output_ids: Vec<&str> = def.outputs.iter().map(|o| o.id.as_str()).collect();
        assert!(output_ids.contains(&"current_indoor_temp"));
        assert!(output_ids.contains(&"solar_production"));
        assert!(output_ids.contains(&"user_is_home"));
    }

    #[test]
    fn test_ac_planner_node_definition() {
        let def = AcPlannerNode::definition();
        
        assert_eq!(def.node_type, "ac_planner");
        assert_eq!(def.name, "AC Planner Logic");
        assert_eq!(def.category, "AC Controller");
        assert_eq!(def.inputs.len(), 5); // Five inputs matching PlanInput struct
        assert_eq!(def.outputs.len(), 1); // One output: plan_result
        
        // All inputs should be required
        assert!(def.inputs.iter().all(|i| i.required));
    }

    #[test]
    fn test_ac_plan_result_node_definition() {
        let def = AcPlanResultNode::definition();
        
        assert_eq!(def.node_type, "ac_plan_result");
        assert_eq!(def.name, "AC Plan Result");
        assert_eq!(def.category, "AC Controller");
        assert_eq!(def.inputs.len(), 1); // One input: plan_result
        assert_eq!(def.outputs.len(), 3); // Three outputs: mode, intensity, cause
        
        // Verify output IDs
        let output_ids: Vec<&str> = def.outputs.iter().map(|o| o.id.as_str()).collect();
        assert!(output_ids.contains(&"mode"));
        assert!(output_ids.contains(&"intensity"));
        assert!(output_ids.contains(&"cause"));
    }

    #[test]
    fn test_mode_enum_values() {
        let def = AcPlanResultNode::definition();
        let mode_output = def.outputs.iter().find(|o| o.id == "mode").unwrap();
        
        match &mode_output.value_type {
            ValueType::Enum(values) => {
                assert_eq!(values.len(), 4);
                assert!(values.contains(&"Colder".to_string()));
                assert!(values.contains(&"Warmer".to_string()));
                assert!(values.contains(&"Off".to_string()));
                assert!(values.contains(&"NoChange".to_string()));
            }
            _ => panic!("Expected Enum type for mode output"),
        }
    }

    #[test]
    fn test_intensity_enum_values() {
        let def = AcPlanResultNode::definition();
        let intensity_output = def.outputs.iter().find(|o| o.id == "intensity").unwrap();
        
        match &intensity_output.value_type {
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
    fn test_node_definitions_are_serializable() {
        let definitions = vec![
            AcPlanInputNode::definition(),
            AcPlannerNode::definition(),
            AcPlanResultNode::definition(),
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
