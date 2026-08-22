pub(crate) mod audit;
pub(crate) mod authenticator;
pub(crate) mod grant;
pub(crate) mod health;
pub(crate) mod identity;
pub(crate) mod issuer;
pub(crate) mod job;
pub(crate) mod master_key;
pub(crate) mod recovery;
pub(crate) mod secret;

use rusqlite::Connection;

use crate::error::Result;

pub(crate) use self::{
    audit::SqliteAuditRepo, authenticator::SqliteAuthenticatorRepo, grant::SqliteGrantRepo,
    health::SqliteHealthRepo, identity::SqliteIdentityRepo, issuer::SqliteIssuerRepo,
    job::SqliteJobRepo, master_key::SqliteMasterKeyRepo, recovery::SqliteRecoveryRepo,
    secret::SqliteSecretRepo,
};

pub(crate) fn create_db_connection(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    // Enable WAL mode to improve concurrency
    conn.pragma_update(None, "journal_mode", "WAL")?;

    // Set busy timeout to prevent immediate failure on lock conflicts
    conn.busy_timeout(std::time::Duration::from_millis(500))?;

    Ok(conn)
}
