use std::sync::{Arc, Mutex};

use crate::{
    error::{Result, SealboxError},
    repo::{Issuer, IssuerRepo},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteIssuerRepo {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteIssuerRepo {
    pub(crate) fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }

    /// Only public keys are stored here, which is what makes registering them safe. A JWKS is
    /// published material: holding it lets the server *verify* and never *sign*.
    pub fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS issuers (
                name TEXT PRIMARY KEY,
                url TEXT NOT NULL UNIQUE,
                jwks TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            (),
        )?;
        Ok(())
    }
}

impl IssuerRepo for SqliteIssuerRepo {
    fn register(&self, issuer: &Issuer) -> Result<()> {
        let guard = self.conn.lock()?;
        guard
            .execute(
                "INSERT INTO issuers (name, url, jwks, created_at) VALUES (?1, ?2, ?3, ?4)",
                (&issuer.name, &issuer.url, &issuer.jwks, issuer.created_at),
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    SealboxError::InvalidRequest(format!(
                        "an issuer named `{}` or using that URL is already registered",
                        issuer.name
                    ))
                }
                other => other.into(),
            })?;
        Ok(())
    }

    /// Replace an issuer's keys. This is how a rotation lands: register the JWKS holding both the
    /// old key and the new one, then register it again without the old one once nothing presents
    /// it. The URL is not updated — a different URL is a different issuer.
    fn update_keys(&self, name: &str, jwks: &str) -> Result<()> {
        let guard = self.conn.lock()?;
        let changed =
            guard.execute("UPDATE issuers SET jwks = ?1 WHERE name = ?2", (jwks, name))?;
        if changed == 0 {
            return Err(SealboxError::InvalidRequest(format!(
                "no issuer named `{name}`"
            )));
        }
        Ok(())
    }

    fn remove(&self, name: &str) -> Result<()> {
        let guard = self.conn.lock()?;
        let changed = guard.execute("DELETE FROM issuers WHERE name = ?1", [name])?;
        if changed == 0 {
            return Err(SealboxError::InvalidRequest(format!(
                "no issuer named `{name}`"
            )));
        }
        Ok(())
    }

    fn list(&self) -> Result<Vec<Issuer>> {
        let guard = self.conn.lock()?;
        let mut stmt =
            guard.prepare("SELECT name, url, jwks, created_at FROM issuers ORDER BY name")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Issuer {
                    name: row.get(0)?,
                    url: row.get(1)?,
                    jwks: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| SealboxError::DatabaseError(e.to_string()))?;
        Ok(rows)
    }

    /// By the `iss` claim rather than by name: that is what arrives in a token.
    fn find_by_url(&self, url: &str) -> Result<Option<Issuer>> {
        let guard = self.conn.lock()?;
        let mut stmt =
            guard.prepare("SELECT name, url, jwks, created_at FROM issuers WHERE url = ?1")?;
        let mut rows = stmt.query([url])?;
        match rows.next()? {
            Some(row) => Ok(Some(Issuer {
                name: row.get(0)?,
                url: row.get(1)?,
                jwks: row.get(2)?,
                created_at: row.get(3)?,
            })),
            None => Ok(None),
        }
    }
}
