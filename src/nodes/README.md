# Node System

This module provides a flexible node-based system for defining data flow and logic components, particularly for the AC control planner.

## Overview

The node system allows defining reusable components with typed inputs and outputs that can be:
- Serialized to JSON for frontend visualization
- Connected together to form data flow graphs
- Used to document and understand the AC planning logic

## Architecture

### Core Components

1. **NodeDefinition** - Describes a node type with its inputs, outputs, name, and category
2. **NodeInput** - Defines an input port with type, label, and description
3. **NodeOutput** - Defines an output port with type, label, and description
4. **ValueType** - Type system for node values (Float, Integer, Boolean, String, Enum, Object)
5. **Node trait** - Trait for types that can provide their node definition

### Example Nodes

The module includes three example nodes that represent the AC planning system:

1. **AcPlanInputNode** - Source node that provides input data:
   - current_indoor_temp (Float)
   - solar_production (Integer)
   - user_is_home (Boolean)
   - current_outdoor_temp (Float)
   - avg_next_12h_outdoor_temp (Float)

2. **AcPlannerNode** - Logic node that processes inputs:
   - Takes all 5 inputs from AcPlanInputNode
   - Outputs a PlanResult object
   - Represents the `get_plan()` function logic

3. **AcPlanResultNode** - Output node that breaks down the result:
   - Takes PlanResult as input
   - Outputs mode (Enum: Colder/Warmer/Off/NoChange)
   - Outputs intensity (Enum: Low/Medium/High)
   - Outputs cause/reason (String)

## Usage

### Getting All Node Definitions

```rust
use crate::nodes;

let definitions = nodes::get_all_node_definitions();
// Returns Vec<NodeDefinition> with all available node types
```

### Creating a Custom Node

```rust
use crate::nodes::{Node, NodeDefinition, NodeInput, NodeOutput, ValueType};

pub struct MyCustomNode;

impl Node for MyCustomNode {
    fn definition() -> NodeDefinition {
        NodeDefinition::new(
            "my_custom_node",
            "My Custom Node",
            "Does something interesting",
            "Custom Category",
            vec![
                NodeInput::new(
                    "input1",
                    "Input 1",
                    "First input",
                    ValueType::Float,
                    true, // required
                ),
            ],
            vec![
                NodeOutput::new(
                    "output1",
                    "Output 1",
                    "Result output",
                    ValueType::Float,
                ),
            ],
        )
    }
}
```

### Frontend Integration

The node definitions are exposed via the `/api/nodes/definitions` endpoint and can be used by the frontend node editor to:
- Display available node types
- Show input/output ports with correct types
- Validate connections between nodes
- Generate UI for node configuration

## API Endpoint

**GET /api/nodes/definitions**

Returns all available node definitions in JSON format:

```json
{
  "success": true,
  "data": [
    {
      "node_type": "ac_plan_input",
      "name": "AC Plan Input",
      "description": "Provides all input data needed for AC planning decisions",
      "category": "AC Controller",
      "inputs": [],
      "outputs": [
        {
          "id": "current_indoor_temp",
          "label": "Indoor Temperature",
          "description": "Current indoor temperature in Celsius",
          "value_type": {
            "type": "Float"
          }
        }
        // ... more outputs
      ]
    }
    // ... more node definitions
  ]
}
```

## Testing

The module includes comprehensive tests:
- Unit tests for each component (in node_system.rs and ac_planner_nodes.rs)
- Integration tests verifying the complete flow (in integration_test.rs)
- Serialization/deserialization tests
- Validation that node connections are compatible

Run tests with:
```bash
cargo test nodes::
```
