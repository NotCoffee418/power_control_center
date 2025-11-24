mod node_system;
mod ac_planner_nodes;

pub use node_system::{Node, NodeInput, NodeOutput, NodeDefinition, ValueType};
pub use ac_planner_nodes::{
    AcPlanInputNode,
    AcPlannerNode,
    AcPlanResultNode,
};

/// Get all available node definitions for the frontend
pub fn get_all_node_definitions() -> Vec<NodeDefinition> {
    vec![
        AcPlanInputNode::definition(),
        AcPlannerNode::definition(),
        AcPlanResultNode::definition(),
    ]
}
