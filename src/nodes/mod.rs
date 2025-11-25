mod node_system;
mod ac_planner_nodes;
mod logical_nodes;
mod primitive_nodes;
mod enum_nodes;
mod sensor_nodes;
#[cfg(test)]
mod integration_test;

pub use node_system::{Node, NodeInput, NodeOutput, NodeDefinition, ValueType};
pub use ac_planner_nodes::{
    ExecutePlanNode,
    AcPlanInputNode,
    ClassicPlannerNode,
    AcPlanResultNode,
};
pub use logical_nodes::{AndNode, OrNode, NandNode, IfNode, NotNode, EqualsNode};
pub use primitive_nodes::{FloatNode, IntegerNode, BooleanNode};
pub use enum_nodes::{DeviceNode, CurrentlyEvaluatingDeviceNode, IntensityNode};
pub use sensor_nodes::PirDetectionNode;

/// Get all available node definitions for the frontend
pub fn get_all_node_definitions() -> Vec<NodeDefinition> {
    vec![
        // System nodes
        ExecutePlanNode::definition(),
        CurrentlyEvaluatingDeviceNode::definition(),
        // AC Controller nodes
        AcPlanInputNode::definition(),
        ClassicPlannerNode::definition(),
        AcPlanResultNode::definition(),
        // Sensor nodes
        PirDetectionNode::definition(),
        // Logic nodes
        AndNode::definition(),
        OrNode::definition(),
        NandNode::definition(),
        IfNode::definition(),
        NotNode::definition(),
        EqualsNode::definition(),
        // Primitive nodes
        FloatNode::definition(),
        IntegerNode::definition(),
        BooleanNode::definition(),
        // Enum nodes
        DeviceNode::definition(),
        IntensityNode::definition(),
    ]
}
