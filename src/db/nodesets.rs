//! Database access for nodeset configuration
//! 
//! This module provides functions to query nodeset data from the database,
//! particularly for extracting configuration values like evaluate_every_minutes.

use crate::nodes::flow_nodes::MAX_EVALUATE_EVERY_MINUTES;

/// Default evaluation interval in minutes if not specified in the nodeset
/// This matches the original hardcoded 5-minute interval
const DEFAULT_EVALUATE_EVERY_MINUTES: i32 = 5;

/// ID for the default nodeset that is loaded when no other nodeset is active
const DEFAULT_NODESET_ID: i64 = 0;

/// Get the evaluate_every_minutes value from the active nodeset
/// 
/// This function reads the active nodeset from the database and extracts
/// the primitiveValue from the Start node, which contains the evaluation interval.
/// 
/// Returns the default value (5 minutes) if:
/// - No active nodeset is configured
/// - The nodeset cannot be parsed
/// - The Start node doesn't have a primitiveValue set
/// - The value is outside the valid range (1-1440)
pub async fn get_evaluate_every_minutes() -> i32 {
    let pool = crate::db::get_pool().await;
    
    // Get the active nodeset ID
    let active_id = match get_active_nodeset_id(pool).await {
        Ok(id) => id,
        Err(e) => {
            log::warn!("Failed to get active nodeset id: {}. Using default interval.", e);
            return DEFAULT_EVALUATE_EVERY_MINUTES;
        }
    };
    
    // Fetch the nodeset JSON
    let result = sqlx::query_as::<_, (String,)>(
        "SELECT node_json FROM nodesets WHERE id = ?"
    )
    .bind(active_id)
    .fetch_optional(pool)
    .await;
    
    match result {
        Ok(Some((node_json,))) => {
            extract_evaluate_minutes_from_json(&node_json)
        }
        Ok(None) => {
            log::debug!("No active nodeset found (id={}). Using default interval.", active_id);
            DEFAULT_EVALUATE_EVERY_MINUTES
        }
        Err(e) => {
            log::warn!("Failed to fetch active nodeset: {}. Using default interval.", e);
            DEFAULT_EVALUATE_EVERY_MINUTES
        }
    }
}

/// Helper function to get the active nodeset ID from the database
async fn get_active_nodeset_id(pool: &sqlx::SqlitePool) -> Result<i64, sqlx::Error> {
    let result = sqlx::query_as::<_, (String,)>(
        "SELECT setting_value FROM settings WHERE setting_key = 'active_nodeset'"
    )
    .fetch_optional(pool)
    .await?;
    
    match result {
        Some((value,)) => {
            value.parse::<i64>().map_err(|e| {
                log::error!("Failed to parse active_nodeset value '{}': {}", value, e);
                sqlx::Error::Decode(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid active_nodeset value"
                )))
            })
        }
        None => Ok(DEFAULT_NODESET_ID)
    }
}

/// Extract the evaluate_every_minutes value from nodeset JSON
/// Returns the default value if extraction fails
fn extract_evaluate_minutes_from_json(node_json: &str) -> i32 {
    // Parse the JSON
    let parsed: serde_json::Value = match serde_json::from_str(node_json) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Failed to parse nodeset JSON: {}. Using default interval.", e);
            return DEFAULT_EVALUATE_EVERY_MINUTES;
        }
    };
    
    // Get the nodes array
    let nodes = match parsed.get("nodes").and_then(|n| n.as_array()) {
        Some(n) => n,
        None => {
            log::debug!("No nodes array found in nodeset. Using default interval.");
            return DEFAULT_EVALUATE_EVERY_MINUTES;
        }
    };
    
    // Find the Start node
    for node in nodes {
        let is_start_node = node
            .get("data")
            .and_then(|d| d.get("definition"))
            .and_then(|def| def.get("node_type"))
            .and_then(|nt| nt.as_str())
            == Some("flow_start");
        
        if is_start_node {
            // Get the primitiveValue which stores evaluate_every_minutes
            if let Some(data) = node.get("data") {
                if let Some(value) = data.get("primitiveValue") {
                    if let Some(minutes) = value.as_i64() {
                        // Validate the value is within range
                        if minutes >= 1 && minutes <= MAX_EVALUATE_EVERY_MINUTES as i64 {
                            log::debug!("Using evaluate_every_minutes={} from active nodeset", minutes);
                            return minutes as i32;
                        } else {
                            log::warn!(
                                "evaluate_every_minutes value {} is out of range (1-{}). Using default.",
                                minutes, MAX_EVALUATE_EVERY_MINUTES
                            );
                            return DEFAULT_EVALUATE_EVERY_MINUTES;
                        }
                    }
                }
            }
            
            // Start node found but no primitiveValue set
            log::debug!("Start node found but no evaluate_every_minutes value set. Using default.");
            return DEFAULT_EVALUATE_EVERY_MINUTES;
        }
    }
    
    // No Start node found
    log::debug!("No Start node found in nodeset. Using default interval.");
    DEFAULT_EVALUATE_EVERY_MINUTES
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_evaluate_minutes_valid() {
        let json = r#"{
            "nodes": [
                {
                    "id": "flow_start-1",
                    "data": {
                        "primitiveValue": 10,
                        "definition": {
                            "node_type": "flow_start"
                        }
                    }
                }
            ],
            "edges": []
        }"#;
        
        assert_eq!(extract_evaluate_minutes_from_json(json), 10);
    }
    
    #[test]
    fn test_extract_evaluate_minutes_max_value() {
        let json = r#"{
            "nodes": [
                {
                    "id": "flow_start-1",
                    "data": {
                        "primitiveValue": 1440,
                        "definition": {
                            "node_type": "flow_start"
                        }
                    }
                }
            ],
            "edges": []
        }"#;
        
        assert_eq!(extract_evaluate_minutes_from_json(json), 1440);
    }
    
    #[test]
    fn test_extract_evaluate_minutes_exceeds_max() {
        let json = r#"{
            "nodes": [
                {
                    "id": "flow_start-1",
                    "data": {
                        "primitiveValue": 1500,
                        "definition": {
                            "node_type": "flow_start"
                        }
                    }
                }
            ],
            "edges": []
        }"#;
        
        assert_eq!(extract_evaluate_minutes_from_json(json), DEFAULT_EVALUATE_EVERY_MINUTES);
    }
    
    #[test]
    fn test_extract_evaluate_minutes_zero() {
        let json = r#"{
            "nodes": [
                {
                    "id": "flow_start-1",
                    "data": {
                        "primitiveValue": 0,
                        "definition": {
                            "node_type": "flow_start"
                        }
                    }
                }
            ],
            "edges": []
        }"#;
        
        assert_eq!(extract_evaluate_minutes_from_json(json), DEFAULT_EVALUATE_EVERY_MINUTES);
    }
    
    #[test]
    fn test_extract_evaluate_minutes_negative() {
        let json = r#"{
            "nodes": [
                {
                    "id": "flow_start-1",
                    "data": {
                        "primitiveValue": -5,
                        "definition": {
                            "node_type": "flow_start"
                        }
                    }
                }
            ],
            "edges": []
        }"#;
        
        assert_eq!(extract_evaluate_minutes_from_json(json), DEFAULT_EVALUATE_EVERY_MINUTES);
    }
    
    #[test]
    fn test_extract_evaluate_minutes_no_start_node() {
        let json = r#"{
            "nodes": [
                {
                    "id": "logic_and-1",
                    "data": {
                        "definition": {
                            "node_type": "logic_and"
                        }
                    }
                }
            ],
            "edges": []
        }"#;
        
        assert_eq!(extract_evaluate_minutes_from_json(json), DEFAULT_EVALUATE_EVERY_MINUTES);
    }
    
    #[test]
    fn test_extract_evaluate_minutes_no_primitive_value() {
        let json = r#"{
            "nodes": [
                {
                    "id": "flow_start-1",
                    "data": {
                        "definition": {
                            "node_type": "flow_start"
                        }
                    }
                }
            ],
            "edges": []
        }"#;
        
        assert_eq!(extract_evaluate_minutes_from_json(json), DEFAULT_EVALUATE_EVERY_MINUTES);
    }
    
    #[test]
    fn test_extract_evaluate_minutes_empty_nodes() {
        let json = r#"{
            "nodes": [],
            "edges": []
        }"#;
        
        assert_eq!(extract_evaluate_minutes_from_json(json), DEFAULT_EVALUATE_EVERY_MINUTES);
    }
    
    #[test]
    fn test_extract_evaluate_minutes_invalid_json() {
        let json = "not valid json";
        
        assert_eq!(extract_evaluate_minutes_from_json(json), DEFAULT_EVALUATE_EVERY_MINUTES);
    }
}
