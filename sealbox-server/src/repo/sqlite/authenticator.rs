use std::sync::{Arc, Mutex};

use crate::{
    error::{Result, SealboxError},
    repo::{Authenticator, AuthenticatorRepo},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteAuthenticatorRepo {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteAuthenticatorRepo {
    pub(crate) fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }

    /// What is stored here is a public key and a credential id — enough to *verify* a signature
    /// and useless for producing one. Reading this table gives an attacker nothing to replay.
    pub fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS authenticators (
                id TEXT PRIMARY KEY,
                identity TEXT NOT NULL,
                passkey TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            (),
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_authenticators_identity ON authenticators (identity)",
            (),
        )?;
        Ok(())
    }
}

impl AuthenticatorRepo for SqliteAuthenticatorRepo {
    fn register(&self, identity: &str, credential_id: &str, passkey: &str) -> Result<()> {
        let guard = self.conn.lock()?;
        guard
            .execute(
                "INSERT INTO authenticators (id, identity, passkey, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                (
                    credential_id,
                    identity,
                    passkey,
                    time::OffsetDateTime::now_utc().unix_timestamp(),
                ),
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    SealboxError::InvalidRequest("that authenticator is already registered".into())
                }
                other => other.into(),
            })?;
        Ok(())
    }

    fn for_identity(&self, identity: &str) -> Result<Vec<Authenticator>> {
        let guard = self.conn.lock()?;
        let mut stmt = guard.prepare(
            "SELECT id, identity, passkey, created_at FROM authenticators WHERE identity = ?1",
        )?;
        let rows = stmt.query_map([identity], |row| {
            Ok(Authenticator {
                credential_id: row.get(0)?,
                identity: row.get(1)?,
                passkey: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn count_for(&self, identity: &str) -> Result<usize> {
        let guard = self.conn.lock()?;
        let count: i64 = guard.query_row(
            "SELECT count(*) FROM authenticators WHERE identity = ?1",
            [identity],
            |r| r.get(0),
        )?;
        Ok(count as usize)
    }
}
