use crate::db::get_pool;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

/// Database model for cause_reason
#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct CauseReasonRecord {
    pub id: i32,
    pub label: String,
    pub description: String,
    pub is_hidden: bool,
    pub is_editable: bool,
}

/// Get all cause reasons (optionally include hidden)
pub async fn get_all(include_hidden: bool) -> Result<Vec<CauseReasonRecord>, sqlx::Error> {
    let pool = get_pool().await;
    
    if include_hidden {
        sqlx::query_as::<_, CauseReasonRecord>(
            "SELECT id, label, description, is_hidden, is_editable FROM cause_reasons ORDER BY id"
        )
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, CauseReasonRecord>(
            "SELECT id, label, description, is_hidden, is_editable FROM cause_reasons WHERE is_hidden = 0 ORDER BY id"
        )
        .fetch_all(pool)
        .await
    }
}

/// Get a single cause reason by ID
pub async fn get_by_id(id: i32) -> Result<Option<CauseReasonRecord>, sqlx::Error> {
    let pool = get_pool().await;
    
    sqlx::query_as::<_, CauseReasonRecord>(
        "SELECT id, label, description, is_hidden, is_editable FROM cause_reasons WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Create a new cause reason (ID is auto-generated, new reasons are always editable)
pub async fn create(label: &str, description: &str) -> Result<CauseReasonRecord, sqlx::Error> {
    let pool = get_pool().await;
    
    // Get the next available ID
    let (max_id,): (Option<i32>,) = sqlx::query_as(
        "SELECT MAX(id) FROM cause_reasons"
    )
    .fetch_one(pool)
    .await?;
    
    let new_id = max_id.unwrap_or(0) + 1;
    
    sqlx::query(
        "INSERT INTO cause_reasons (id, label, description, is_hidden, is_editable) VALUES (?, ?, ?, 0, 1)"
    )
    .bind(new_id)
    .bind(label)
    .bind(description)
    .execute(pool)
    .await?;
    
    Ok(CauseReasonRecord {
        id: new_id,
        label: label.to_string(),
        description: description.to_string(),
        is_hidden: false,
        is_editable: true,
    })
}

/// Update a cause reason
pub async fn update(id: i32, label: &str, description: &str) -> Result<bool, sqlx::Error> {
    let pool = get_pool().await;
    
    let result = sqlx::query(
        "UPDATE cause_reasons SET label = ?, description = ? WHERE id = ?"
    )
    .bind(label)
    .bind(description)
    .bind(id)
    .execute(pool)
    .await?;
    
    Ok(result.rows_affected() > 0)
}

/// Set the hidden status of a cause reason
pub async fn set_hidden(id: i32, is_hidden: bool) -> Result<bool, sqlx::Error> {
    let pool = get_pool().await;
    
    let result = sqlx::query(
        "UPDATE cause_reasons SET is_hidden = ? WHERE id = ?"
    )
    .bind(is_hidden)
    .bind(id)
    .execute(pool)
    .await?;
    
    Ok(result.rows_affected() > 0)
}

/// Delete a cause reason (cannot delete ID 0)
pub async fn delete(id: i32) -> Result<bool, sqlx::Error> {
    if id == 0 {
        // Cannot delete the Undefined reason
        return Ok(false);
    }
    
    let pool = get_pool().await;
    
    let result = sqlx::query(
        "DELETE FROM cause_reasons WHERE id = ?"
    )
    .bind(id)
    .execute(pool)
    .await?;
    
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cause_reason_record_serializable() {
        let record = CauseReasonRecord {
            id: 1,
            label: "Test".to_string(),
            description: "Test description".to_string(),
            is_hidden: false,
            is_editable: true,
        };
        
        let json = serde_json::to_string(&record).unwrap();
        let deserialized: CauseReasonRecord = serde_json::from_str(&json).unwrap();
        
        assert_eq!(record.id, deserialized.id);
        assert_eq!(record.label, deserialized.label);
        assert_eq!(record.description, deserialized.description);
        assert_eq!(record.is_hidden, deserialized.is_hidden);
        assert_eq!(record.is_editable, deserialized.is_editable);
    }
}
