mod node_system;
mod ac_planner_nodes;
mod logical_nodes;
mod primitive_nodes;
#[cfg(test)]
mod integration_test;

pub use node_system::{Node, NodeInput, NodeOutput, NodeDefinition, ValueType};
pub use ac_planner_nodes::{
    OnEvaluateNode,
    ExecutePlanNode,
    AcPlanInputNode,
    AcPlannerNode,
    AcPlanResultNode,
};
pub use logical_nodes::{AndNode, OrNode, NandNode, IfNode};
pub use primitive_nodes::{FloatNode, IntegerNode, BooleanNode};

/// Get all available node definitions for the frontend
pub fn get_all_node_definitions() -> Vec<NodeDefinition> {
    vec![
        // System nodes
        OnEvaluateNode::definition(),
        ExecutePlanNode::definition(),
        // AC Controller nodes
        AcPlanInputNode::definition(),
        AcPlannerNode::definition(),
        AcPlanResultNode::definition(),
        // Logic nodes
        AndNode::definition(),
        OrNode::definition(),
        NandNode::definition(),
        IfNode::definition(),
        // Primitive nodes
        FloatNode::definition(),
        IntegerNode::definition(),
        BooleanNode::definition(),
    ]
}
