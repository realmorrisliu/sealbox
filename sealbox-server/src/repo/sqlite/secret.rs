use std::sync::{Arc, Mutex};

use rusqlite::OptionalExtension;
use tracing::info;
use uuid::Uuid;

use crate::{
    error::{Result, SealboxError},
    repo::{Secret, SecretRepo},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteSecretRepo {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteSecretRepo {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Result<Self> {
        let temp_conn = conn.clone();
        let acquired_conn = temp_conn.lock().unwrap();

        // Initialize database table structure
        acquired_conn.execute(
            "CREATE TABLE IF NOT EXISTS secrets (
                namespace TEXT NOT NULL,
                key TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                encrypted_data BLOB NOT NULL,
                encrypted_data_key BLOB NOT NULL,
                master_key_id BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                expires_at INTEGER,
                metadata TEXT,
                PRIMARY KEY (namespace, key, version)
            )",
            (),
        )?;

        Ok(Self { conn })
    }
}

impl SecretRepo for SqliteSecretRepo {
    fn get_secret(&self, key: &str) -> Result<Secret> {
        info!("get_secret");
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT
                namespace,
                key,
                version,
                encrypted_data,
                encrypted_data_key,
                master_key_id,
                created_at,
                updated_at,
                expires_at,
                metadata
            FROM secrets
            WHERE key = ?1
            LIMIT 1",
        )?;
        let row = stmt
            .query_one([key], |row| {
                Ok(Secret {
                    namespace: row.get(0)?,
                    key: row.get(1)?,
                    version: row.get(2)?,
                    encrypted_data: row.get(3)?,
                    encrypted_data_key: row.get(4)?,
                    master_key_id: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    expires_at: row.get(8)?,
                    metadata: row.get(9)?,
                })
            })
            .optional()?;

        match row {
            Some(secret) => Ok(secret),
            None => Err(SealboxError::SecretNotFound(key.to_string())),
        }
    }

    fn save_secret(&self, secret: &Secret) -> Result<()> {
        info!("save_secret");
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO secrets (
              namespace,
              key,
              version,
              encrypted_data,
              encrypted_data_key,
              master_key_id,
              created_at,
              updated_at,
              expires_at,
              metadata
          ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            (
                &secret.namespace,
                &secret.key,
                &secret.version,
                &secret.encrypted_data,
                &secret.encrypted_data_key,
                &secret.master_key_id,
                &secret.created_at,
                &secret.updated_at,
                &secret.expires_at,
                &secret.metadata,
            ),
        )?;

        Ok(())
    }

    fn delete_secret(&self, key: &str) -> Result<()> {
        info!("delete_secret");
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM secrets WHERE key = ?1", [key])?;
        Ok(())
    }

    fn fetch_secrets_by_master_key(&self, master_key_id: &Uuid) -> Result<Vec<Secret>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT
                namespace,
                key,
                version,
                encrypted_data,
                encrypted_data_key,
                master_key_id,
                created_at,
                updated_at,
                expires_at,
                metadata
            FROM secrets
            WHERE master_key_id = ?1",
        )?;
        let secrets: Vec<Secret> = stmt
            .query_map([master_key_id], |row| {
                Ok(Secret {
                    namespace: row.get(0)?,
                    key: row.get(1)?,
                    version: row.get(2)?,
                    encrypted_data: row.get(3)?,
                    encrypted_data_key: row.get(4)?,
                    master_key_id: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    expires_at: row.get(8)?,
                    metadata: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(secrets)
    }

    fn update_secret_master_key(&self, secret: &Secret) -> Result<()> {
        use rusqlite::params;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE secrets SET
                encrypted_data_key = ?1,
                master_key_id = ?2,
                updated_at = ?3
             WHERE namespace = ?4 AND key = ?5 AND version = ?6",
            params![
                &secret.encrypted_data_key,
                &secret.master_key_id,
                &secret.updated_at,
                &secret.namespace,
                &secret.key,
                &secret.version,
            ],
        )?;
        Ok(())
    }
}
