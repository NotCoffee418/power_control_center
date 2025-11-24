use chrono::{Datelike, Local, Timelike};

/// Simple estimate if user is home and awake based on time of day
/// Can be replaced later with some phone presence detection or other methods
pub fn is_user_home_and_awake() -> bool {
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
