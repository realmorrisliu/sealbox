use serde_rusqlite::*;
use tracing::info;
use uuid::Uuid;

use crate::{
    error::{Result, SealboxError},
    repo::{Secret, SecretRepo},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteSecretRepo;

impl SqliteSecretRepo {
    pub fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        // Initialize database table structure
        conn.execute(
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

        Ok(())
    }
}

impl SqliteSecretRepo {
    /// Helper function to check expiry and clean up expired secrets atomically
    fn check_and_cleanup_expired(
        tx: &rusqlite::Transaction,
        secret: &Secret,
    ) -> Result<Option<Secret>> {
        if let Some(expires_at) = secret.expires_at {
            let now = time::OffsetDateTime::now_utc().unix_timestamp();
            if expires_at < now {
                // Secret has expired, delete it atomically within transaction
                tx.execute(
                    "DELETE FROM secrets WHERE key = ?1 AND version = ?2",
                    [&secret.key, &secret.version.to_string()],
                )?;
                info!(
                    "Secret '{}' version {} has expired and been deleted",
                    secret.key, secret.version
                );
                return Ok(None);
            }
        }
        Ok(Some(secret.clone()))
    }

    /// Common implementation for getting secrets with atomic cleanup
    fn get_secret_with_query(
        &self,
        conn: &mut rusqlite::Connection,
        query: &str,
        params: impl rusqlite::Params,
        key: &str,
    ) -> Result<Secret> {
        let tx = conn.transaction()?;

        let row = {
            let mut stmt = tx.prepare_cached(query)?;
            // Using query_and_then() and from_row() as shown in the official example
            let mut rows = stmt.query_and_then(params, from_row::<Secret>)?;
            rows.next()
                .transpose()
                .map_err(|e| SealboxError::DatabaseError(e.to_string()))?
        };

        match row {
            Some(secret) => match Self::check_and_cleanup_expired(&tx, &secret)? {
                Some(valid_secret) => {
                    tx.commit()?;
                    Ok(valid_secret)
                }
                None => {
                    tx.commit()?;
                    Err(SealboxError::SecretNotFound(key.to_string()))
                }
            },
            None => {
                tx.commit()?;
                Err(SealboxError::SecretNotFound(key.to_string()))
            }
        }
    }
}

impl SecretRepo for SqliteSecretRepo {
    fn get_secret(&self, conn: &mut rusqlite::Connection, key: &str) -> Result<Secret> {
        info!("get_secret: key={}", key);

        self.get_secret_with_query(
            conn,
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
            ORDER BY version DESC
            LIMIT 1",
            [key],
            key,
        )
    }

    fn get_secret_by_version(
        &self,
        conn: &mut rusqlite::Connection,
        key: &str,
        version: i32,
    ) -> Result<Secret> {
        info!("get_secret_by_version: key={}, version={}", key, version);

        self.get_secret_with_query(
            conn,
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
            WHERE key = ?1 AND version = ?2
            LIMIT 1",
            (key, version),
            key,
        )
    }

    fn create_new_version(
        &self,
        conn: &mut rusqlite::Connection,
        key: &str,
        data: &str,
        master_key: crate::repo::MasterKey,
        ttl: Option<i64>,
    ) -> Result<Secret> {
        info!("create_new_version");

        let tx = conn.transaction()?;

        let next_version = {
            let mut stmt =
                tx.prepare("SELECT COALESCE(MAX(version), 0) FROM secrets WHERE key = ?1")?;
            let latest_version: i32 = stmt.query_one([key], |row| row.get(0))?;
            latest_version + 1
        };

        let secret = Secret::new(key, data, master_key, next_version, ttl)?;

        tx.execute(
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

        tx.commit()?;

        Ok(secret)
    }

    fn delete_secret_by_version(
        &self,
        conn: &rusqlite::Connection,
        key: &str,
        version: i32,
    ) -> Result<()> {
        info!("delete_secret_by_version");
        let changed = conn.execute(
            "DELETE FROM secrets WHERE key = ?1 AND version = ?2",
            (key, version),
        )?;
        if changed == 0 {
            return Err(SealboxError::SecretNotFound(key.to_string()));
        }
        Ok(())
    }

    fn fetch_secrets_by_master_key(
        &self,
        conn: &rusqlite::Connection,
        master_key_id: &Uuid,
    ) -> Result<Vec<Secret>> {
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
        // Using query() and from_rows(), the most efficient way as shown in the official example
        let rows = stmt.query([master_key_id])?;
        let secrets: Vec<Secret> = from_rows::<Secret>(rows)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| SealboxError::DatabaseError(e.to_string()))?;
        Ok(secrets)
    }

    fn update_secret_master_key(&self, conn: &rusqlite::Connection, secret: &Secret) -> Result<()> {
        conn.execute(
            "UPDATE secrets SET
                encrypted_data_key = ?1,
                master_key_id = ?2,
                updated_at = ?3
             WHERE namespace = ?4 AND key = ?5 AND version = ?6",
            rusqlite::params![
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

    fn cleanup_expired_secrets(&self, conn: &rusqlite::Connection) -> Result<usize> {
        info!("cleanup_expired_secrets");
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let deleted_count = conn.execute(
            "DELETE FROM secrets WHERE expires_at IS NOT NULL AND expires_at < ?1",
            [now],
        )?;
        info!("Cleaned up {} expired secrets", deleted_count);
        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::master_key::generate_key_pair;
    use crate::repo::MasterKey;

    fn setup_test_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().expect("Should create in-memory DB");
        SqliteSecretRepo::init_table(&conn).expect("Should initialize tables");
        conn
    }

    fn create_test_master_key() -> MasterKey {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        MasterKey::new(public_pem).expect("Should create master key")
    }

    #[test]
    fn test_init_table() {
        let conn = setup_test_db();

        // Verify table exists and has correct structure
        let table_info: Vec<String> = conn
            .prepare("PRAGMA table_info(secrets)")
            .expect("Should prepare query")
            .query_map([], |row| {
                let column_name: String = row.get(1)?;
                Ok(column_name)
            })
            .expect("Should execute query")
            .collect::<Result<Vec<_>, _>>()
            .expect("Should collect results");

        let expected_columns = vec![
            "namespace",
            "key",
            "version",
            "encrypted_data",
            "encrypted_data_key",
            "master_key_id",
            "created_at",
            "updated_at",
            "expires_at",
            "metadata",
        ];

        for expected_col in expected_columns {
            assert!(
                table_info.contains(&expected_col.to_string()),
                "Missing column: {expected_col}"
            );
        }
    }

    #[test]
    fn test_create_and_get_secret() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;
        let master_key = create_test_master_key();

        let secret_key = "test-secret";
        let secret_data = "This is secret data";

        // Create secret
        let mut conn_mut = conn;
        let created_secret = repo
            .create_new_version(&mut conn_mut, secret_key, secret_data, master_key, None)
            .expect("Should create secret");

        // Get secret back
        let retrieved_secret = repo
            .get_secret(&mut conn_mut, secret_key)
            .expect("Should retrieve secret");

        assert_eq!(created_secret.key, retrieved_secret.key);
        assert_eq!(created_secret.version, retrieved_secret.version);
        assert_eq!(
            created_secret.encrypted_data,
            retrieved_secret.encrypted_data
        );
        assert_eq!(
            created_secret.encrypted_data_key,
            retrieved_secret.encrypted_data_key
        );
        assert_eq!(created_secret.master_key_id, retrieved_secret.master_key_id);
    }

    #[test]
    fn test_get_secret_not_found() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;

        let mut conn = conn;
        let result = repo.get_secret(&mut conn, "nonexistent-key");
        assert!(result.is_err());

        match result.unwrap_err() {
            SealboxError::SecretNotFound(key) => assert_eq!(key, "nonexistent-key"),
            _ => panic!("Expected SecretNotFound error"),
        }
    }

    #[test]
    fn test_create_multiple_versions() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;
        let master_key = create_test_master_key();

        let secret_key = "test-secret";

        // Create first version
        let mut conn_mut = conn;
        let secret_v1 = repo
            .create_new_version(
                &mut conn_mut,
                secret_key,
                "data version 1",
                master_key.clone(),
                None,
            )
            .expect("Should create version 1");

        // Create second version
        let secret_v2 = repo
            .create_new_version(
                &mut conn_mut,
                secret_key,
                "data version 2",
                master_key,
                None,
            )
            .expect("Should create version 2");

        assert_eq!(secret_v1.version, 1);
        assert_eq!(secret_v2.version, 2);
        assert_ne!(secret_v1.encrypted_data, secret_v2.encrypted_data);

        // Get latest version (should be v2)
        let latest = repo
            .get_secret(&mut conn_mut, secret_key)
            .expect("Should get latest version");
        assert_eq!(latest.version, 2);
        assert_eq!(latest.encrypted_data, secret_v2.encrypted_data);
    }

    #[test]
    fn test_get_secret_by_version() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;
        let master_key = create_test_master_key();

        let secret_key = "test-secret";

        // Create multiple versions
        let mut conn_mut = conn;
        let secret_v1 = repo
            .create_new_version(
                &mut conn_mut,
                secret_key,
                "data version 1",
                master_key.clone(),
                None,
            )
            .expect("Should create version 1");

        let _secret_v2 = repo
            .create_new_version(
                &mut conn_mut,
                secret_key,
                "data version 2",
                master_key,
                None,
            )
            .expect("Should create version 2");

        // Get specific version
        let retrieved_v1 = repo
            .get_secret_by_version(&mut conn_mut, secret_key, 1)
            .expect("Should get version 1");

        assert_eq!(retrieved_v1.version, 1);
        assert_eq!(retrieved_v1.encrypted_data, secret_v1.encrypted_data);
    }

    #[test]
    fn test_get_secret_by_version_not_found() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;

        let mut conn = conn;
        let result = repo.get_secret_by_version(&mut conn, "nonexistent-key", 1);
        assert!(result.is_err());

        match result.unwrap_err() {
            SealboxError::SecretNotFound(key) => assert_eq!(key, "nonexistent-key"),
            _ => panic!("Expected SecretNotFound error"),
        }
    }

    #[test]
    fn test_delete_secret_by_version() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;
        let master_key = create_test_master_key();

        let secret_key = "test-secret";

        // Create multiple versions
        let mut conn_mut = conn;
        let _secret_v1 = repo
            .create_new_version(
                &mut conn_mut,
                secret_key,
                "data version 1",
                master_key.clone(),
                None,
            )
            .expect("Should create version 1");

        let secret_v2 = repo
            .create_new_version(
                &mut conn_mut,
                secret_key,
                "data version 2",
                master_key,
                None,
            )
            .expect("Should create version 2");

        // Delete version 1
        repo.delete_secret_by_version(&conn_mut, secret_key, 1)
            .expect("Should delete version 1");

        // Version 1 should be gone
        let result = repo.get_secret_by_version(&mut conn_mut, secret_key, 1);
        assert!(result.is_err());

        // Version 2 should still exist and be the latest
        let latest = repo
            .get_secret(&mut conn_mut, secret_key)
            .expect("Should get latest version");
        assert_eq!(latest.version, 2);
        assert_eq!(latest.encrypted_data, secret_v2.encrypted_data);
    }

    #[test]
    fn test_delete_secret_by_version_not_found() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;

        let result = repo.delete_secret_by_version(&conn, "nonexistent-key", 1);
        assert!(result.is_err());

        match result.unwrap_err() {
            SealboxError::SecretNotFound(key) => assert_eq!(key, "nonexistent-key"),
            _ => panic!("Expected SecretNotFound error"),
        }
    }

    #[test]
    fn test_fetch_secrets_by_master_key() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;
        let master_key1 = create_test_master_key();
        let master_key2 = create_test_master_key();

        // Create secrets with different master keys
        let mut conn_mut = conn;
        let _secret1 = repo
            .create_new_version(&mut conn_mut, "secret1", "data1", master_key1.clone(), None)
            .expect("Should create secret1");

        let _secret2 = repo
            .create_new_version(&mut conn_mut, "secret2", "data2", master_key1.clone(), None)
            .expect("Should create secret2");

        let _secret3 = repo
            .create_new_version(&mut conn_mut, "secret3", "data3", master_key2.clone(), None)
            .expect("Should create secret3");

        // Fetch secrets by master key 1
        let secrets_mk1 = repo
            .fetch_secrets_by_master_key(&conn_mut, &master_key1.id)
            .expect("Should fetch secrets for master key 1");

        assert_eq!(secrets_mk1.len(), 2);
        assert!(
            secrets_mk1
                .iter()
                .all(|s| s.master_key_id == master_key1.id)
        );

        // Fetch secrets by master key 2
        let secrets_mk2 = repo
            .fetch_secrets_by_master_key(&conn_mut, &master_key2.id)
            .expect("Should fetch secrets for master key 2");

        assert_eq!(secrets_mk2.len(), 1);
        assert_eq!(secrets_mk2[0].master_key_id, master_key2.id);
    }

    #[test]
    fn test_update_secret_master_key() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;
        let master_key = create_test_master_key();

        // Create a secret
        let mut conn_mut = conn;
        let mut secret = repo
            .create_new_version(&mut conn_mut, "test-secret", "test-data", master_key, None)
            .expect("Should create secret");

        // Modify the secret
        let new_master_key = create_test_master_key();
        secret.master_key_id = new_master_key.id;
        secret.encrypted_data_key = vec![1, 2, 3, 4]; // Dummy new encrypted key
        secret.updated_at = time::OffsetDateTime::now_utc().unix_timestamp();

        // Update in database
        repo.update_secret_master_key(&conn_mut, &secret)
            .expect("Should update secret");

        // Verify the update
        let updated_secret = repo
            .get_secret(&mut conn_mut, "test-secret")
            .expect("Should retrieve updated secret");

        assert_eq!(updated_secret.master_key_id, new_master_key.id);
        assert_eq!(updated_secret.encrypted_data_key, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_secret_with_ttl() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;
        let master_key = create_test_master_key();

        let ttl = Some(3600i64); // 1 hour

        // Create secret with TTL
        let mut conn_mut = conn;
        let secret = repo
            .create_new_version(
                &mut conn_mut,
                "ttl-secret",
                "temporary-data",
                master_key,
                ttl,
            )
            .expect("Should create secret with TTL");

        assert!(secret.expires_at.is_some());
        let expected_expiry = secret.created_at + 3600;
        assert_eq!(secret.expires_at, Some(expected_expiry));

        // Retrieve and verify TTL is preserved
        let retrieved = repo
            .get_secret(&mut conn_mut, "ttl-secret")
            .expect("Should retrieve secret");
        assert_eq!(retrieved.expires_at, secret.expires_at);
    }

    #[test]
    fn test_expired_secret_not_retrievable() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;
        let master_key = create_test_master_key();

        // Create a secret that expires immediately (TTL = 1 second)
        let mut conn_mut = conn;
        let _secret = repo
            .create_new_version(
                &mut conn_mut,
                "expired-secret",
                "temporary-data",
                master_key,
                Some(1i64), // 1 second
            )
            .expect("Should create secret with short TTL");

        // Wait for the secret to expire
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Try to retrieve the expired secret
        let result = repo.get_secret(&mut conn_mut, "expired-secret");
        assert!(result.is_err());

        match result.unwrap_err() {
            SealboxError::SecretNotFound(key) => assert_eq!(key, "expired-secret"),
            _ => panic!("Expected SecretNotFound error"),
        }
    }

    #[test]
    fn test_expired_secret_by_version_not_retrievable() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;
        let master_key = create_test_master_key();

        // Create a secret that expires immediately
        let mut conn_mut = conn;
        let secret = repo
            .create_new_version(
                &mut conn_mut,
                "expired-secret-v",
                "temporary-data",
                master_key,
                Some(1i64), // 1 second
            )
            .expect("Should create secret with short TTL");

        // Wait for the secret to expire
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Try to retrieve the expired secret by version
        let result = repo.get_secret_by_version(&mut conn_mut, "expired-secret-v", secret.version);
        assert!(result.is_err());

        match result.unwrap_err() {
            SealboxError::SecretNotFound(key) => assert_eq!(key, "expired-secret-v"),
            _ => panic!("Expected SecretNotFound error"),
        }
    }

    #[test]
    fn test_cleanup_expired_secrets() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;
        let master_key = create_test_master_key();

        let mut conn_mut = conn;

        // Create several secrets: some expired, some not
        let _expired1 = repo
            .create_new_version(
                &mut conn_mut,
                "expired1",
                "data1",
                master_key.clone(),
                Some(1i64), // 1 second
            )
            .expect("Should create expired secret 1");

        let _expired2 = repo
            .create_new_version(
                &mut conn_mut,
                "expired2",
                "data2",
                master_key.clone(),
                Some(1i64), // 1 second
            )
            .expect("Should create expired secret 2");

        let _permanent = repo
            .create_new_version(
                &mut conn_mut,
                "permanent",
                "permanent-data",
                master_key.clone(),
                None, // No TTL
            )
            .expect("Should create permanent secret");

        let _long_lived = repo
            .create_new_version(
                &mut conn_mut,
                "long-lived",
                "long-data",
                master_key,
                Some(3600i64), // 1 hour
            )
            .expect("Should create long-lived secret");

        // Wait for short-lived secrets to expire
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Run cleanup
        let deleted_count = repo
            .cleanup_expired_secrets(&conn_mut)
            .expect("Should cleanup expired secrets");

        // Should have deleted 2 expired secrets
        assert_eq!(deleted_count, 2);

        // Verify that permanent and long-lived secrets are still retrievable
        let permanent = repo
            .get_secret(&mut conn_mut, "permanent")
            .expect("Permanent secret should still exist");
        assert_eq!(permanent.key, "permanent");

        let long_lived = repo
            .get_secret(&mut conn_mut, "long-lived")
            .expect("Long-lived secret should still exist");
        assert_eq!(long_lived.key, "long-lived");

        // Verify expired secrets are gone
        let expired1_result = repo.get_secret(&mut conn_mut, "expired1");
        assert!(expired1_result.is_err());

        let expired2_result = repo.get_secret(&mut conn_mut, "expired2");
        assert!(expired2_result.is_err());
    }

    #[test]
    fn test_cleanup_no_expired_secrets() {
        let conn = setup_test_db();
        let repo = SqliteSecretRepo;
        let master_key = create_test_master_key();

        let mut conn_mut = conn;

        // Create only non-expired secrets
        let _permanent = repo
            .create_new_version(&mut conn_mut, "permanent", "data", master_key.clone(), None)
            .expect("Should create permanent secret");

        let _long_lived = repo
            .create_new_version(
                &mut conn_mut,
                "long-lived",
                "data",
                master_key,
                Some(3600i64),
            )
            .expect("Should create long-lived secret");

        // Run cleanup
        let deleted_count = repo
            .cleanup_expired_secrets(&conn_mut)
            .expect("Should cleanup expired secrets");

        // Should have deleted 0 secrets
        assert_eq!(deleted_count, 0);

        // All secrets should still be retrievable
        repo.get_secret(&mut conn_mut, "permanent")
            .expect("Permanent secret should still exist");
        repo.get_secret(&mut conn_mut, "long-lived")
            .expect("Long-lived secret should still exist");
    }
}
