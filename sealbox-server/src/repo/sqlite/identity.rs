use std::sync::{Arc, Mutex};

use rusqlite::OptionalExtension;

use crate::{
    error::{Result, SealboxError},
    repo::{Identity, IdentityRepo, hash_token},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteIdentityRepo {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteIdentityRepo {
    pub(crate) fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }

    pub fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS identities (
                id BLOB PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                role TEXT NOT NULL,
                token_hash BLOB NOT NULL,
                issuer TEXT,
                subject TEXT,
                audience TEXT,
                created_at INTEGER NOT NULL,
                revoked_at INTEGER
            )",
            (),
        )?;
        // Authentication is a lookup by hash on every request, so it must be indexed.
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_identities_token_hash ON identities (token_hash)",
            (),
        )?;
        Ok(())
    }

    const COLUMNS: &'static str =
        "id, name, role, token_hash, issuer, subject, audience, created_at, revoked_at";

    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Identity> {
        Ok(Identity {
            id: row.get(0)?,
            name: row.get(1)?,
            role: row.get(2)?,
            token_hash: row.get(3)?,
            issuer: row.get(4)?,
            subject: row.get(5)?,
            audience: row.get(6)?,
            created_at: row.get(7)?,
            revoked_at: row.get(8)?,
        })
    }
}

impl IdentityRepo for SqliteIdentityRepo {
    fn create(&self, identity: &Identity) -> Result<()> {
        let guard = self.conn.lock()?;
        guard
            .execute(
                "INSERT INTO identities
                     (id, name, role, token_hash, issuer, subject, audience, created_at, revoked_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                (
                    &identity.id,
                    &identity.name,
                    &identity.role,
                    &identity.token_hash,
                    &identity.issuer,
                    &identity.subject,
                    &identity.audience,
                    &identity.created_at,
                    &identity.revoked_at,
                ),
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    SealboxError::IdentityAlreadyExists(identity.name.clone())
                }
                other => other.into(),
            })?;
        Ok(())
    }

    /// An empty hash never matches: a workload identity holds no token, and its row stores an
    /// empty one. Without this, presenting an empty credential would resolve to it.
    fn find_by_token(&self, token: &str) -> Result<Option<Identity>> {
        let hash = hash_token(token);
        let guard = self.conn.lock()?;
        let mut stmt = guard.prepare(&format!(
            "SELECT {} FROM identities
             WHERE token_hash = ?1 AND token_hash != X'' AND revoked_at IS NULL LIMIT 1",
            Self::COLUMNS
        ))?;
        Ok(stmt.query_one([hash], Self::from_row).optional()?)
    }

    fn find_by_workload(&self, issuer: &str, subject: &str) -> Result<Option<Identity>> {
        let guard = self.conn.lock()?;
        let mut stmt = guard.prepare(&format!(
            "SELECT {} FROM identities
             WHERE issuer = ?1 AND subject = ?2 AND revoked_at IS NULL LIMIT 1",
            Self::COLUMNS
        ))?;
        Ok(stmt
            .query_one([issuer, subject], Self::from_row)
            .optional()?)
    }

    /// Resolves a presented token by looking up its hash — a single indexed query, rather than
    /// reading candidates and comparing them. A revoked identity resolves to `None`, so a
    /// revocation takes effect on the very next request.
    fn find_by_name(&self, name: &str) -> Result<Option<Identity>> {
        let guard = self.conn.lock()?;
        let mut stmt = guard.prepare(&format!(
            "SELECT {} FROM identities WHERE name = ?1 AND revoked_at IS NULL LIMIT 1",
            Self::COLUMNS
        ))?;
        Ok(stmt.query_one([name], Self::from_row).optional()?)
    }

    fn list(&self) -> Result<Vec<Identity>> {
        let guard = self.conn.lock()?;
        let mut stmt = guard.prepare(&format!(
            "SELECT {} FROM identities ORDER BY created_at",
            Self::COLUMNS
        ))?;
        let rows = stmt.query_map([], Self::from_row)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn revoke(&self, name: &str) -> Result<()> {
        let guard = self.conn.lock()?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let updated = guard.execute(
            "UPDATE identities SET revoked_at = ?1 WHERE name = ?2 AND revoked_at IS NULL",
            (now, name),
        )?;
        if updated == 0 {
            return Err(SealboxError::IdentityNotFound(name.to_string()));
        }
        Ok(())
    }

    fn any_exists(&self) -> Result<bool> {
        let guard = self.conn.lock()?;
        let count: i64 = guard.query_row("SELECT count(*) FROM identities", [], |r| r.get(0))?;
        Ok(count > 0)
    }
}
