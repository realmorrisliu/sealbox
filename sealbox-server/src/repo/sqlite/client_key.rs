use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::{
    error::{Result, SealboxError},
    repo::{ClientKey, ClientKeyRepo, ClientKeyStatus},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteClientKeyRepo;

impl SqliteClientKeyRepo {
    pub fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        // Initialize database table structure
        conn.execute(
            "CREATE TABLE IF NOT EXISTS client_keys (
                id BLOB PRIMARY KEY,
                public_key TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL,
                description TEXT,
                version INTEGER,
                metadata TEXT,
                name TEXT,
                last_used_at INTEGER
            )",
            (),
        )?;
        
        // Create indexes for performance optimization
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_client_keys_status 
             ON client_keys(status)",
            (),
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_client_keys_created_at 
             ON client_keys(created_at)",
            (),
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_client_keys_last_used_at 
             ON client_keys(last_used_at)",
            (),
        )?;
        
        Ok(())
    }
}

impl ClientKeyRepo for SqliteClientKeyRepo {
    fn create_client_key(&self, conn: &rusqlite::Connection, key: &ClientKey) -> Result<()> {
        conn.execute(
            "INSERT INTO client_keys (
                id,
                public_key,
                created_at,
                status,
                description,
                metadata,
                name,
                last_used_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                &key.id,
                &key.public_key,
                &key.created_at,
                &key.status,
                &key.description,
                &key.metadata,
                &key.name,
                &key.last_used_at,
            ),
        )?;
        Ok(())
    }

    fn fetch_client_key(
        &self,
        conn: &rusqlite::Connection,
        client_key_id: &Uuid,
    ) -> Result<Option<ClientKey>> {
        let mut stmt = conn.prepare("SELECT id, public_key, created_at, status, description, metadata, name, last_used_at FROM client_keys WHERE id = ?1 LIMIT 1")?;
        let client_key = stmt
            .query_one([client_key_id], |row| {
                Ok(ClientKey {
                    id: row.get(0)?,
                    public_key: row.get(1)?,
                    created_at: row.get(2)?,
                    status: row.get(3)?,
                    description: row.get(4)?,
                    metadata: row.get(5)?,
                    name: row.get(6)?,
                    last_used_at: row.get(7)?,
                })
            })
            .optional()?;
        Ok(client_key)
    }

    fn fetch_public_key(
        &self,
        conn: &rusqlite::Connection,
        client_key_id: &Uuid,
    ) -> Result<Option<String>> {
        let mut stmt = conn.prepare("SELECT public_key FROM client_keys WHERE id = ?1 LIMIT 1")?;
        let public_key = stmt
            .query_one([client_key_id], |row| row.get(0))
            .optional()?;
        Ok(public_key)
    }

    fn get_valid_client_key(&self, conn: &rusqlite::Connection) -> Result<ClientKey> {
        let mut stmt = conn.prepare("SELECT id, public_key, created_at, status, description, metadata, name, last_used_at FROM client_keys WHERE status = ?1 LIMIT 1")?;
        let client_key = stmt
            .query_one([ClientKeyStatus::Active], |row| {
                Ok(ClientKey {
                    id: row.get(0)?,
                    public_key: row.get(1)?,
                    created_at: row.get(2)?,
                    status: row.get(3)?,
                    description: row.get(4)?,
                    metadata: row.get(5)?,
                    name: row.get(6)?,
                    last_used_at: row.get(7)?,
                })
            })
            .optional()?;

        if let Some(client_key) = client_key {
            Ok(client_key)
        } else {
            Err(SealboxError::MissingValidClientKey)
        }
    }

    fn fetch_all_client_keys(&self, conn: &rusqlite::Connection) -> Result<Vec<ClientKey>> {
        let mut stmt = conn.prepare(
            "SELECT id, created_at, status, description, metadata, name, last_used_at FROM client_keys",
        )?;
        let client_key_iter = stmt.query_map([], |row| {
            Ok(ClientKey {
                id: row.get(0)?,
                public_key: "[HIDDEN]".to_string(),
                created_at: row.get(1)?,
                status: row.get(2)?,
                description: row.get(3)?,
                metadata: row.get(4)?,
                name: row.get(5)?,
                last_used_at: row.get(6)?,
            })
        })?;

        let client_keys: Vec<_> = client_key_iter
            .filter_map(|res| {
                res.map_err(|err| tracing::error!("Failed to fetch client key: {}", err))
                    .ok()
            })
            .collect();

        Ok(client_keys)
    }
    
    fn update_last_used(&self, conn: &rusqlite::Connection, client_key_id: &Uuid) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        conn.execute(
            "UPDATE client_keys SET last_used_at = ?1 WHERE id = ?2",
            (now, client_key_id),
        )?;
        Ok(())
    }
}
