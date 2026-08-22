use std::sync::{Arc, Mutex};

use rusqlite::OptionalExtension;

use crate::{
    error::{Result, SealboxError},
    repo::{Grant, GrantRepo, Implementation},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteGrantRepo {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteGrantRepo {
    pub(crate) fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }

    pub fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS grants (
                name TEXT PRIMARY KEY,
                implementation TEXT NOT NULL,
                runner TEXT NOT NULL,
                secrets TEXT NOT NULL,
                files TEXT NOT NULL DEFAULT '{}',
                chain TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                created_by TEXT NOT NULL
            )",
            (),
        )?;
        Ok(())
    }

    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Grant> {
        let implementation: String = row.get(1)?;
        let secrets: String = row.get(3)?;
        let files: String = row.get(4)?;
        let chain: String = row.get(5)?;
        Ok(Grant {
            name: row.get(0)?,
            implementation: serde_json::from_str::<Implementation>(&implementation).map_err(
                |e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                },
            )?,
            runner: row.get(2)?,
            files: serde_json::from_str(&files).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
            secrets: serde_json::from_str(&secrets).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
            then: serde_json::from_str(&chain).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
            created_at: row.get(6)?,
            created_by: row.get(7)?,
        })
    }

    const COLUMNS: &'static str =
        "name, implementation, runner, secrets, files, chain, created_at, created_by";
}

impl GrantRepo for SqliteGrantRepo {
    fn create(&self, grant: &Grant) -> Result<()> {
        let guard = self.conn.lock()?;
        guard
            .execute(
                "INSERT INTO grants (name, implementation, runner, secrets, files, chain, created_at, created_by)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    &grant.name,
                    serde_json::to_string(&grant.implementation)
                        .map_err(|e| SealboxError::DatabaseError(e.to_string()))?,
                    &grant.runner,
                    serde_json::to_string(&grant.secrets)
                        .map_err(|e| SealboxError::DatabaseError(e.to_string()))?,
                    serde_json::to_string(&grant.files)
                        .map_err(|e| SealboxError::DatabaseError(e.to_string()))?,
                    serde_json::to_string(&grant.then)
                        .map_err(|e| SealboxError::DatabaseError(e.to_string()))?,
                    &grant.created_at,
                    &grant.created_by,
                ),
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    SealboxError::GrantAlreadyExists(grant.name.clone())
                }
                other => other.into(),
            })?;
        Ok(())
    }

    fn get(&self, name: &str) -> Result<Option<Grant>> {
        let guard = self.conn.lock()?;
        let mut stmt = guard.prepare(&format!(
            "SELECT {} FROM grants WHERE name = ?1",
            Self::COLUMNS
        ))?;
        Ok(stmt.query_one([name], Self::from_row).optional()?)
    }

    fn list(&self) -> Result<Vec<Grant>> {
        let guard = self.conn.lock()?;
        let mut stmt = guard.prepare(&format!(
            "SELECT {} FROM grants ORDER BY name",
            Self::COLUMNS
        ))?;
        let rows = stmt.query_map([], Self::from_row)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn remove(&self, name: &str) -> Result<()> {
        let guard = self.conn.lock()?;
        let removed = guard.execute("DELETE FROM grants WHERE name = ?1", [name])?;
        if removed == 0 {
            return Err(SealboxError::GrantNotFound(name.to_string()));
        }
        Ok(())
    }
}
