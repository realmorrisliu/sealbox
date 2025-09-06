use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::{
    error::{Result, SealboxError},
    repo::{SecretClientKeyAssociation, SecretClientKeyRepo},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteSecretClientKeyRepo;

impl SecretClientKeyRepo for SqliteSecretClientKeyRepo {
    fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS secret_client_keys (
                secret_key TEXT NOT NULL,
                secret_version INTEGER NOT NULL,
                client_key_id BLOB NOT NULL,
                encrypted_data_key BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (secret_key, secret_version, client_key_id)
            )",
            (),
        )?;
        
        // Create indexes for performance optimization
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_secret_client_keys_client_key 
             ON secret_client_keys(client_key_id, secret_key)",
            (),
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_secret_client_keys_secret_client 
             ON secret_client_keys(secret_key, secret_version, client_key_id)",
            (),
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_secret_client_keys_created_at 
             ON secret_client_keys(created_at)",
            (),
        )?;
        
        Ok(())
    }

    fn create_association(
        &self,
        conn: &rusqlite::Connection,
        secret_key: &str,
        secret_version: i32,
        client_key_id: &Uuid,
        encrypted_data_key: &[u8],
    ) -> Result<()> {
        let created_at = time::OffsetDateTime::now_utc().unix_timestamp();

        conn.execute(
            "INSERT INTO secret_client_keys (
                secret_key,
                secret_version,
                client_key_id,
                encrypted_data_key,
                created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                secret_key,
                secret_version,
                client_key_id,
                encrypted_data_key,
                created_at,
            ),
        )?;
        Ok(())
    }

    fn get_associations_for_secret(
        &self,
        conn: &rusqlite::Connection,
        secret_key: &str,
        secret_version: i32,
    ) -> Result<Vec<SecretClientKeyAssociation>> {
        let mut stmt = conn.prepare(
            "SELECT secret_key, secret_version, client_key_id, encrypted_data_key, created_at 
             FROM secret_client_keys 
             WHERE secret_key = ?1 AND secret_version = ?2",
        )?;

        let association_iter = stmt.query_map((secret_key, secret_version), |row| {
            Ok(SecretClientKeyAssociation {
                secret_key: row.get(0)?,
                secret_version: row.get(1)?,
                client_key_id: row.get(2)?,
                encrypted_data_key: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let associations: Vec<_> = association_iter
            .filter_map(|res| {
                res.map_err(|err| tracing::error!("Failed to fetch association: {}", err))
                    .ok()
            })
            .collect();

        Ok(associations)
    }

    fn get_association(
        &self,
        conn: &rusqlite::Connection,
        secret_key: &str,
        secret_version: i32,
        client_key_id: &Uuid,
    ) -> Result<Option<SecretClientKeyAssociation>> {
        let mut stmt = conn.prepare(
            "SELECT secret_key, secret_version, client_key_id, encrypted_data_key, created_at 
             FROM secret_client_keys 
             WHERE secret_key = ?1 AND secret_version = ?2 AND client_key_id = ?3",
        )?;

        let association = stmt
            .query_row((secret_key, secret_version, client_key_id), |row| {
                Ok(SecretClientKeyAssociation {
                    secret_key: row.get(0)?,
                    secret_version: row.get(1)?,
                    client_key_id: row.get(2)?,
                    encrypted_data_key: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .optional()?;

        Ok(association)
    }

    fn remove_association(
        &self,
        conn: &rusqlite::Connection,
        secret_key: &str,
        secret_version: i32,
        client_key_id: &Uuid,
    ) -> Result<()> {
        let changed = conn.execute(
            "DELETE FROM secret_client_keys 
             WHERE secret_key = ?1 AND secret_version = ?2 AND client_key_id = ?3",
            (secret_key, secret_version, client_key_id),
        )?;

        if changed == 0 {
            return Err(SealboxError::ClientKeyNotFound(*client_key_id));
        }

        Ok(())
    }
}
