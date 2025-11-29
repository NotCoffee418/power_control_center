use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use serde::{Serialize, Deserialize};

use crate::{
    db,
    types::ApiResponse,
};

/// ID for a new unsaved nodeset (not yet in database)
const NEW_NODESET_ID: i64 = -1;
/// ID for the default nodeset that cannot be modified or deleted
const DEFAULT_NODESET_ID: i64 = 0;

/// Node type for the Start node
const NODE_TYPE_START: &str = "flow_start";
/// Node type for the Execute Action node
const NODE_TYPE_EXECUTE_ACTION: &str = "flow_execute_action";

/// Result of nodeset validation
#[derive(Debug)]
pub struct NodesetValidationResult {
    pub is_valid: bool,
    pub start_count: usize,
    pub execute_count: usize,
    pub errors: Vec<String>,
}

/// Validates that a nodeset has exactly one Start node and exactly one Execute Action node
/// Returns a validation result with counts and any errors
pub fn validate_nodeset(nodes: &[serde_json::Value]) -> NodesetValidationResult {
    let mut start_count = 0;
    let mut execute_count = 0;

    for node in nodes {
        // Node type is stored in data.definition.node_type
        if let Some(node_type) = node
            .get("data")
            .and_then(|d| d.get("definition"))
            .and_then(|def| def.get("node_type"))
            .and_then(|nt| nt.as_str())
        {
            match node_type {
                NODE_TYPE_START => start_count += 1,
                NODE_TYPE_EXECUTE_ACTION => execute_count += 1,
                _ => {}
            }
        }
    }

    let mut errors = Vec::new();
    
    if start_count == 0 {
        errors.push("Profile must have exactly one Start node (found 0)".to_string());
    } else if start_count > 1 {
        errors.push(format!("Profile must have exactly one Start node (found {})", start_count));
    }
    
    if execute_count == 0 {
        errors.push("Profile must have exactly one Execute Action node (found 0)".to_string());
    } else if execute_count > 1 {
        errors.push(format!("Profile must have exactly one Execute Action node (found {})", execute_count));
    }

    NodesetValidationResult {
        is_valid: start_count == 1 && execute_count == 1,
        start_count,
        execute_count,
        errors,
    }
}

pub fn nodes_routes() -> Router {
    Router::new()
        // Legacy endpoint for backwards compatibility - returns active nodeset configuration
        .route("/configuration", get(get_node_configuration))
        // Nodeset management endpoints
        .route("/nodesets", get(list_nodesets))
        .route("/nodesets", post(create_nodeset))
        .route("/nodesets/active", get(get_active_nodeset))
        .route("/nodesets/active/:id", put(set_active_nodeset))
        .route("/nodesets/:id", get(get_nodeset))
        .route("/nodesets/:id", put(update_nodeset))
        .route("/nodesets/:id", delete(delete_nodeset))
        .route("/definitions", get(get_node_definitions))
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct NodeConfiguration {
    pub nodes: Vec<serde_json::Value>,
    pub edges: Vec<serde_json::Value>,
}

/// Nodeset with id and name
#[derive(Serialize, Deserialize, Clone)]
pub struct Nodeset {
    pub id: i64,
    pub name: String,
    pub nodes: Vec<serde_json::Value>,
    pub edges: Vec<serde_json::Value>,
}

/// Nodeset summary for list view
#[derive(Serialize, Deserialize, Clone)]
pub struct NodesetSummary {
    pub id: i64,
    pub name: String,
}

/// Request for creating a new nodeset
#[derive(Serialize, Deserialize)]
pub struct CreateNodesetRequest {
    pub name: String,
    pub nodes: Vec<serde_json::Value>,
    pub edges: Vec<serde_json::Value>,
}

/// Request for updating a nodeset
#[derive(Serialize, Deserialize)]
pub struct UpdateNodesetRequest {
    pub name: Option<String>,
    pub nodes: Vec<serde_json::Value>,
    pub edges: Vec<serde_json::Value>,
}

/// GET /api/nodes/configuration
/// Returns the current active nodeset configuration (backwards compatibility)
async fn get_node_configuration() -> Response {
    let pool = db::get_pool().await;
    
    // Get the active nodeset id
    let active_id = match get_active_nodeset_id(pool).await {
        Ok(id) => id,
        Err(e) => {
            log::error!("Failed to get active nodeset id: {}", e);
            let response = ApiResponse::<()>::error("Failed to get active nodeset");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };
    
    // Fetch the nodeset
    let result = sqlx::query_as::<_, (String,)>(
        "SELECT node_json FROM nodesets WHERE id = ?"
    )
    .bind(active_id)
    .fetch_optional(pool)
    .await;
    
    match result {
        Ok(Some(record)) => {
            match serde_json::from_str::<NodeConfiguration>(&record.0) {
                Ok(config) => {
                    let response = ApiResponse::success(config);
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => {
                    log::error!("Failed to parse node configuration: {}", e);
                    let response = ApiResponse::<()>::error("Failed to parse node configuration");
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
                }
            }
        }
        Ok(None) => {
            let response = ApiResponse::success(NodeConfiguration::default());
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to fetch node configuration: {}", e);
            let response = ApiResponse::<()>::error("Failed to fetch node configuration");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// GET /api/nodes/nodesets
/// Returns a list of all nodesets
async fn list_nodesets() -> Response {
    let pool = db::get_pool().await;
    
    let result = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, name FROM nodesets ORDER BY id"
    )
    .fetch_all(pool)
    .await;
    
    match result {
        Ok(rows) => {
            let nodesets: Vec<NodesetSummary> = rows
                .into_iter()
                .map(|(id, name)| NodesetSummary { id, name })
                .collect();
            let response = ApiResponse::success(nodesets);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to list nodesets: {}", e);
            let response = ApiResponse::<()>::error("Failed to list nodesets");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// GET /api/nodes/nodesets/:id
/// Returns a specific nodeset by id
async fn get_nodeset(Path(id): Path<i64>) -> Response {
    let pool = db::get_pool().await;
    
    let result = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, node_json FROM nodesets WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await;
    
    match result {
        Ok(Some((id, name, node_json))) => {
            match serde_json::from_str::<NodeConfiguration>(&node_json) {
                Ok(config) => {
                    let nodeset = Nodeset {
                        id,
                        name,
                        nodes: config.nodes,
                        edges: config.edges,
                    };
                    let response = ApiResponse::success(nodeset);
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => {
                    log::error!("Failed to parse nodeset configuration: {}", e);
                    let response = ApiResponse::<()>::error("Failed to parse nodeset configuration");
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
                }
            }
        }
        Ok(None) => {
            let response = ApiResponse::<()>::error("Nodeset not found");
            (StatusCode::NOT_FOUND, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to fetch nodeset: {}", e);
            let response = ApiResponse::<()>::error("Failed to fetch nodeset");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// POST /api/nodes/nodesets
/// Creates a new nodeset
async fn create_nodeset(Json(request): Json<CreateNodesetRequest>) -> Response {
    let pool = db::get_pool().await;
    
    // Validate name is not empty
    if request.name.trim().is_empty() {
        let response = ApiResponse::<()>::error("Nodeset name cannot be empty");
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }
    
    // Serialize the configuration
    let config = NodeConfiguration {
        nodes: request.nodes.clone(),
        edges: request.edges.clone(),
    };
    let json_str = match serde_json::to_string(&config) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to serialize nodeset configuration: {}", e);
            let response = ApiResponse::<()>::error("Failed to serialize nodeset configuration");
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };
    
    // Insert the new nodeset
    let result = sqlx::query(
        "INSERT INTO nodesets (name, node_json) VALUES (?, ?)"
    )
    .bind(&request.name)
    .bind(&json_str)
    .execute(pool)
    .await;
    
    match result {
        Ok(query_result) => {
            let new_id = query_result.last_insert_rowid();
            log::info!("Nodeset created with id {}", new_id);
            let nodeset = Nodeset {
                id: new_id,
                name: request.name,
                nodes: request.nodes,
                edges: request.edges,
            };
            let response = ApiResponse::success(nodeset);
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to create nodeset: {}", e);
            let response = ApiResponse::<()>::error("Failed to create nodeset");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// PUT /api/nodes/nodesets/:id
/// Updates an existing nodeset
async fn update_nodeset(Path(id): Path<i64>, Json(request): Json<UpdateNodesetRequest>) -> Response {
    let pool = db::get_pool().await;
    
    // Prevent modifying the default nodeset
    if id == DEFAULT_NODESET_ID {
        let response = ApiResponse::<()>::error("Cannot modify the default nodeset. Please create a new profile instead.");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }
    
    // Check if nodeset exists
    let exists = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM nodesets WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await;
    
    match exists {
        Ok(None) => {
            let response = ApiResponse::<()>::error("Nodeset not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
        Err(e) => {
            log::error!("Failed to check nodeset existence: {}", e);
            let response = ApiResponse::<()>::error("Failed to update nodeset");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
        Ok(Some(_)) => {}
    }
    
    // Serialize the configuration
    let config = NodeConfiguration {
        nodes: request.nodes.clone(),
        edges: request.edges.clone(),
    };
    let json_str = match serde_json::to_string(&config) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to serialize nodeset configuration: {}", e);
            let response = ApiResponse::<()>::error("Failed to serialize nodeset configuration");
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    };
    
    // Build update query based on whether name is provided
    let result = if let Some(ref name) = request.name {
        if name.trim().is_empty() {
            let response = ApiResponse::<()>::error("Nodeset name cannot be empty");
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
        sqlx::query(
            "UPDATE nodesets SET name = ?, node_json = ? WHERE id = ?"
        )
        .bind(name)
        .bind(&json_str)
        .bind(id)
        .execute(pool)
        .await
    } else {
        sqlx::query(
            "UPDATE nodesets SET node_json = ? WHERE id = ?"
        )
        .bind(&json_str)
        .bind(id)
        .execute(pool)
        .await
    };
    
    match result {
        Ok(_) => {
            log::info!("Nodeset {} updated", id);
            // Fetch the updated nodeset to return
            let updated = sqlx::query_as::<_, (i64, String, String)>(
                "SELECT id, name, node_json FROM nodesets WHERE id = ?"
            )
            .bind(id)
            .fetch_one(pool)
            .await;
            
            match updated {
                Ok((id, name, _)) => {
                    let nodeset = Nodeset {
                        id,
                        name,
                        nodes: request.nodes,
                        edges: request.edges,
                    };
                    let response = ApiResponse::success(nodeset);
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => {
                    log::error!("Failed to fetch updated nodeset: {}", e);
                    let response = ApiResponse::<()>::error("Nodeset updated but failed to retrieve");
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
                }
            }
        }
        Err(e) => {
            log::error!("Failed to update nodeset: {}", e);
            let response = ApiResponse::<()>::error("Failed to update nodeset");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// DELETE /api/nodes/nodesets/:id
/// Deletes a nodeset
async fn delete_nodeset(Path(id): Path<i64>) -> Response {
    let pool = db::get_pool().await;
    
    // Prevent deleting the default nodeset
    if id == DEFAULT_NODESET_ID {
        let response = ApiResponse::<()>::error("Cannot delete the default nodeset");
        return (StatusCode::FORBIDDEN, Json(response)).into_response();
    }
    
    // Check if this is the active nodeset
    let active_id = match get_active_nodeset_id(pool).await {
        Ok(aid) => aid,
        Err(e) => {
            log::error!("Failed to get active nodeset id: {}", e);
            let response = ApiResponse::<()>::error("Failed to delete nodeset");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };
    
    if id == active_id {
        let response = ApiResponse::<()>::error("Cannot delete the currently active nodeset. Please activate a different nodeset first.");
        return (StatusCode::CONFLICT, Json(response)).into_response();
    }
    
    let result = sqlx::query(
        "DELETE FROM nodesets WHERE id = ?"
    )
    .bind(id)
    .execute(pool)
    .await;
    
    match result {
        Ok(query_result) => {
            if query_result.rows_affected() == 0 {
                let response = ApiResponse::<()>::error("Nodeset not found");
                (StatusCode::NOT_FOUND, Json(response)).into_response()
            } else {
                log::info!("Nodeset {} deleted", id);
                let response = ApiResponse::success("Nodeset deleted");
                (StatusCode::OK, Json(response)).into_response()
            }
        }
        Err(e) => {
            log::error!("Failed to delete nodeset: {}", e);
            let response = ApiResponse::<()>::error("Failed to delete nodeset");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// GET /api/nodes/nodesets/active
/// Returns the currently active nodeset with full details
async fn get_active_nodeset() -> Response {
    let pool = db::get_pool().await;
    
    let active_id = match get_active_nodeset_id(pool).await {
        Ok(id) => id,
        Err(e) => {
            log::error!("Failed to get active nodeset id: {}", e);
            let response = ApiResponse::<()>::error("Failed to get active nodeset");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };
    
    let result = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, node_json FROM nodesets WHERE id = ?"
    )
    .bind(active_id)
    .fetch_optional(pool)
    .await;
    
    match result {
        Ok(Some((id, name, node_json))) => {
            match serde_json::from_str::<NodeConfiguration>(&node_json) {
                Ok(config) => {
                    let nodeset = Nodeset {
                        id,
                        name,
                        nodes: config.nodes,
                        edges: config.edges,
                    };
                    let response = ApiResponse::success(nodeset);
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => {
                    log::error!("Failed to parse active nodeset configuration: {}", e);
                    let response = ApiResponse::<()>::error("Failed to parse active nodeset configuration");
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
                }
            }
        }
        Ok(None) => {
            // Active nodeset not found - return default empty
            let nodeset = Nodeset {
                id: NEW_NODESET_ID,
                name: "New".to_string(),
                nodes: vec![],
                edges: vec![],
            };
            let response = ApiResponse::success(nodeset);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to fetch active nodeset: {}", e);
            let response = ApiResponse::<()>::error("Failed to fetch active nodeset");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// PUT /api/nodes/nodesets/active/:id
/// Sets the active nodeset
async fn set_active_nodeset(Path(id): Path<i64>) -> Response {
    let pool = db::get_pool().await;
    
    // Check if nodeset exists (unless id is NEW_NODESET_ID for new unsaved nodeset)
    if id != NEW_NODESET_ID {
        // Fetch nodeset to validate it
        let result = sqlx::query_as::<_, (String,)>(
            "SELECT node_json FROM nodesets WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await;
        
        match result {
            Ok(None) => {
                let response = ApiResponse::<()>::error("Nodeset not found");
                return (StatusCode::NOT_FOUND, Json(response)).into_response();
            }
            Err(e) => {
                log::error!("Failed to check nodeset existence: {}", e);
                let response = ApiResponse::<()>::error("Failed to set active nodeset");
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
            Ok(Some((node_json,))) => {
                // Parse and validate the nodeset
                match serde_json::from_str::<NodeConfiguration>(&node_json) {
                    Ok(config) => {
                        let validation = validate_nodeset(&config.nodes);
                        if !validation.is_valid {
                            let error_message = validation.errors.join("; ");
                            let response = ApiResponse::<()>::error(format!("Invalid profile: {}", error_message));
                            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to parse nodeset configuration: {}", e);
                        let response = ApiResponse::<()>::error("Failed to parse nodeset configuration");
                        return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
                    }
                }
            }
        }
    }
    
    // Update the active nodeset setting
    let result = sqlx::query(
        "INSERT INTO settings (setting_key, setting_value) VALUES ('active_nodeset', ?)
         ON CONFLICT(setting_key) DO UPDATE SET setting_value = excluded.setting_value"
    )
    .bind(id.to_string())
    .execute(pool)
    .await;
    
    match result {
        Ok(_) => {
            log::info!("Active nodeset set to {}", id);
            let response = ApiResponse::success(id);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            log::error!("Failed to set active nodeset: {}", e);
            let response = ApiResponse::<()>::error("Failed to set active nodeset");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Helper function to get the active nodeset id
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
        None => Ok(DEFAULT_NODESET_ID) // Default to default nodeset if not set
    }
}

/// GET /api/nodes/definitions
/// Returns all available node type definitions
async fn get_node_definitions() -> Response {
    let mut definitions = crate::nodes::get_all_node_definitions();
    
    // Load cause reasons from database and inject them into the CauseReasonNode definition
    match db::cause_reasons::get_all(false).await {
        Ok(cause_reasons) => {
            // Find the cause_reason node definition and update its enum values
            if let Some(cause_reason_def) = definitions.iter_mut().find(|d| d.node_type == "cause_reason") {
                if let Some(output) = cause_reason_def.outputs.first_mut() {
                    // Replace the enum values with ID-label pairs from the database
                    let options: Vec<crate::nodes::EnumOption> = cause_reasons.iter().map(|cr| {
                        crate::nodes::EnumOption {
                            id: cr.id.to_string(),
                            label: cr.label.clone(),
                        }
                    }).collect();
                    output.value_type = crate::nodes::ValueType::EnumWithIds(options);
                }
            }
        }
        Err(e) => {
            log::warn!("Failed to load cause reasons from database, using defaults: {}", e);
            // Continue with default values from the hardcoded definition
        }
    }
    
    let response = ApiResponse::success(definitions);
    (StatusCode::OK, Json(response)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_node(node_type: &str) -> serde_json::Value {
        json!({
            "id": format!("{}-1", node_type),
            "type": "custom",
            "position": { "x": 0, "y": 0 },
            "data": {
                "label": "Test Node",
                "definition": {
                    "node_type": node_type,
                    "name": "Test",
                    "description": "Test node",
                    "category": "System",
                    "inputs": [],
                    "outputs": [],
                    "color": "#000000"
                }
            }
        })
    }

    #[test]
    fn test_validate_nodeset_empty() {
        let nodes: Vec<serde_json::Value> = vec![];
        let result = validate_nodeset(&nodes);
        
        assert!(!result.is_valid);
        assert_eq!(result.start_count, 0);
        assert_eq!(result.execute_count, 0);
        assert_eq!(result.errors.len(), 2);
        assert!(result.errors.iter().any(|e| e.contains("Start node")));
        assert!(result.errors.iter().any(|e| e.contains("Execute Action node")));
    }

    #[test]
    fn test_validate_nodeset_valid() {
        let nodes = vec![
            create_node(NODE_TYPE_START),
            create_node(NODE_TYPE_EXECUTE_ACTION),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(result.is_valid);
        assert_eq!(result.start_count, 1);
        assert_eq!(result.execute_count, 1);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_nodeset_missing_start() {
        let nodes = vec![
            create_node(NODE_TYPE_EXECUTE_ACTION),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(!result.is_valid);
        assert_eq!(result.start_count, 0);
        assert_eq!(result.execute_count, 1);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("Start node"));
    }

    #[test]
    fn test_validate_nodeset_missing_execute() {
        let nodes = vec![
            create_node(NODE_TYPE_START),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(!result.is_valid);
        assert_eq!(result.start_count, 1);
        assert_eq!(result.execute_count, 0);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("Execute Action node"));
    }

    #[test]
    fn test_validate_nodeset_multiple_starts() {
        let nodes = vec![
            create_node(NODE_TYPE_START),
            create_node(NODE_TYPE_START),
            create_node(NODE_TYPE_EXECUTE_ACTION),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(!result.is_valid);
        assert_eq!(result.start_count, 2);
        assert_eq!(result.execute_count, 1);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("found 2"));
    }

    #[test]
    fn test_validate_nodeset_multiple_executes() {
        let nodes = vec![
            create_node(NODE_TYPE_START),
            create_node(NODE_TYPE_EXECUTE_ACTION),
            create_node(NODE_TYPE_EXECUTE_ACTION),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(!result.is_valid);
        assert_eq!(result.start_count, 1);
        assert_eq!(result.execute_count, 2);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("found 2"));
    }

    #[test]
    fn test_validate_nodeset_with_other_nodes() {
        // Valid nodeset with additional nodes that should be ignored
        let nodes = vec![
            create_node(NODE_TYPE_START),
            create_node(NODE_TYPE_EXECUTE_ACTION),
            create_node("logic_and"),
            create_node("primitive_float"),
            create_node("device"),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(result.is_valid);
        assert_eq!(result.start_count, 1);
        assert_eq!(result.execute_count, 1);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_nodeset_invalid_structure() {
        // Node without proper structure should be safely ignored
        let nodes = vec![
            json!({}),
            json!({ "data": {} }),
            json!({ "data": { "definition": {} } }),
            create_node(NODE_TYPE_START),
            create_node(NODE_TYPE_EXECUTE_ACTION),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(result.is_valid);
        assert_eq!(result.start_count, 1);
        assert_eq!(result.execute_count, 1);
    }
}
