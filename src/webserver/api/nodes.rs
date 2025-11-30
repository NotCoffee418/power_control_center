use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};

use crate::{
    db,
    nodes::{self, flow_nodes::MAX_EVALUATE_EVERY_MINUTES},
    types::ApiResponse,
};

/// ID for a new unsaved nodeset (not yet in database)
const NEW_NODESET_ID: i64 = -1;
/// ID for the default nodeset that cannot be modified or deleted
pub const DEFAULT_NODESET_ID: i64 = 0;

/// Node type for the Start node
const NODE_TYPE_START: &str = "flow_start";
/// Node type for the Execute Action node
const NODE_TYPE_EXECUTE_ACTION: &str = "flow_execute_action";
/// Node type for the Do Nothing node
const NODE_TYPE_DO_NOTHING: &str = "flow_do_nothing";
/// Node type for the Turn Off node
const NODE_TYPE_TURN_OFF: &str = "flow_turn_off";

/// Result of nodeset validation
#[derive(Debug)]
pub struct NodesetValidationResult {
    pub is_valid: bool,
    pub start_count: usize,
    pub terminal_count: usize,
    pub errors: Vec<String>,
}

/// Check if a node type is a terminal node (Execute Action, Do Nothing, or Turn Off)
fn is_terminal_node(node_type: &str) -> bool {
    matches!(node_type, NODE_TYPE_EXECUTE_ACTION | NODE_TYPE_DO_NOTHING | NODE_TYPE_TURN_OFF)
}

/// Validates that a nodeset has exactly one Start node and at least one terminal node
/// (Execute Action, Do Nothing, or Turn Off).
/// Also validates the evaluate_every_minutes value on the Start node.
/// Note: Disconnected nodes are allowed and will be treated as "Do Nothing" at runtime.
/// Returns a validation result with counts and any errors
pub fn validate_nodeset(nodes: &[serde_json::Value]) -> NodesetValidationResult {
    let mut start_count = 0;
    let mut terminal_count = 0;
    let mut errors = Vec::new();

    for node in nodes {
        // Node type is stored in data.definition.node_type
        if let Some(node_type) = node
            .get("data")
            .and_then(|d| d.get("definition"))
            .and_then(|def| def.get("node_type"))
            .and_then(|nt| nt.as_str())
        {
            if node_type == NODE_TYPE_START {
                start_count += 1;
                // Validate evaluate_every_minutes value
                if let Some(data) = node.get("data") {
                    if let Some(value) = data.get("primitiveValue") {
                        if let Some(minutes) = value.as_i64() {
                            if minutes < 1 {
                                errors.push(format!(
                                    "Evaluate Every Minutes must be at least 1 (found {})",
                                    minutes
                                ));
                            } else if minutes > MAX_EVALUATE_EVERY_MINUTES as i64 {
                                errors.push(format!(
                                    "Evaluate Every Minutes cannot exceed {} (24 hours), found {}",
                                    MAX_EVALUATE_EVERY_MINUTES, minutes
                                ));
                            }
                        }
                    }
                }
            } else if is_terminal_node(node_type) {
                terminal_count += 1;
            }
        }
    }
    
    if start_count == 0 {
        errors.push("Profile must have exactly one Start node (found 0)".to_string());
    } else if start_count > 1 {
        errors.push(format!("Profile must have exactly one Start node (found {})", start_count));
    }
    
    if terminal_count == 0 {
        errors.push("Profile must have at least one terminal node (Execute Action, Do Nothing, or Turn Off)".to_string());
    }

    NodesetValidationResult {
        is_valid: start_count == 1 && terminal_count >= 1 && errors.is_empty(),
        start_count,
        terminal_count,
        errors,
    }
}

/// Gets node definitions enriched with cause reasons from the database.
/// This ensures CauseReason nodes have their dropdown options populated.
async fn get_enriched_node_definitions() -> Vec<nodes::NodeDefinition> {
    let mut definitions = nodes::get_all_node_definitions();
    
    // Load cause reasons from database and inject them into node definitions
    if let Ok(cause_reasons) = db::cause_reasons::get_all(false).await {
        let options: Vec<nodes::EnumOption> = cause_reasons.iter().map(|cr| {
            nodes::EnumOption {
                id: cr.id.to_string(),
                label: cr.label.clone(),
            }
        }).collect();
        let cause_reason_type = nodes::ValueType::CauseReason(options);
        
        // Update the cause_reason node output
        if let Some(cause_reason_def) = definitions.iter_mut().find(|d| d.node_type == "cause_reason") {
            if let Some(output) = cause_reason_def.outputs.first_mut() {
                output.value_type = cause_reason_type.clone();
            }
        }
        
        // Update the flow_execute_action node's cause_reason input
        if let Some(execute_action_def) = definitions.iter_mut().find(|d| d.node_type == "flow_execute_action") {
            if let Some(cause_input) = execute_action_def.inputs.iter_mut().find(|i| i.id == "cause_reason") {
                cause_input.value_type = cause_reason_type.clone();
            }
        }
        
        // Update the flow_do_nothing node's cause_reason input
        if let Some(do_nothing_def) = definitions.iter_mut().find(|d| d.node_type == "flow_do_nothing") {
            if let Some(cause_input) = do_nothing_def.inputs.iter_mut().find(|i| i.id == "cause_reason") {
                cause_input.value_type = cause_reason_type;
            }
        }
    }
    
    definitions
}

/// Updates node definitions in a nodeset to match the current version.
/// This ensures that when a profile is loaded, all nodes have their definitions
/// updated to the latest version, preventing crashes due to outdated definitions.
/// 
/// The function:
/// - Preserves user data (position, enumValue, primitiveValue, dynamicInputs, comment, etc.)
/// - Updates the node definition to the current version (including enriched CauseReason options)
/// - Removes nodes whose type no longer exists (returns list of removed node IDs)
/// 
/// Returns tuple of (updated_nodes, removed_node_ids)
async fn update_node_definitions(nodes: Vec<serde_json::Value>) -> (Vec<serde_json::Value>, Vec<String>) {
    // Build a map of current node definitions by node_type (enriched with cause reasons)
    let current_definitions: HashMap<String, serde_json::Value> = get_enriched_node_definitions().await
        .into_iter()
        .filter_map(|def| {
            serde_json::to_value(&def)
                .ok()
                .map(|v| (def.node_type.clone(), v))
        })
        .collect();
    
    let mut updated_nodes = Vec::new();
    let mut removed_node_ids = Vec::new();
    
    for mut node in nodes {
        // Get the node type from the stored definition
        let node_type = node
            .get("data")
            .and_then(|d| d.get("definition"))
            .and_then(|def| def.get("node_type"))
            .and_then(|nt| nt.as_str())
            .map(|s| s.to_string());
        
        if let Some(node_type) = node_type {
            if let Some(current_def) = current_definitions.get(&node_type) {
                // Update the definition while preserving user data
                if let Some(data) = node.get_mut("data") {
                    if let Some(data_obj) = data.as_object_mut() {
                        data_obj.insert("definition".to_string(), current_def.clone());
                    }
                }
                updated_nodes.push(node);
            } else {
                // Node type no longer exists - record for removal
                if let Some(id) = node.get("id").and_then(|id| id.as_str()) {
                    log::warn!("Removing node '{}' with unknown type '{}'", id, node_type);
                    removed_node_ids.push(id.to_string());
                }
            }
        } else {
            // Node has no type - keep it but log a warning
            log::warn!("Node has no type defined, keeping as-is: {:?}", node.get("id"));
            updated_nodes.push(node);
        }
    }
    
    (updated_nodes, removed_node_ids)
}

/// Removes edges that reference removed nodes
fn remove_orphaned_edges(edges: Vec<serde_json::Value>, removed_node_ids: &[String]) -> Vec<serde_json::Value> {
    if removed_node_ids.is_empty() {
        return edges;
    }
    
    // Use HashSet for O(1) lookups
    let removed_set: HashSet<&str> = removed_node_ids.iter().map(|s| s.as_str()).collect();
    
    edges
        .into_iter()
        .filter(|edge| {
            let source = edge.get("source").and_then(|s| s.as_str()).unwrap_or("");
            let target = edge.get("target").and_then(|t| t.as_str()).unwrap_or("");
            
            !removed_set.contains(source) && !removed_set.contains(target)
        })
        .collect()
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
                    // Update node definitions to current version
                    let (updated_nodes, removed_node_ids) = update_node_definitions(config.nodes).await;
                    let updated_edges = remove_orphaned_edges(config.edges, &removed_node_ids);
                    
                    let updated_config = NodeConfiguration {
                        nodes: updated_nodes,
                        edges: updated_edges,
                    };
                    let response = ApiResponse::success(updated_config);
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
                    // Update node definitions to current version
                    let (updated_nodes, removed_node_ids) = update_node_definitions(config.nodes).await;
                    let updated_edges = remove_orphaned_edges(config.edges, &removed_node_ids);
                    
                    let nodeset = Nodeset {
                        id,
                        name,
                        nodes: updated_nodes,
                        edges: updated_edges,
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
    
    // Check if this is the active nodeset - if so, validate before allowing update
    let active_id = match get_active_nodeset_id(pool).await {
        Ok(aid) => aid,
        Err(e) => {
            log::error!("Failed to get active nodeset id: {}", e);
            let response = ApiResponse::<()>::error("Failed to update nodeset");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };
    
    if id == active_id {
        // Validate the nodeset since it's the active one
        let validation = validate_nodeset(&request.nodes);
        if !validation.is_valid {
            let error_message = validation.errors.join("; ");
            let response = ApiResponse::<()>::error(format!("Cannot save active profile with invalid configuration: {}", error_message));
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
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
                    // Update node definitions to current version
                    let (updated_nodes, removed_node_ids) = update_node_definitions(config.nodes).await;
                    let updated_edges = remove_orphaned_edges(config.edges, &removed_node_ids);
                    
                    let nodeset = Nodeset {
                        id,
                        name,
                        nodes: updated_nodes,
                        edges: updated_edges,
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
pub async fn get_active_nodeset_id(pool: &sqlx::SqlitePool) -> Result<i64, sqlx::Error> {
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
    let definitions = get_enriched_node_definitions().await;
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
        assert_eq!(result.terminal_count, 0);
        assert_eq!(result.errors.len(), 2);
        assert!(result.errors.iter().any(|e| e.contains("Start node")));
        assert!(result.errors.iter().any(|e| e.contains("terminal node")));
    }

    #[test]
    fn test_validate_nodeset_valid() {
        // Disconnected nodes are allowed - they will be treated as "Do Nothing" at runtime
        let nodes = vec![
            create_node(NODE_TYPE_START),
            create_node(NODE_TYPE_EXECUTE_ACTION),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(result.is_valid);
        assert_eq!(result.start_count, 1);
        assert_eq!(result.terminal_count, 1);
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
        assert_eq!(result.terminal_count, 1);
        assert!(result.errors.iter().any(|e| e.contains("Start node")));
    }

    #[test]
    fn test_validate_nodeset_missing_terminal() {
        let nodes = vec![
            create_node(NODE_TYPE_START),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(!result.is_valid);
        assert_eq!(result.start_count, 1);
        assert_eq!(result.terminal_count, 0);
        assert!(result.errors.iter().any(|e| e.contains("terminal node")));
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
        assert_eq!(result.terminal_count, 1);
        assert!(result.errors.iter().any(|e| e.contains("found 2")));
    }

    #[test]
    fn test_validate_nodeset_multiple_terminals() {
        // Multiple terminal nodes are allowed for flow control support
        let nodes = vec![
            create_node(NODE_TYPE_START),
            create_node(NODE_TYPE_EXECUTE_ACTION),
            create_node(NODE_TYPE_DO_NOTHING),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(result.is_valid);
        assert_eq!(result.start_count, 1);
        assert_eq!(result.terminal_count, 2);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_nodeset_with_do_nothing() {
        // Do Nothing should be a valid terminal node
        let nodes = vec![
            create_node(NODE_TYPE_START),
            create_node(NODE_TYPE_DO_NOTHING),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(result.is_valid);
        assert_eq!(result.start_count, 1);
        assert_eq!(result.terminal_count, 1);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_nodeset_with_turn_off() {
        // Turn Off should be a valid terminal node
        let nodes = vec![
            create_node(NODE_TYPE_START),
            create_node(NODE_TYPE_TURN_OFF),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(result.is_valid);
        assert_eq!(result.start_count, 1);
        assert_eq!(result.terminal_count, 1);
        assert!(result.errors.is_empty());
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
        assert_eq!(result.terminal_count, 1);
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
        assert_eq!(result.terminal_count, 1);
    }

    #[test]
    fn test_validate_nodeset_disconnected_nodes_allowed() {
        // Disconnected nodes are allowed - they will be treated as "Do Nothing" at runtime
        let nodes = vec![
            create_node(NODE_TYPE_START),
            create_node(NODE_TYPE_EXECUTE_ACTION),
        ];
        // No edges connecting them - this is allowed
        let result = validate_nodeset(&nodes);
        
        assert!(result.is_valid);
        assert_eq!(result.start_count, 1);
        assert_eq!(result.terminal_count, 1);
        assert!(result.errors.is_empty());
    }

    // -------------------------------------------------------------------------
    // Tests for update_node_definitions
    // -------------------------------------------------------------------------

    /// Creates a node with an outdated definition (missing inputs/outputs)
    fn create_outdated_node(node_type: &str) -> serde_json::Value {
        json!({
            "id": format!("{}-outdated-1", node_type),
            "type": "custom",
            "position": { "x": 100, "y": 200 },
            "data": {
                "label": "Outdated Node",
                "definition": {
                    "node_type": node_type,
                    "name": "Old Name",
                    "description": "Old description",
                    "category": "Old Category",
                    "inputs": [],
                    "outputs": [],
                    "color": "#000000"
                },
                "comment": "User comment",
                "primitiveValue": 42
            }
        })
    }

    #[test]
    fn test_update_node_definitions_updates_outdated_nodes() {
        // Create a node with an outdated definition
        let outdated_node = create_outdated_node("logic_and");
        let nodes = vec![outdated_node];
        
        // Update the definitions
        let (updated_nodes, removed_ids) = update_node_definitions(nodes);
        
        // Should have one node with no removals
        assert_eq!(updated_nodes.len(), 1);
        assert!(removed_ids.is_empty());
        
        // Verify the definition was updated
        let updated = &updated_nodes[0];
        let definition = updated.get("data")
            .and_then(|d| d.get("definition"))
            .expect("Node should have definition");
        
        // Name should be updated to current version
        assert_eq!(definition.get("name").and_then(|n| n.as_str()), Some("AND"));
        // Category should be updated
        assert_eq!(definition.get("category").and_then(|c| c.as_str()), Some("Logic"));
        // Should now have inputs and outputs from current definition
        let inputs = definition.get("inputs").and_then(|i| i.as_array()).expect("Should have inputs");
        assert!(inputs.len() >= 2, "AND node should have at least 2 inputs");
        let outputs = definition.get("outputs").and_then(|o| o.as_array()).expect("Should have outputs");
        assert_eq!(outputs.len(), 1, "AND node should have 1 output");
    }

    #[test]
    fn test_update_node_definitions_preserves_user_data() {
        // Create a node with user-specific data
        let node = json!({
            "id": "primitive_integer-123",
            "type": "custom",
            "position": { "x": 50, "y": 75 },
            "data": {
                "label": "Integer",
                "definition": {
                    "node_type": "primitive_integer",
                    "name": "Old Name",
                    "description": "Old desc",
                    "category": "Primitives",
                    "inputs": [],
                    "outputs": [],
                    "color": "#000000"
                },
                "primitiveValue": 999,
                "comment": "Important integer value"
            }
        });
        let nodes = vec![node];
        
        let (updated_nodes, removed_ids) = update_node_definitions(nodes);
        
        assert_eq!(updated_nodes.len(), 1);
        assert!(removed_ids.is_empty());
        
        let updated = &updated_nodes[0];
        let data = updated.get("data").expect("Should have data");
        
        // User data should be preserved
        assert_eq!(data.get("primitiveValue").and_then(|v| v.as_i64()), Some(999));
        assert_eq!(data.get("comment").and_then(|c| c.as_str()), Some("Important integer value"));
        
        // Position should be preserved
        let position = updated.get("position").expect("Should have position");
        assert_eq!(position.get("x").and_then(|x| x.as_f64()), Some(50.0));
        assert_eq!(position.get("y").and_then(|y| y.as_f64()), Some(75.0));
    }

    #[test]
    fn test_update_node_definitions_removes_unknown_nodes() {
        // Create a node with an unknown type
        let unknown_node = json!({
            "id": "unknown_type-1",
            "type": "custom",
            "position": { "x": 0, "y": 0 },
            "data": {
                "definition": {
                    "node_type": "non_existent_node_type",
                    "name": "Unknown",
                    "description": "This node type does not exist",
                    "category": "Unknown",
                    "inputs": [],
                    "outputs": [],
                    "color": "#FF0000"
                }
            }
        });
        
        // Also add a valid node
        let valid_node = create_node("logic_and");
        let nodes = vec![unknown_node, valid_node];
        
        let (updated_nodes, removed_ids) = update_node_definitions(nodes);
        
        // Only valid node should remain
        assert_eq!(updated_nodes.len(), 1);
        assert_eq!(removed_ids.len(), 1);
        assert_eq!(removed_ids[0], "unknown_type-1");
        
        // Verify the remaining node is the valid one
        let node_type = updated_nodes[0]
            .get("data")
            .and_then(|d| d.get("definition"))
            .and_then(|def| def.get("node_type"))
            .and_then(|nt| nt.as_str());
        assert_eq!(node_type, Some("logic_and"));
    }

    #[test]
    fn test_update_node_definitions_handles_empty_list() {
        let nodes: Vec<serde_json::Value> = vec![];
        let (updated_nodes, removed_ids) = update_node_definitions(nodes);
        
        assert!(updated_nodes.is_empty());
        assert!(removed_ids.is_empty());
    }

    #[test]
    fn test_remove_orphaned_edges_removes_edges_to_removed_nodes() {
        let edges = vec![
            json!({
                "id": "edge-1",
                "source": "node-1",
                "sourceHandle": "output",
                "target": "node-2",
                "targetHandle": "input"
            }),
            json!({
                "id": "edge-2",
                "source": "node-2",
                "sourceHandle": "output",
                "target": "node-3",
                "targetHandle": "input"
            }),
            json!({
                "id": "edge-3",
                "source": "node-1",
                "sourceHandle": "output",
                "target": "node-3",
                "targetHandle": "input"
            }),
        ];
        
        // Remove node-2
        let removed_ids = vec!["node-2".to_string()];
        let filtered_edges = remove_orphaned_edges(edges, &removed_ids);
        
        // Only edge-3 should remain (node-1 -> node-3)
        assert_eq!(filtered_edges.len(), 1);
        assert_eq!(
            filtered_edges[0].get("id").and_then(|id| id.as_str()),
            Some("edge-3")
        );
    }

    #[test]
    fn test_remove_orphaned_edges_preserves_valid_edges() {
        let edges = vec![
            json!({
                "id": "edge-1",
                "source": "node-1",
                "target": "node-2"
            }),
            json!({
                "id": "edge-2",
                "source": "node-2",
                "target": "node-3"
            }),
        ];
        
        // No nodes removed
        let removed_ids: Vec<String> = vec![];
        let filtered_edges = remove_orphaned_edges(edges.clone(), &removed_ids);
        
        assert_eq!(filtered_edges.len(), 2);
    }

    #[test]
    fn test_update_preserves_enum_value() {
        // Create a cause_reason node with a specific enum value
        let node = json!({
            "id": "cause_reason-1",
            "type": "custom",
            "position": { "x": 0, "y": 0 },
            "data": {
                "label": "Cause Reason",
                "definition": {
                    "node_type": "cause_reason",
                    "name": "Old Cause Reason",
                    "description": "Old description",
                    "category": "Enums",
                    "inputs": [],
                    "outputs": [{ "id": "value", "label": "Value" }],
                    "color": "#E91E63"
                },
                "enumValue": "5"
            }
        });
        let nodes = vec![node];
        
        let (updated_nodes, removed_ids) = update_node_definitions(nodes);
        
        assert_eq!(updated_nodes.len(), 1);
        assert!(removed_ids.is_empty());
        
        let data = updated_nodes[0].get("data").expect("Should have data");
        
        // Enum value should be preserved
        assert_eq!(data.get("enumValue").and_then(|v| v.as_str()), Some("5"));
        
        // But definition should be updated
        let definition = data.get("definition").expect("Should have definition");
        assert_eq!(definition.get("name").and_then(|n| n.as_str()), Some("Cause Reason"));
    }

    #[test]
    fn test_update_preserves_dynamic_inputs() {
        // Create an AND node with dynamically added inputs
        let node = json!({
            "id": "logic_and-1",
            "type": "custom",
            "position": { "x": 0, "y": 0 },
            "data": {
                "label": "AND",
                "definition": {
                    "node_type": "logic_and",
                    "name": "AND",
                    "description": "Old description",
                    "category": "Logic",
                    "inputs": [
                        { "id": "input_1", "label": "Input 1" },
                        { "id": "input_2", "label": "Input 2" }
                    ],
                    "outputs": [{ "id": "result", "label": "Result" }],
                    "color": "#9C27B0"
                },
                "dynamicInputs": [
                    { "id": "input_1", "label": "Input 1" },
                    { "id": "input_2", "label": "Input 2" },
                    { "id": "input_3", "label": "Input 3" },
                    { "id": "input_4", "label": "Input 4" }
                ]
            }
        });
        let nodes = vec![node];
        
        let (updated_nodes, removed_ids) = update_node_definitions(nodes);
        
        assert_eq!(updated_nodes.len(), 1);
        assert!(removed_ids.is_empty());
        
        let data = updated_nodes[0].get("data").expect("Should have data");
        
        // Dynamic inputs should be preserved
        let dynamic_inputs = data.get("dynamicInputs").and_then(|i| i.as_array()).expect("Should have dynamicInputs");
        assert_eq!(dynamic_inputs.len(), 4);
    }

    // -------------------------------------------------------------------------
    // Tests for evaluate_every_minutes validation
    // -------------------------------------------------------------------------

    fn create_start_node_with_evaluate_minutes(minutes: i64) -> serde_json::Value {
        json!({
            "id": "flow_start-1",
            "type": "custom",
            "position": { "x": 0, "y": 0 },
            "data": {
                "label": "Start",
                "primitiveValue": minutes,
                "definition": {
                    "node_type": "flow_start",
                    "name": "Start",
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
    fn test_validate_nodeset_evaluate_minutes_valid() {
        let nodes = vec![
            create_start_node_with_evaluate_minutes(5),
            create_node(NODE_TYPE_EXECUTE_ACTION),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_nodeset_evaluate_minutes_at_max() {
        let nodes = vec![
            create_start_node_with_evaluate_minutes(MAX_EVALUATE_EVERY_MINUTES as i64),
            create_node(NODE_TYPE_EXECUTE_ACTION),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_nodeset_evaluate_minutes_exceeds_max() {
        let nodes = vec![
            create_start_node_with_evaluate_minutes(1441), // 1 more than max
            create_node(NODE_TYPE_EXECUTE_ACTION),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("cannot exceed 1440")));
    }

    #[test]
    fn test_validate_nodeset_evaluate_minutes_zero() {
        let nodes = vec![
            create_start_node_with_evaluate_minutes(0),
            create_node(NODE_TYPE_EXECUTE_ACTION),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("must be at least 1")));
    }

    #[test]
    fn test_validate_nodeset_evaluate_minutes_negative() {
        let nodes = vec![
            create_start_node_with_evaluate_minutes(-5),
            create_node(NODE_TYPE_EXECUTE_ACTION),
        ];
        let result = validate_nodeset(&nodes);
        
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("must be at least 1")));
    }
}
