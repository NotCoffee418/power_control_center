mod node_system;
mod logical_nodes;
mod primitive_nodes;
mod enum_nodes;
mod sensor_nodes;
mod flow_nodes;
pub mod execution;
#[cfg(test)]
mod integration_test;

pub use node_system::{Node, NodeInput, NodeOutput, NodeDefinition, ValueType, EnumOption};
pub use logical_nodes::{AndNode, OrNode, NandNode, IfNode, NotNode, EqualsNode, EvaluateNumberNode, BranchNode};
pub use primitive_nodes::{FloatNode, IntegerNode, BooleanNode};
pub use enum_nodes::{DeviceNode, IntensityNode, CauseReasonNode, RequestModeNode, FanSpeedNode};
pub use sensor_nodes::PirDetectionNode;
pub use flow_nodes::{StartNode, ExecuteActionNode, DoNothingNode};
pub use execution::{NodesetExecutor, ExecutionInputs, ExecutionResult, ActionResult, RuntimeValue, validate_nodeset_for_execution};

/// Get all available node definitions for the frontend
pub fn get_all_node_definitions() -> Vec<NodeDefinition> {
    vec![
        // System/Flow nodes
        StartNode::definition(),
        ExecuteActionNode::definition(),
        DoNothingNode::definition(),
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
        BranchNode::definition(),
        // Primitive nodes
        FloatNode::definition(),
        IntegerNode::definition(),
        BooleanNode::definition(),
        // Enum nodes
        DeviceNode::definition(),
        IntensityNode::definition(),
        CauseReasonNode::definition(),
        RequestModeNode::definition(),
        FanSpeedNode::definition(),
    ]
}
