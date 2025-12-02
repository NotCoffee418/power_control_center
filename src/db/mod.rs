pub mod ac_actions;

pub mod cause_reasons;

pub mod defaults;

pub mod nodesets;

use crate::config;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::time::Duration;
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

        // Create connection options with extended timeouts for slow devices
        let conn_str = format!("sqlite://{}", cfg.database_path);
        let connect_options = SqliteConnectOptions::from_str(&conn_str)
            .expect("Invalid database connection string")
            .busy_timeout(Duration::from_secs(30)) // Wait up to 30 seconds if database is locked
            .create_if_missing(true);

        // Create connection pool with appropriate settings
        SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(30)) // Wait up to 30 seconds to acquire a connection
            .connect_with(connect_options)
            .await
            .expect("Failed to create database pool")
    })
    .await
}
