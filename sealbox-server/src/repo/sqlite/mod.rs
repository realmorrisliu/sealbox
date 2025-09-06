pub(crate) mod client_key;
pub(crate) mod enroll;
pub(crate) mod health;
pub(crate) mod secret;
pub(crate) mod secret_client_key;

use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

use crate::error::Result;

pub(crate) use self::{
    client_key::SqliteClientKeyRepo, enroll::SqliteEnrollRepo, health::SqliteHealthRepo,
    secret::SqliteSecretRepo, secret_client_key::SqliteSecretClientKeyRepo,
};

pub(crate) async fn create_db_pool(db_path: &str) -> Result<SqlitePool> {
    let database_url = format!("sqlite:{db_path}");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&database_url)
        .await?;

    // Enable WAL mode to improve concurrency
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;

    // Set busy timeout to prevent immediate failure on lock conflicts
    sqlx::query("PRAGMA busy_timeout = 30000")
        .execute(&pool)
        .await?;

    Ok(pool)
}
