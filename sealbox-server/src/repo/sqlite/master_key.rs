use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::{
    error::{Result, SealboxError},
    repo::{MasterKey, MasterKeyRepo, MasterKeyStatus},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteMasterKeyRepo;

impl SqliteMasterKeyRepo {
    pub fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        // Initialize database table structure
        conn.execute(
            "CREATE TABLE IF NOT EXISTS master_keys (
                id BLOB PRIMARY KEY,
                public_key TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL,
                description TEXT,
                version INTEGER,
                metadata TEXT,
                name TEXT
            )",
            (),
        )?;
        Ok(())
    }
}

impl MasterKeyRepo for SqliteMasterKeyRepo {
    fn create_master_key(&self, conn: &rusqlite::Connection, key: &MasterKey) -> Result<()> {
        conn.execute(
            "INSERT INTO master_keys (
                id,
                public_key,
                created_at,
                status,
                description,
                metadata,
                name
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                &key.id,
                &key.public_key,
                &key.created_at,
                &key.status,
                &key.description,
                &key.metadata,
                &key.name,
            ),
        )?;
        Ok(())
    }

    fn fetch_public_key(
        &self,
        conn: &rusqlite::Connection,
        master_key_id: &Uuid,
    ) -> Result<Option<String>> {
        let mut stmt = conn.prepare("SELECT public_key FROM master_keys WHERE id = ?1 LIMIT 1")?;
        let public_key = stmt
            .query_one([master_key_id], |row| row.get(0))
            .optional()?;
        Ok(public_key)
    }

    fn get_valid_master_key(&self, conn: &rusqlite::Connection) -> Result<MasterKey> {
        let mut stmt = conn.prepare("SELECT id, public_key, created_at, status, description, metadata, name FROM master_keys WHERE status = ?1 LIMIT 1")?;
        let master_key = stmt
            .query_one([MasterKeyStatus::Active], |row| {
                Ok(MasterKey {
                    id: row.get(0)?,
                    public_key: row.get(1)?,
                    created_at: row.get(2)?,
                    status: row.get(3)?,
                    description: row.get(4)?,
                    metadata: row.get(5)?,
                    name: row.get(6)?,
                })
            })
            .optional()?;

        if let Some(master_key) = master_key {
            Ok(master_key)
        } else {
            Err(SealboxError::MissingValidMasterKey)
        }
    }

    fn fetch_all_master_keys(&self, conn: &rusqlite::Connection) -> Result<Vec<MasterKey>> {
        let mut stmt =
            conn.prepare("SELECT id, created_at, status, description, metadata, name FROM master_keys")?;
        let master_key_iter = stmt.query_map([], |row| {
            Ok(MasterKey {
                id: row.get(0)?,
                public_key: "[HIDDEN]".to_string(),
                created_at: row.get(1)?,
                status: row.get(2)?,
                description: row.get(3)?,
                metadata: row.get(4)?,
                name: row.get(5)?,
            })
        })?;

        let master_keys: Vec<_> = master_key_iter
            .filter_map(|res| {
                res.map_err(|err| tracing::error!("Failed to fetch master key: {}", err))
                    .ok()
            })
            .collect();

        Ok(master_keys)
    }
}
