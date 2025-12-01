//! Database defaults initialization
//!
//! This module handles loading default data for cause_reasons and nodesets
//! from embedded JSON files at startup. System defaults are always updated
//! on startup to ensure consistency.

use rust_embed::Embed;
use sqlx::SqlitePool;

/// Embedded default data files
#[derive(Embed)]
#[folder = "defaults/"]
struct DefaultsAssets;

/// System cause_reason IDs are reserved (0-99). User-created reasons start at 100.
const SYSTEM_CAUSE_REASON_MAX_ID: i32 = 99;

/// Initialize database defaults on startup
///
/// This function updates system cause_reasons and the default nodeset
/// from embedded JSON files. System defaults are always updated to ensure
/// consistency across versions.
pub async fn initialize_defaults(pool: &SqlitePool) {
    if let Err(e) = update_system_cause_reasons(pool).await {
        log::error!("Failed to update system cause_reasons: {}", e);
    }

    if let Err(e) = update_default_nodeset(pool).await {
        log::error!("Failed to update default nodeset: {}", e);
    }
}

/// Update system cause_reasons with values from embedded JSON
/// System cause_reasons (IDs 0-99) are always updated on startup
async fn update_system_cause_reasons(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    // Load the embedded JSON file
    let file = DefaultsAssets::get("cause_reasons.json")
        .ok_or("cause_reasons.json not found in embedded assets")?;

    let json_str = std::str::from_utf8(&file.data)?;
    let reasons: Vec<CauseReasonDefault> = serde_json::from_str(json_str)?;

    // Update each system cause_reason using INSERT OR REPLACE
    for reason in &reasons {
        if reason.id > SYSTEM_CAUSE_REASON_MAX_ID {
            log::warn!(
                "Skipping cause_reason with ID {} - system IDs must be <= {}",
                reason.id,
                SYSTEM_CAUSE_REASON_MAX_ID
            );
            continue;
        }

        sqlx::query(
            "INSERT OR REPLACE INTO cause_reasons (id, label, description, is_hidden, is_editable) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(reason.id)
        .bind(&reason.label)
        .bind(&reason.description)
        .bind(reason.is_hidden)
        .bind(reason.is_editable)
        .execute(pool)
        .await?;
    }

    log::debug!(
        "Updated {} system cause_reasons from defaults",
        reasons.len()
    );
    Ok(())
}

/// Update the default nodeset (ID 0) with the embedded default profile
async fn update_default_nodeset(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    // Load the embedded JSON file
    let file = DefaultsAssets::get("default_nodeset.json")
        .ok_or("default_nodeset.json not found in embedded assets")?;

    let json_str = std::str::from_utf8(&file.data)?;

    // Check if the default nodeset exists and has user modifications
    let result: Option<(String,)> =
        sqlx::query_as("SELECT node_json FROM nodesets WHERE id = 0")
            .fetch_optional(pool)
            .await?;

    match result {
        Some((existing_json,)) => {
            // Only update if the nodeset is empty or has the default empty structure
            match serde_json::from_str::<serde_json::Value>(&existing_json) {
                Ok(parsed) => {
                    let nodes = parsed.get("nodes").and_then(|n| n.as_array());
                    if nodes.map(|n| !n.is_empty()).unwrap_or(false) {
                        log::debug!(
                            "Default nodeset has user content, skipping update"
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

    // Update or insert the default nodeset
    sqlx::query("INSERT OR REPLACE INTO nodesets (id, name, node_json) VALUES (0, 'Default', ?)")
        .bind(json_str)
        .execute(pool)
        .await?;

    log::info!("Updated default nodeset with system defaults");
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
