use chrono::{Datelike, Local, Timelike};

/// Simple estimate if user is home and awake based on time of day
/// Can be replaced later with some phone presence detection or other methods
/// Now integrates with user_is_home_override setting from database
pub fn is_user_home_and_awake() -> bool {
    // Check database override first
    if let Some(override_result) = check_user_home_override() {
        return override_result;
    }

    // Fall back to time-based logic
    let now = Local::now();
    let hour = now.hour();
    let minute = now.minute();
    let weekday = now.weekday();

    // Convert to minutes since midnight for easier comparison
    let current_minutes = hour * 60 + minute;

    // Check if weekend (Saturday = 6, Sunday = 0 in chrono)
    let is_weekend = weekday.number_from_monday() >= 6;

    if is_weekend {
        // Weekend: 9am (540 min) through 2am (120 min next day)
        // 2am is tricky - treating as "late night" so 9am-11:59pm OR midnight-2am
        current_minutes >= 9 * 60 || current_minutes < 2 * 60
    } else {
        // Weekday: 3:30pm (930 min) through 2am
        current_minutes >= 15 * 60 + 30 || current_minutes < 2 * 60
    }
}

/// Check if there's an active user home override in the database
/// Returns Some(true) if override is active and user is home
/// Returns Some(false) if override expired (also clears it)
/// Returns None if no override is set
fn check_user_home_override() -> Option<bool> {
    // Use tokio runtime to run async code in this sync context
    let runtime = tokio::runtime::Handle::try_current().ok()?;
    
    runtime.block_on(async {
        let pool = crate::db::get_pool().await;
        
        // Get the override value from settings
        let result = sqlx::query_as::<_, (String,)>(
            "SELECT setting_value FROM settings WHERE setting_key = 'user_is_home_override'"
        )
        .fetch_optional(pool)
        .await;
        
        match result {
            Ok(Some((value_str,))) => {
                if let Ok(override_timestamp) = value_str.parse::<i64>() {
                    if override_timestamp > 0 {
                        let now = chrono::Utc::now().timestamp();
                        
                        if now < override_timestamp {
                            // Override is still active
                            Some(true)
                        } else {
                            // Override has expired, reset it to 0
                            log::info!("User home override expired, resetting to 0");
                            let _ = sqlx::query(
                                "UPDATE settings SET setting_value = '0' WHERE setting_key = 'user_is_home_override'"
                            )
                            .execute(pool)
                            .await;
                            
                            // Return None to fall through to normal logic
                            None
                        }
                    } else {
                        // Override is 0 (disabled)
                        None
                    }
                } else {
                    log::warn!("Failed to parse user_is_home_override value: {}", value_str);
                    None
                }
            }
            Ok(None) => None,
            Err(e) => {
                log::warn!("Failed to check user_is_home_override: {}", e);
                None
            }
        }
    })
}
