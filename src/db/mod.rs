pub mod ac_actions;

use crate::config;
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
