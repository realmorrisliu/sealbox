use rusqlite::OptionalExtension;
use std::sync::{Arc, Mutex};
use tracing::info;
use uuid::Uuid;

use crate::{
    error::{Result, SealboxError},
    repo::{MasterKey, MasterKeyRepo, MasterKeyStatus},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteMasterKeyRepo {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteMasterKeyRepo {
    pub(crate) fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }
}

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
                server_held INTEGER NOT NULL DEFAULT 0
            )",
            (),
        )?;

        Self::add_server_held_column(conn)?;

        Ok(())
    }

    /// Whether the server holds this key's private half, which is what makes secrets encrypted
    /// under it readable by the broker (ADR 0001). Idempotent.
    fn add_server_held_column(conn: &rusqlite::Connection) -> Result<()> {
        if super::has_column(conn, "master_keys", "server_held")? {
            return Ok(());
        }
        conn.execute(
            "ALTER TABLE master_keys ADD COLUMN server_held INTEGER NOT NULL DEFAULT 0",
            (),
        )?;
        Ok(())
    }
}

impl MasterKeyRepo for SqliteMasterKeyRepo {
    fn create_master_key(&self, key: &MasterKey) -> Result<()> {
        let guard = self.conn.lock()?;
        let conn = &*guard;
        conn.execute(
            "INSERT INTO master_keys (
                id,
                public_key,
                created_at,
                status,
                description,
                metadata,
                server_held
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                &key.id,
                &key.public_key,
                &key.created_at,
                &key.status,
                &key.description,
                &key.metadata,
                &key.server_held,
            ),
        )?;
        Ok(())
    }

    fn fetch_master_key(&self, master_key_id: &Uuid) -> Result<Option<MasterKey>> {
        let guard = self.conn.lock()?;
        let conn = &*guard;
        let mut stmt = conn.prepare(
            "SELECT id, public_key, created_at, status, description, metadata, server_held
             FROM master_keys WHERE id = ?1 LIMIT 1",
        )?;
        let master_key = stmt
            .query_one([master_key_id], |row| {
                Ok(MasterKey {
                    id: row.get(0)?,
                    public_key: row.get(1)?,
                    created_at: row.get(2)?,
                    status: row.get(3)?,
                    description: row.get(4)?,
                    metadata: row.get(5)?,
                    server_held: row.get(6)?,
                })
            })
            .optional()?;
        Ok(master_key)
    }

    fn ensure_server_held(&self, public_key_pem: &str) -> Result<MasterKey> {
        {
            let guard = self.conn.lock()?;
            let conn = &*guard;
            let mut stmt = conn.prepare(
                "SELECT id, public_key, created_at, status, description, metadata, server_held
                 FROM master_keys WHERE public_key = ?1 AND server_held = 1 LIMIT 1",
            )?;
            let existing = stmt
                .query_one([public_key_pem], |row| {
                    Ok(MasterKey {
                        id: row.get(0)?,
                        public_key: row.get(1)?,
                        created_at: row.get(2)?,
                        status: row.get(3)?,
                        description: row.get(4)?,
                        metadata: row.get(5)?,
                        server_held: row.get(6)?,
                    })
                })
                .optional()?;
            if let Some(key) = existing {
                return Ok(key);
            }
        }

        let key = MasterKey::server_held(public_key_pem.to_string())?;
        self.create_master_key(&key)?;
        info!("Registered the server's master key: {}", key.id);
        Ok(key)
    }

    fn get_valid_master_key(&self) -> Result<MasterKey> {
        let guard = self.conn.lock()?;
        let conn = &*guard;
        // Only a server-held key: new secrets must be readable by the broker. A cold key can
        // still be registered and used as a rekey destination, but never as the current key.
        let mut stmt = conn.prepare(
            "SELECT id, public_key, created_at, status, description, metadata, server_held
             FROM master_keys WHERE status = ?1 AND server_held = 1 LIMIT 1",
        )?;
        let master_key = stmt
            .query_one([MasterKeyStatus::Active], |row| {
                Ok(MasterKey {
                    id: row.get(0)?,
                    public_key: row.get(1)?,
                    created_at: row.get(2)?,
                    status: row.get(3)?,
                    description: row.get(4)?,
                    metadata: row.get(5)?,
                    server_held: row.get(6)?,
                })
            })
            .optional()?;

        if let Some(master_key) = master_key {
            Ok(master_key)
        } else {
            Err(SealboxError::MissingValidMasterKey)
        }
    }

    fn fetch_all_master_keys(&self) -> Result<Vec<MasterKey>> {
        let guard = self.conn.lock()?;
        let conn = &*guard;
        let mut stmt = conn.prepare(
            "SELECT id, created_at, status, description, metadata, server_held FROM master_keys",
        )?;
        let master_key_iter = stmt.query_map([], |row| {
            Ok(MasterKey {
                id: row.get(0)?,
                public_key: "[HIDDEN]".to_string(),
                created_at: row.get(1)?,
                status: row.get(2)?,
                description: row.get(3)?,
                metadata: row.get(4)?,
                server_held: row.get(5)?,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::master_key::generate_key_pair;

    #[test]
    fn test_ensure_server_held_is_idempotent_and_excludes_cold_keys() {
        let conn = rusqlite::Connection::open_in_memory().expect("Should create in-memory DB");
        SqliteMasterKeyRepo::init_table(&conn).expect("Should init table");
        let repo = SqliteMasterKeyRepo::new(Arc::new(Mutex::new(conn)));

        // A client-registered key: public half only, so it is cold.
        let (_, cold_public) = generate_key_pair().expect("Should generate a key pair");
        let cold = MasterKey::new(cold_public).expect("Should build a cold master key");
        repo.create_master_key(&cold).expect("Should register it");

        // With only a cold key present there is nothing to encrypt new secrets under.
        assert!(
            repo.get_valid_master_key().is_err(),
            "a cold key must never be offered as the current key"
        );

        let (_, server_public) = generate_key_pair().expect("Should generate a key pair");
        let first = repo
            .ensure_server_held(&server_public)
            .expect("Should register the server key");
        assert!(first.server_held);

        // Restarting must reuse it rather than register a duplicate.
        let second = repo
            .ensure_server_held(&server_public)
            .expect("Should find the existing key");
        assert_eq!(first.id, second.id);

        let current = repo
            .get_valid_master_key()
            .expect("Should now have a current key");
        assert_eq!(current.id, first.id);
        assert!(current.server_held);
    }

    #[test]
    fn test_private_key_accepts_both_pem_encodings() {
        use crate::crypto::master_key::PrivateMasterKey;
        use std::str::FromStr;

        // PKCS#1 is what generate_key_pair emits.
        let (pkcs1_pem, _) = generate_key_pair().expect("Should generate a key pair");
        assert!(pkcs1_pem.contains("BEGIN RSA PRIVATE KEY"));
        let from_pkcs1 = PrivateMasterKey::from_str(&pkcs1_pem).expect("Should parse PKCS#1");

        // OpenSSL 3 emits PKCS#8 by default, which operators will hit first.
        let pkcs8_pem = {
            use rsa::pkcs1::DecodeRsaPrivateKey;
            use rsa::pkcs8::{EncodePrivateKey, LineEnding};
            let key = rsa::RsaPrivateKey::from_pkcs1_pem(&pkcs1_pem).expect("Should decode");
            key.to_pkcs8_pem(LineEnding::LF)
                .expect("Should encode PKCS#8")
                .to_string()
        };
        assert!(pkcs8_pem.contains("BEGIN PRIVATE KEY"));
        let from_pkcs8 = PrivateMasterKey::from_str(&pkcs8_pem).expect("Should parse PKCS#8");

        // Both encodings describe the same key.
        assert_eq!(
            from_pkcs1.public_key_pem().expect("Should export"),
            from_pkcs8.public_key_pem().expect("Should export")
        );
    }
}
