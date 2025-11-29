use serde::{Serialize, Deserialize};

/// Represents a key-value pair for enums that need ID-based tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnumOption {
    pub id: String,
    pub label: String,
}

/// Represents a type of value that can flow through nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum ValueType {
    /// Floating point number (temperature, etc.)
    Float,
    /// Integer number (watts, etc.)
    Integer,
    /// Boolean value
    Boolean,
    /// String value
    String,
    /// Enumeration with possible values (simple string labels)
    Enum(Vec<String>),
    /// Enumeration with ID-label pairs (for tracking by ID even when labels change)
    EnumWithIds(Vec<EnumOption>),
    /// Structured object (for complex types like PlanResult)
    Object,
    /// Any type - used for dynamic type matching (e.g., Equals node, Evaluate Number node)
    Any,
}

impl ValueType {
    /// Get the display color for this value type
    pub fn get_color(&self) -> &'static str {
        match self {
            ValueType::Float => "#FF6B6B",      // Red for floats (temperatures, etc.)
            ValueType::Integer => "#4ECDC4",    // Teal for integers (watts, etc.)
            ValueType::Boolean => "#95E1D3",    // Light green for booleans
            ValueType::String => "#FFA07A",     // Light salmon for strings
            ValueType::Enum(_) => "#C7A5E0",    // Purple for enums
            ValueType::EnumWithIds(_) => "#C7A5E0", // Purple for enums with IDs
            ValueType::Object => "#FFD93D",     // Yellow for complex objects
            ValueType::Any => "#AAAAAA",        // Gray for any type (dynamic matching)
        }
    }
}

/// Represents an input port on a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInput {
    /// Unique identifier for this input
    pub id: String,
    /// Human-readable label
    pub label: String,
    /// Description of what this input expects
    pub description: String,
    /// Type of value this input accepts
    pub value_type: ValueType,
    /// Whether this input is required
    pub required: bool,
    /// Color for visual representation
    pub color: String,
}

impl NodeInput {
    pub fn new(id: &str, label: &str, description: &str, value_type: ValueType, required: bool) -> Self {
        let color = value_type.get_color().to_string();
        Self {
            id: id.to_string(),
            label: label.to_string(),
            description: description.to_string(),
            value_type,
            required,
            color,
        }
    }
}

/// Represents an output port on a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutput {
    /// Unique identifier for this output
    pub id: String,
    /// Human-readable label
    pub label: String,
    /// Description of what this output provides
    pub description: String,
    /// Type of value this output produces
    pub value_type: ValueType,
    /// Color for visual representation
    pub color: String,
}

impl NodeOutput {
    pub fn new(id: &str, label: &str, description: &str, value_type: ValueType) -> Self {
        let color = value_type.get_color().to_string();
        Self {
            id: id.to_string(),
            label: label.to_string(),
            description: description.to_string(),
            value_type,
            color,
        }
    }
}

/// Defines the structure and capabilities of a node type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDefinition {
    /// Unique type identifier for this node
    pub node_type: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this node does
    pub description: String,
    /// Category for organization
    pub category: String,
    /// List of input ports
    pub inputs: Vec<NodeInput>,
    /// List of output ports
    pub outputs: Vec<NodeOutput>,
    /// Color for the node background
    pub color: String,
}

impl NodeDefinition {
    pub fn new(
        node_type: &str,
        name: &str,
        description: &str,
        category: &str,
        inputs: Vec<NodeInput>,
        outputs: Vec<NodeOutput>,
    ) -> Self {
        // Assign color based on category
        let color = Self::get_category_color(category);
        Self {
            node_type: node_type.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            category: category.to_string(),
            inputs,
            outputs,
            color: color.to_string(),
        }
    }

    /// Get the color for a category
    fn get_category_color(category: &str) -> &'static str {
        match category {
            "System" => "#4CAF50",         // Green for system nodes
            "AC Controller" => "#2196F3",  // Blue for AC controller nodes
            "Logic" => "#9C27B0",          // Purple for logic nodes
            "Primitives" => "#FF9800",     // Orange for primitive nodes
            "Sensors" => "#00BCD4",        // Cyan for sensor nodes
            "Enums" => "#E91E63",          // Pink for enum nodes
            _ => "#757575",                // Gray for others
        }
    }
}

/// Trait for nodes that can provide their definition
pub trait Node {
    /// Get the definition of this node type
    fn definition() -> NodeDefinition;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_input_creation() {
        let input = NodeInput::new(
            "temp",
            "Temperature",
            "Current temperature in Celsius",
            ValueType::Float,
            true,
        );
        
        assert_eq!(input.id, "temp");
        assert_eq!(input.label, "Temperature");
        assert!(input.required);
    }

    #[test]
    fn test_node_output_creation() {
        let output = NodeOutput::new(
            "result",
            "Result",
            "Computed result",
            ValueType::Integer,
        );
        
        assert_eq!(output.id, "result");
        assert_eq!(output.label, "Result");
    }

    #[test]
    fn test_node_definition_creation() {
        let inputs = vec![
            NodeInput::new("in1", "Input 1", "First input", ValueType::Float, true),
        ];
        let outputs = vec![
            NodeOutput::new("out1", "Output 1", "First output", ValueType::Float),
        ];
        
        let def = NodeDefinition::new(
            "test_node",
            "Test Node",
            "A test node",
            "Testing",
            inputs,
            outputs,
        );
        
        assert_eq!(def.node_type, "test_node");
        assert_eq!(def.name, "Test Node");
        assert_eq!(def.category, "Testing");
        assert_eq!(def.inputs.len(), 1);
        assert_eq!(def.outputs.len(), 1);
    }

    #[test]
    fn test_value_type_serialization() {
        let vt = ValueType::Enum(vec!["Option1".to_string(), "Option2".to_string()]);
        let json = serde_json::to_string(&vt).unwrap();
        let deserialized: ValueType = serde_json::from_str(&json).unwrap();
        
        assert_eq!(vt, deserialized);
    }
}
