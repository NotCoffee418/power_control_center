mod node_system;
mod logical_nodes;
mod primitive_nodes;
mod enum_nodes;
mod sensor_nodes;
#[cfg(test)]
mod integration_test;

pub use node_system::{Node, NodeInput, NodeOutput, NodeDefinition, ValueType};
pub use logical_nodes::{AndNode, OrNode, NandNode, IfNode, NotNode, EqualsNode, EvaluateNumberNode};
pub use primitive_nodes::{FloatNode, IntegerNode, BooleanNode};
pub use enum_nodes::{DeviceNode, CurrentlyEvaluatingDeviceNode, IntensityNode, CauseReasonNode, RequestModeNode};
pub use sensor_nodes::PirDetectionNode;

/// Get all available node definitions for the frontend
pub fn get_all_node_definitions() -> Vec<NodeDefinition> {
    vec![
        // System nodes
        CurrentlyEvaluatingDeviceNode::definition(),
        // Sensor nodes
        PirDetectionNode::definition(),
        // Logic nodes
        AndNode::definition(),
        OrNode::definition(),
        NandNode::definition(),
        IfNode::definition(),
        NotNode::definition(),
        EqualsNode::definition(),
        EvaluateNumberNode::definition(),
        // Primitive nodes
        FloatNode::definition(),
        IntegerNode::definition(),
        BooleanNode::definition(),
        // Enum nodes
        DeviceNode::definition(),
        IntensityNode::definition(),
        CauseReasonNode::definition(),
        RequestModeNode::definition(),
    ]
}
