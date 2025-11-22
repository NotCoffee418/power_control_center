use crate::{db::get_pool, types::db_types};

pub async fn insert(ac_action: db_types::AcAction) -> Result<(), sqlx::Error> {
    let pool = get_pool().await;

    sqlx::query(
        r#"
        INSERT INTO ac_actions (action_timestamp, device_identifier, action_type, mode, fan_speed, request_temperature, swing, measured_temperature, measured_net_power_watt, is_human_home, cause_id)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&ac_action.action_timestamp)
    .bind(&ac_action.device_identifier)
    .bind(&ac_action.action_type)
    .bind(ac_action.mode)
    .bind(ac_action.fan_speed)
    .bind(ac_action.request_temperature)
    .bind(ac_action.swing)
    .bind(ac_action.measured_temperature)
    .bind(ac_action.measured_net_power_watt)
    .bind(ac_action.is_human_home)
    .bind(ac_action.cause_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_page(limit: i64, offset: i64) -> Result<Vec<db_types::AcAction>, sqlx::Error> {
    let pool = get_pool().await;

    let actions = sqlx::query_as::<_, db_types::AcAction>(
        r#"
        SELECT * FROM ac_actions
        ORDER BY action_timestamp DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(actions)
}

pub async fn get_count() -> Result<i64, sqlx::Error> {
    let pool = get_pool().await;

    let (count,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM ac_actions
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(count)
}
