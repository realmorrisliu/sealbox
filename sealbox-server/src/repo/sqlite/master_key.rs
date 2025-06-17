use std::sync::{Arc, Mutex};

use rusqlite::OptionalExtension;

use crate::{
    error::Result,
    repo::{MasterKey, MasterKeyRepo},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteMasterKeyRepo {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteMasterKeyRepo {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Result<Self> {
        let temp_conn = conn.clone();
        let acquired_conn = temp_conn.lock().unwrap();

        // Initialize database table structure
        acquired_conn.execute(
            "CREATE TABLE IF NOT EXISTS master_keys (
                id TEXT PRIMARY KEY,
                public_key TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL,
                description TEXT,
                version INTEGER,
                metadata TEXT
            )",
            (),
        )?;

        Ok(Self { conn })
    }
}

impl MasterKeyRepo for SqliteMasterKeyRepo {
    fn create_master_key(&self, key: &MasterKey) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO master_keys (
                id,
                public_key,
                created_at,
                status,
                description,
                version,
                metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                &key.id,
                &key.public_key,
                &key.created_at,
                &key.status,
                &key.description,
                &key.version,
                &key.metadata,
            ),
        )?;
        Ok(())
    }

    fn delete_master_key(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM master_keys WHERE id = ?1", [id])?;
        Ok(())
    }

    fn fetch_public_key(&self, master_key_id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT public_key FROM master_keys WHERE id = ?1 LIMIT 1")?;
        let public_key = stmt
            .query_one([master_key_id], |row| row.get(0))
            .optional()?;
        Ok(public_key)
    }

    fn get_valid_master_key(&self) -> Result<Option<MasterKey>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM master_keys WHERE status = 'VALID' LIMIT 1")?;
        let master_key = stmt
            .query_one([], |row| {
                Ok(MasterKey {
                    id: row.get(0)?,
                    public_key: row.get(1)?,
                    created_at: row.get(2)?,
                    status: row.get(3)?,
                    description: row.get(4)?,
                    version: row.get(5)?,
                    metadata: row.get(6)?,
                })
            })
            .optional()?;
        Ok(master_key)
    }
}
