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
    // Check if cause_reasons table has any data
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cause_reasons")
        .fetch_one(pool)
        .await?;

    if count.0 > 0 {
        log::debug!("cause_reasons table already has data, skipping defaults initialization");
        return Ok(());
    }

    log::info!("Initializing cause_reasons with default values...");

    // Load the embedded JSON file
    let file = DefaultsAssets::get("cause_reasons.json")
        .ok_or("cause_reasons.json not found in embedded assets")?;

    let json_str = std::str::from_utf8(&file.data)?;
    let reasons: Vec<CauseReasonDefault> = serde_json::from_str(json_str)?;

    // Insert each reason
    for reason in reasons {
        sqlx::query(
            "INSERT INTO cause_reasons (id, label, description, is_hidden, is_editable) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(reason.id)
        .bind(&reason.label)
        .bind(&reason.description)
        .bind(reason.is_hidden)
        .bind(reason.is_editable)
        .execute(pool)
        .await?;
    }

    log::info!("Successfully initialized cause_reasons with default values");
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
            let parsed: serde_json::Value = serde_json::from_str(&node_json).unwrap_or_default();
            let nodes = parsed.get("nodes").and_then(|n| n.as_array());

            if nodes.map(|n| !n.is_empty()).unwrap_or(false) {
                log::debug!(
                    "Default nodeset already has content, skipping defaults initialization"
                );
                return Ok(());
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

    // Validate it's valid JSON
    let _: serde_json::Value = serde_json::from_str(json_str)?;

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
