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
                namespace TEXT NOT NULL DEFAULT 'legacy',
                id BLOB PRIMARY KEY,
                public_key TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL,
                description TEXT,
                version INTEGER,
                metadata TEXT
            )",
            (),
        )?;
        let has_namespace = conn
            .prepare("PRAGMA table_info(master_keys)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<std::result::Result<Vec<_>, _>>()?
            .iter()
            .any(|column| column == "namespace");
        if !has_namespace {
            conn.execute(
                "ALTER TABLE master_keys ADD COLUMN namespace TEXT NOT NULL DEFAULT 'legacy'",
                (),
            )?;
        }
        conn.execute(
            "UPDATE master_keys
             SET status = 'Retired'
             WHERE status = 'Active'
               AND id NOT IN (
                   SELECT scoped.id
                   FROM master_keys AS scoped
                   WHERE scoped.status = 'Active'
                     AND scoped.namespace = master_keys.namespace
                   ORDER BY created_at DESC, id DESC
                   LIMIT 1
               )",
            (),
        )?;
        conn.execute("DROP INDEX IF EXISTS idx_master_keys_one_active", ())?;
        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_master_keys_one_active_per_namespace
             ON master_keys(namespace, status)
             WHERE status = 'Active'",
            (),
        )?;
        Ok(())
    }
}

impl MasterKeyRepo for SqliteMasterKeyRepo {
    fn create_master_key(&self, conn: &rusqlite::Connection, key: &MasterKey) -> Result<()> {
        conn.execute(
            "INSERT INTO master_keys (
                namespace,
                id,
                public_key,
                created_at,
                status,
                description,
                metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                &key.namespace,
                &key.id,
                &key.public_key,
                &key.created_at,
                &key.status,
                &key.description,
                &key.metadata,
            ),
        )?;
        Ok(())
    }

    fn fetch_master_key(
        &self,
        conn: &rusqlite::Connection,
        namespace: &str,
        master_key_id: &Uuid,
    ) -> Result<Option<MasterKey>> {
        let mut stmt = conn.prepare(
            "SELECT namespace, id, public_key, created_at, status, description, metadata
             FROM master_keys
             WHERE namespace = ?1 AND id = ?2
             LIMIT 1",
        )?;
        let master_key = stmt
            .query_one(rusqlite::params![namespace, master_key_id], |row| {
                Ok(MasterKey {
                    namespace: row.get(0)?,
                    id: row.get(1)?,
                    public_key: row.get(2)?,
                    created_at: row.get(3)?,
                    status: row.get(4)?,
                    description: row.get(5)?,
                    metadata: row.get(6)?,
                })
            })
            .optional()?;
        Ok(master_key)
    }

    fn get_valid_master_key(
        &self,
        conn: &rusqlite::Connection,
        namespace: &str,
    ) -> Result<MasterKey> {
        let mut stmt = conn.prepare(
            "SELECT namespace, id, public_key, created_at, status, description, metadata
             FROM master_keys
             WHERE namespace = ?1 AND status = ?2
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
        )?;
        let master_key = stmt
            .query_one(
                rusqlite::params![namespace, MasterKeyStatus::Active],
                |row| {
                    Ok(MasterKey {
                        namespace: row.get(0)?,
                        id: row.get(1)?,
                        public_key: row.get(2)?,
                        created_at: row.get(3)?,
                        status: row.get(4)?,
                        description: row.get(5)?,
                        metadata: row.get(6)?,
                    })
                },
            )
            .optional()?;

        if let Some(master_key) = master_key {
            Ok(master_key)
        } else {
            Err(SealboxError::MissingValidMasterKey)
        }
    }

    fn fetch_all_master_keys(
        &self,
        conn: &rusqlite::Connection,
        namespace: &str,
    ) -> Result<Vec<MasterKey>> {
        let mut stmt = conn.prepare(
            "SELECT namespace, id, created_at, status, description, metadata
             FROM master_keys WHERE namespace = ?1",
        )?;
        let master_key_iter = stmt.query_map([namespace], |row| {
            Ok(MasterKey {
                namespace: row.get(0)?,
                id: row.get(1)?,
                public_key: "[HIDDEN]".to_string(),
                created_at: row.get(2)?,
                status: row.get(3)?,
                description: row.get(4)?,
                metadata: row.get(5)?,
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
