//! Database defaults initialization
//!
//! This module handles loading default data for cause_reasons and nodesets
//! from embedded JSON files at startup.

use rust_embed::Embed;
use sqlx::SqlitePool;

/// Embedded default data files
#[derive(Embed)]
#[folder = "defaults/"]
struct DefaultsAssets;

/// Initialize database defaults if tables are empty
///
/// This function checks if cause_reasons and nodesets tables need
/// default data and populates them from embedded JSON files.
pub async fn initialize_defaults(pool: &SqlitePool) {
    if let Err(e) = initialize_cause_reasons(pool).await {
        log::error!("Failed to initialize cause_reasons defaults: {}", e);
    }

    if let Err(e) = initialize_default_nodeset(pool).await {
        log::error!("Failed to initialize default nodeset: {}", e);
    }
}

/// Initialize cause_reasons with default values if the table is empty
async fn initialize_cause_reasons(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    // Load the embedded JSON file
    let file = DefaultsAssets::get("cause_reasons.json")
        .ok_or("cause_reasons.json not found in embedded assets")?;

    let json_str = std::str::from_utf8(&file.data)?;
    let reasons: Vec<CauseReasonDefault> = serde_json::from_str(json_str)?;

    // Get existing cause_reason IDs
    let existing: Vec<(i32,)> = sqlx::query_as("SELECT id FROM cause_reasons")
        .fetch_all(pool)
        .await?;
    let existing_ids: std::collections::HashSet<i32> =
        existing.into_iter().map(|(id,)| id).collect();

    // Insert missing reasons (use INSERT OR IGNORE to handle race conditions)
    let mut inserted_count = 0;
    for reason in reasons {
        if !existing_ids.contains(&reason.id) {
            sqlx::query(
                "INSERT OR IGNORE INTO cause_reasons (id, label, description, is_hidden, is_editable) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(reason.id)
            .bind(&reason.label)
            .bind(&reason.description)
            .bind(reason.is_hidden)
            .bind(reason.is_editable)
            .execute(pool)
            .await?;
            inserted_count += 1;
        }
    }

    if inserted_count > 0 {
        log::info!(
            "Initialized {} missing cause_reasons with default values",
            inserted_count
        );
    } else {
        log::debug!("All cause_reasons already exist, no defaults needed");
    }

    Ok(())
}

/// Initialize default nodeset if the default nodeset (id=0) is empty
async fn initialize_default_nodeset(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    // Check if default nodeset exists and has content
    let result: Option<(String,)> =
        sqlx::query_as("SELECT node_json FROM nodesets WHERE id = 0")
            .fetch_optional(pool)
            .await?;

    match result {
        Some((node_json,)) => {
            // Check if the node_json is empty or just the default empty structure
            match serde_json::from_str::<serde_json::Value>(&node_json) {
                Ok(parsed) => {
                    let nodes = parsed.get("nodes").and_then(|n| n.as_array());
                    if nodes.map(|n| !n.is_empty()).unwrap_or(false) {
                        log::debug!(
                            "Default nodeset already has content, skipping defaults initialization"
                        );
                        return Ok(());
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Existing nodeset JSON is invalid ({}), will replace with defaults",
                        e
                    );
                }
            }
        }
        None => {
            // No default nodeset exists, we'll create one
        }
    }

    log::info!("Initializing default nodeset with default profile...");

    // Load the embedded JSON file
    let file = DefaultsAssets::get("default_nodeset.json")
        .ok_or("default_nodeset.json not found in embedded assets")?;

    let json_str = std::str::from_utf8(&file.data)?;

    // Update or insert the default nodeset
    sqlx::query("INSERT OR REPLACE INTO nodesets (id, name, node_json) VALUES (0, 'Default', ?)")
        .bind(json_str)
        .execute(pool)
        .await?;

    log::info!("Successfully initialized default nodeset with default profile");
    Ok(())
}

/// Struct for deserializing cause_reason defaults
#[derive(serde::Deserialize)]
struct CauseReasonDefault {
    id: i32,
    label: String,
    description: String,
    is_hidden: bool,
    is_editable: bool,
}
