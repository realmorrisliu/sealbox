use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::{
    error::Result,
    repo::{SecretMasterKeyAssociation, SecretMasterKeyRepo},
};

#[derive(Debug, Clone)]
#[allow(dead_code)] // Used in Phase 2 TDD
pub(crate) struct SqliteSecretMasterKeyRepo;

impl SecretMasterKeyRepo for SqliteSecretMasterKeyRepo {
    fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS secret_master_keys (
                secret_key TEXT NOT NULL,
                secret_version INTEGER NOT NULL,
                master_key_id BLOB NOT NULL,
                encrypted_data_key BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (secret_key, secret_version, master_key_id)
            )",
            (),
        )?;
        Ok(())
    }

    fn create_association(
        &self,
        conn: &rusqlite::Connection,
        secret_key: &str,
        secret_version: i32,
        master_key_id: &Uuid,
        encrypted_data_key: &[u8],
    ) -> Result<()> {
        let created_at = time::OffsetDateTime::now_utc().unix_timestamp();
        
        conn.execute(
            "INSERT INTO secret_master_keys (
                secret_key,
                secret_version,
                master_key_id,
                encrypted_data_key,
                created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                secret_key,
                secret_version,
                master_key_id,
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
    ) -> Result<Vec<SecretMasterKeyAssociation>> {
        let mut stmt = conn.prepare(
            "SELECT secret_key, secret_version, master_key_id, encrypted_data_key, created_at 
             FROM secret_master_keys 
             WHERE secret_key = ?1 AND secret_version = ?2"
        )?;
        
        let association_iter = stmt.query_map((secret_key, secret_version), |row| {
            Ok(SecretMasterKeyAssociation {
                secret_key: row.get(0)?,
                secret_version: row.get(1)?,
                master_key_id: row.get(2)?,
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
        master_key_id: &Uuid,
    ) -> Result<Option<SecretMasterKeyAssociation>> {
        let mut stmt = conn.prepare(
            "SELECT secret_key, secret_version, master_key_id, encrypted_data_key, created_at 
             FROM secret_master_keys 
             WHERE secret_key = ?1 AND secret_version = ?2 AND master_key_id = ?3"
        )?;
        
        let association = stmt
            .query_row((secret_key, secret_version, master_key_id), |row| {
                Ok(SecretMasterKeyAssociation {
                    secret_key: row.get(0)?,
                    secret_version: row.get(1)?,
                    master_key_id: row.get(2)?,
                    encrypted_data_key: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .optional()?;
        
        Ok(association)
    }
}