pub(crate) mod client_key;
pub(crate) mod health;
pub(crate) mod secret;
pub(crate) mod secret_client_key;

use rusqlite::Connection;

use crate::error::Result;

pub(crate) use self::{
    client_key::SqliteClientKeyRepo, health::SqliteHealthRepo, secret::SqliteSecretRepo,
    secret_client_key::SqliteSecretClientKeyRepo,
};

pub(crate) fn create_db_connection(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    // Enable WAL mode to improve concurrency
    conn.pragma_update(None, "journal_mode", "WAL")?;

    // Set busy timeout to prevent immediate failure on lock conflicts
    conn.busy_timeout(std::time::Duration::from_millis(500))?;

    Ok(conn)
}
