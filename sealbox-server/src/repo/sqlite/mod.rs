pub(crate) mod health;
pub(crate) mod master_key;
pub(crate) mod secret;

use rusqlite::Connection;

use crate::error::Result;

pub(crate) use self::{
    health::SqliteHealthRepo, master_key::SqliteMasterKeyRepo, secret::SqliteSecretRepo,
};

/// Whether a table already has a given column. Used to make migrations idempotent without
/// carrying a schema-version table for two of them.
pub(crate) fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn create_db_connection(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    // Enable WAL mode to improve concurrency
    conn.pragma_update(None, "journal_mode", "WAL")?;

    // Set busy timeout to prevent immediate failure on lock conflicts
    conn.busy_timeout(std::time::Duration::from_millis(500))?;

    Ok(conn)
}
