use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::{
    error::{Result, SealboxError},
    repo::{RecoveryBlob, RecoveryRepo},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteRecoveryRepo {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteRecoveryRepo {
    pub(crate) fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }

    /// One row per recovery key. Not in `secrets`: a secret is something grants may declare and
    /// listings show, and this is neither.
    pub fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS recovery_blobs (
                recovery_key_id BLOB PRIMARY KEY,
                encrypted_data BLOB NOT NULL,
                encrypted_data_key BLOB NOT NULL,
                master_key_fingerprint TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            (),
        )?;
        Ok(())
    }
}

impl RecoveryRepo for SqliteRecoveryRepo {
    /// Replaces any blob already held for that key, because a blob is a snapshot of the current
    /// master key and an older one is not a second backup — it is a wrong one.
    fn store(&self, blob: &RecoveryBlob) -> Result<()> {
        let guard = self.conn.lock()?;
        guard.execute(
            "INSERT INTO recovery_blobs
                 (recovery_key_id, encrypted_data, encrypted_data_key, master_key_fingerprint, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(recovery_key_id) DO UPDATE SET
                 encrypted_data = excluded.encrypted_data,
                 encrypted_data_key = excluded.encrypted_data_key,
                 master_key_fingerprint = excluded.master_key_fingerprint,
                 created_at = excluded.created_at",
            (
                &blob.recovery_key_id,
                &blob.encrypted_data,
                &blob.encrypted_data_key,
                &blob.master_key_fingerprint,
                blob.created_at,
            ),
        )?;
        Ok(())
    }

    fn get(&self, recovery_key_id: &Uuid) -> Result<Option<RecoveryBlob>> {
        let guard = self.conn.lock()?;
        let mut stmt = guard.prepare(
            "SELECT recovery_key_id, encrypted_data, encrypted_data_key, master_key_fingerprint,
                    created_at
             FROM recovery_blobs WHERE recovery_key_id = ?1",
        )?;
        let mut rows = stmt.query([recovery_key_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(RecoveryBlob {
                recovery_key_id: row.get(0)?,
                encrypted_data: row.get(1)?,
                encrypted_data_key: row.get(2)?,
                master_key_fingerprint: row.get(3)?,
                created_at: row.get(4)?,
            })),
            None => Ok(None),
        }
    }

    fn list(&self) -> Result<Vec<RecoveryBlob>> {
        let guard = self.conn.lock()?;
        let mut stmt = guard.prepare(
            "SELECT recovery_key_id, encrypted_data, encrypted_data_key, master_key_fingerprint,
                    created_at
             FROM recovery_blobs ORDER BY created_at",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(RecoveryBlob {
                    recovery_key_id: row.get(0)?,
                    encrypted_data: row.get(1)?,
                    encrypted_data_key: row.get(2)?,
                    master_key_fingerprint: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| SealboxError::DatabaseError(e.to_string()))?;
        Ok(rows)
    }

    fn remove(&self, recovery_key_id: &Uuid) -> Result<()> {
        let guard = self.conn.lock()?;
        guard.execute(
            "DELETE FROM recovery_blobs WHERE recovery_key_id = ?1",
            [recovery_key_id],
        )?;
        Ok(())
    }
}
