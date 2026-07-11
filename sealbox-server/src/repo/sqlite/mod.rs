pub(crate) mod health;
pub(crate) mod master_key;
mod migrations;
pub(crate) mod secret;
pub(crate) mod tenant;

use rusqlite::Connection;

use crate::error::Result;

pub use self::migrations::{MigrationReport, inspect_migration_path};
pub(crate) use self::{
    health::SqliteHealthRepo,
    master_key::SqliteMasterKeyRepo,
    migrations::{backup_before_migration, run_migrations},
    secret::SqliteSecretRepo,
    tenant::SqliteTenantRepo,
};

pub(crate) fn create_db_connection(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    // Enable WAL mode to improve concurrency
    conn.pragma_update(None, "journal_mode", "WAL")?;

    // Set busy timeout to prevent immediate failure on lock conflicts
    conn.busy_timeout(std::time::Duration::from_millis(500))?;

    Ok(conn)
}
