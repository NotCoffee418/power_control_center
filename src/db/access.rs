use crate::{config, types::db_types};
use sqlx::SqlitePool;
use tokio::sync::OnceCell;

static POOL: OnceCell<SqlitePool> = OnceCell::const_new();

pub async fn get_pool() -> &'static SqlitePool {
    POOL.get_or_init(|| async {
        let cfg = config::get_config();
        // Ensure the directory for the database file exists
        if let Some(parent) = std::path::Path::new(&cfg.database_path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .expect("Failed to create directory directory");
        }
        // Check if we have access to database file
        tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&cfg.database_path)
            .await
            .expect("Insufficient permissions to access database file");

        // Create connection pool
        let conn_str = format!("sqlite://{}", cfg.database_path);
        SqlitePool::connect(&conn_str)
            .await
            .expect("Failed to database create pool")
    })
    .await
}

pub async fn insert_ac_action(ac_action: db_types::AcAction) -> Result<(), sqlx::Error> {
    let pool = get_pool().await;

    sqlx::query(
        r#"
        INSERT INTO ac_actions (action_timestamp, device_identifier, action_type, mode, fan_speed, request_temperature, swing)
        VALUES (?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&ac_action.action_timestamp)
    .bind(&ac_action.device_identifier)
    .bind(&ac_action.action_type)
    .bind(ac_action.mode)
    .bind(ac_action.fan_speed)
    .bind(ac_action.request_temperature)
    .bind(ac_action.swing)
    .execute(pool)
    .await?;

    Ok(())
}
