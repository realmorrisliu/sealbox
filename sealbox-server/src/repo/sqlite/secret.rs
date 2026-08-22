use serde_rusqlite::*;
use std::sync::{Arc, Mutex};
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
    pub(crate) fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }
}

impl SqliteSecretRepo {
    pub fn init_table(conn: &rusqlite::Connection) -> Result<()> {
        // Initialize database table structure
        conn.execute(
            "CREATE TABLE IF NOT EXISTS secrets (
                key TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                encrypted_data BLOB NOT NULL,
                encrypted_data_key BLOB NOT NULL,
                master_key_id BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                expires_at INTEGER,
                metadata TEXT,
                pending INTEGER NOT NULL DEFAULT 0,
                rotate_after INTEGER,
                PRIMARY KEY (key, version)
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
    fn get_secret(&self, key: &str) -> Result<Secret> {
        let mut guard = self.conn.lock()?;
        let conn = &mut *guard;
        info!("get_secret: key={}", key);

        self.get_secret_with_query(
            conn,
            "SELECT
                key,
                version,
                encrypted_data,
                encrypted_data_key,
                master_key_id,
                created_at,
                updated_at,
                expires_at,
                metadata,
                rotate_after
            FROM secrets
            WHERE key = ?1 AND pending = 0
            ORDER BY version DESC
            LIMIT 1",
            [key],
            key,
        )
    }

    fn get_secret_by_version(&self, key: &str, version: i32) -> Result<Secret> {
        let mut guard = self.conn.lock()?;
        let conn = &mut *guard;
        info!("get_secret_by_version: key={}, version={}", key, version);

        self.get_secret_with_query(
            conn,
            "SELECT
                key,
                version,
                encrypted_data,
                encrypted_data_key,
                master_key_id,
                created_at,
                updated_at,
                expires_at,
                metadata,
                rotate_after
            FROM secrets
            WHERE key = ?1 AND version = ?2 AND pending = 0
            LIMIT 1",
            (key, version),
            key,
        )
    }

    fn create_new_version(
        &self,
        key: &str,
        value: &crate::repo::SecretValue,
        master_key: crate::repo::MasterKey,
        ttl: Option<i64>,
        rotate_after: Option<i64>,
        pending: bool,
    ) -> Result<Secret> {
        let mut guard = self.conn.lock()?;
        let conn = &mut *guard;
        info!("create_new_version");

        let tx = conn.transaction()?;

        let next_version = {
            let mut stmt =
                tx.prepare("SELECT COALESCE(MAX(version), 0) FROM secrets WHERE key = ?1")?;
            let latest_version: i32 = stmt.query_one([key], |row| row.get(0))?;
            latest_version + 1
        };

        // The plaintext lives only from here to the envelope inside `Secret::new`.
        let plaintext = value.resolve()?;
        let mut secret = Secret::new(key, &plaintext, master_key, next_version, ttl)?;
        secret.rotate_after = rotate_after;

        tx.execute(
            "INSERT INTO secrets (
              key,
              version,
              encrypted_data,
              encrypted_data_key,
              master_key_id,
              created_at,
              updated_at,
              expires_at,
              metadata,
              pending,
              rotate_after
          ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            (
                &secret.key,
                &secret.version,
                &secret.encrypted_data,
                &secret.encrypted_data_key,
                &secret.master_key_id,
                &secret.created_at,
                &secret.updated_at,
                &secret.expires_at,
                &secret.metadata,
                pending,
                &secret.rotate_after,
            ),
        )?;

        tx.commit()?;

        Ok(secret)
    }

    fn delete_secret_by_version(&self, key: &str, version: i32) -> Result<()> {
        let guard = self.conn.lock()?;
        let conn = &*guard;
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

    fn rekey_secrets(
        &self,
        old_master_key_id: &Uuid,
        old_private_key: &crate::crypto::master_key::PrivateMasterKey,
        new_master_key_id: &Uuid,
        new_public_key_pem: &str,
    ) -> Result<Vec<String>> {
        let mut guard = self.conn.lock()?;
        let conn = &mut *guard;

        let secrets: Vec<Secret> = {
            let mut stmt = conn.prepare(
                "SELECT
                        key,
                    version,
                    encrypted_data,
                    encrypted_data_key,
                    master_key_id,
                    created_at,
                    updated_at,
                    expires_at,
                    metadata,
                    rotate_after
                FROM secrets
                WHERE master_key_id = ?1",
            )?;
            let rows = stmt.query([old_master_key_id])?;
            from_rows::<Secret>(rows)
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| SealboxError::DatabaseError(e.to_string()))?
        };

        let mut failed_secret_keys = Vec::new();
        let tx = conn.transaction()?;

        for secret in secrets {
            let secret_key = secret.key.clone();
            match secret.rekey(
                old_master_key_id,
                old_private_key,
                new_master_key_id,
                new_public_key_pem,
            ) {
                Ok(rekeyed) => {
                    tx.execute(
                        // `updated_at` is deliberately left alone. A rekey re-encrypts the data
                        // key and does not change the value (CONTEXT.md draws exactly this
                        // line), so touching it would make every rekeyed secret look freshly
                        // rotated — and a rotation interval is measured from that timestamp.
                        "UPDATE secrets SET
                            encrypted_data_key = ?1,
                            master_key_id = ?2
                         WHERE key = ?3 AND version = ?4",
                        rusqlite::params![
                            &rekeyed.encrypted_data_key,
                            &rekeyed.master_key_id,
                            &rekeyed.key,
                            &rekeyed.version,
                        ],
                    )?;
                }
                Err(err) => {
                    failed_secret_keys.push(secret_key.clone());
                    info!("Failed to rekey secret {}: {}", secret_key, err);
                }
            }
        }

        if !failed_secret_keys.is_empty() {
            // Dropping the transaction rolls it back. A rekey that half-succeeded would leave
            // secrets split across two master keys, with no record of which is which — worse
            // than not having run at all.
            info!(
                "Rekey aborted: {} secret(s) could not be rekeyed, nothing committed",
                failed_secret_keys.len()
            );
            return Ok(failed_secret_keys);
        }

        tx.commit()?;
        Ok(failed_secret_keys)
    }

    fn get_pending(&self, key: &str, version: i32) -> Result<Secret> {
        let guard = self.conn.lock()?;
        let mut stmt = guard.prepare(
            "SELECT key, version, encrypted_data, encrypted_data_key, master_key_id,
                    created_at, updated_at, expires_at, metadata, rotate_after
             FROM secrets WHERE key = ?1 AND version = ?2 AND pending = 1",
        )?;
        let mut rows = stmt.query((key, version))?;
        let row = rows
            .next()?
            .ok_or_else(|| SealboxError::SecretNotFound(format!("{key} (pending {version})")))?;
        from_row::<Secret>(row).map_err(|e| SealboxError::DatabaseError(e.to_string()))
    }

    /// A pending version becomes the current one. Used when a rotation's grant succeeds.
    fn commit_pending(&self, key: &str, version: i32) -> Result<()> {
        let guard = self.conn.lock()?;
        let updated = guard.execute(
            "UPDATE secrets SET pending = 0, updated_at = ?1 WHERE key = ?2 AND version = ?3 AND pending = 1",
            (
                time::OffsetDateTime::now_utc().unix_timestamp(),
                key,
                version,
            ),
        )?;
        if updated == 0 {
            return Err(SealboxError::InvalidRequest(format!(
                "no pending version {version} of `{key}` to commit"
            )));
        }
        Ok(())
    }

    /// Remove a pending version. A failed rotation must leave the previous value current and
    /// unchanged — a stored credential that silently disagrees with reality is worse than none.
    fn discard_pending(&self, key: &str, version: i32) -> Result<()> {
        let guard = self.conn.lock()?;
        guard.execute(
            "DELETE FROM secrets WHERE key = ?1 AND version = ?2 AND pending = 1",
            (key, version),
        )?;
        Ok(())
    }

    /// Replace a pending version's value, for a rotation that captures what the grant produced.
    fn replace_pending_value(
        &self,
        key: &str,
        version: i32,
        value: &str,
        master_key: crate::repo::MasterKey,
    ) -> Result<()> {
        let replacement = Secret::new(key, value, master_key, version, None)?;
        let guard = self.conn.lock()?;
        let updated = guard.execute(
            "UPDATE secrets SET encrypted_data = ?1, encrypted_data_key = ?2, master_key_id = ?3
             WHERE key = ?4 AND version = ?5 AND pending = 1",
            (
                &replacement.encrypted_data,
                &replacement.encrypted_data_key,
                &replacement.master_key_id,
                key,
                version,
            ),
        )?;
        if updated == 0 {
            return Err(SealboxError::InvalidRequest(format!(
                "no pending version {version} of `{key}` to write"
            )));
        }
        Ok(())
    }

    fn cleanup_expired_secrets(&self) -> Result<usize> {
        let guard = self.conn.lock()?;
        let conn = &*guard;
        info!("cleanup_expired_secrets");
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let deleted_count = conn.execute(
            "DELETE FROM secrets WHERE expires_at IS NOT NULL AND expires_at < ?1",
            [now],
        )?;
        info!("Cleaned up {} expired secrets", deleted_count);
        Ok(deleted_count)
    }

    fn count_secrets(&self) -> Result<usize> {
        let guard = self.conn.lock()?;
        let count: i64 = guard.query_row("SELECT COUNT(*) FROM secrets", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    fn list_secrets(&self) -> Result<Vec<crate::repo::SecretInfo>> {
        let guard = self.conn.lock()?;
        let conn = &*guard;
        info!("list_secrets");
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        // Joined against the latest version per key rather than selecting bare columns beside
        // an aggregate: with more than one MAX in the list, SQLite picks the row matching the
        // last one, so `rotate_after` and `expires_at` could come from a different version than
        // the one being reported.
        let mut stmt = conn.prepare(
            "SELECT
                s.key,
                s.version,
                s.created_at,
                s.updated_at,
                s.expires_at,
                s.rotate_after
            FROM secrets s
            JOIN (
                SELECT key, MAX(version) AS version
                FROM secrets WHERE pending = 0 GROUP BY key
            ) latest ON latest.key = s.key AND latest.version = s.version
            WHERE s.pending = 0 AND (s.expires_at IS NULL OR s.expires_at > ?1)
            ORDER BY s.updated_at DESC",
        )?;

        let secret_infos = stmt
            .query_map([now], |row| {
                let updated_at: i64 = row.get(3)?;
                let rotate_after: Option<i64> = row.get(5)?;
                Ok(crate::repo::SecretInfo {
                    key: row.get(0)?,
                    version: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at,
                    expires_at: row.get(4)?,
                    rotate_after,
                    // Computed at read time rather than stored: a stored due-date is a second
                    // copy of a fact, and it would be wrong the moment a rotation moved the
                    // timestamp it came from.
                    rotate_due_at: rotate_after.map(|after| updated_at + after),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| SealboxError::DatabaseError(e.to_string()))?;

        Ok(secret_infos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::master_key::generate_key_pair;
    use crate::repo::MasterKey;
    use std::str::FromStr;

    fn setup_test_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().expect("Should create in-memory DB");
        SqliteSecretRepo::init_table(&conn).expect("Should initialize tables");
        conn
    }

    /// Tests only ever supply a value; generation has its own tests.
    fn supplied(value: &str) -> crate::repo::SecretValue {
        crate::repo::SecretValue::Supplied(value.to_string())
    }

    fn setup_test_repo() -> SqliteSecretRepo {
        SqliteSecretRepo::new(Arc::new(Mutex::new(setup_test_db())))
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
        let repo = setup_test_repo();
        let master_key = create_test_master_key();

        let secret_key = "test-secret";
        let secret_data = "This is secret data";

        // Create secret
        let created_secret = repo
            .create_new_version(
                secret_key,
                &supplied(secret_data),
                master_key,
                None,
                None,
                false,
            )
            .expect("Should create secret");

        // Get secret back
        let retrieved_secret = repo.get_secret(secret_key).expect("Should retrieve secret");

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
        let repo = setup_test_repo();

        let result = repo.get_secret("nonexistent-key");
        assert!(result.is_err());

        match result.unwrap_err() {
            SealboxError::SecretNotFound(key) => assert_eq!(key, "nonexistent-key"),
            _ => panic!("Expected SecretNotFound error"),
        }
    }

    #[test]
    fn test_create_multiple_versions() {
        let repo = setup_test_repo();
        let master_key = create_test_master_key();

        let secret_key = "test-secret";

        // Create first version
        let secret_v1 = repo
            .create_new_version(
                secret_key,
                &supplied("data version 1"),
                master_key.clone(),
                None,
                None,
                false,
            )
            .expect("Should create version 1");

        // Create second version
        let secret_v2 = repo
            .create_new_version(
                secret_key,
                &supplied("data version 2"),
                master_key,
                None,
                None,
                false,
            )
            .expect("Should create version 2");

        assert_eq!(secret_v1.version, 1);
        assert_eq!(secret_v2.version, 2);
        assert_ne!(secret_v1.encrypted_data, secret_v2.encrypted_data);

        // Get latest version (should be v2)
        let latest = repo
            .get_secret(secret_key)
            .expect("Should get latest version");
        assert_eq!(latest.version, 2);
        assert_eq!(latest.encrypted_data, secret_v2.encrypted_data);
    }

    #[test]
    fn test_get_secret_by_version() {
        let repo = setup_test_repo();
        let master_key = create_test_master_key();

        let secret_key = "test-secret";

        // Create multiple versions
        let secret_v1 = repo
            .create_new_version(
                secret_key,
                &supplied("data version 1"),
                master_key.clone(),
                None,
                None,
                false,
            )
            .expect("Should create version 1");

        let _secret_v2 = repo
            .create_new_version(
                secret_key,
                &supplied("data version 2"),
                master_key,
                None,
                None,
                false,
            )
            .expect("Should create version 2");

        // Get specific version
        let retrieved_v1 = repo
            .get_secret_by_version(secret_key, 1)
            .expect("Should get version 1");

        assert_eq!(retrieved_v1.version, 1);
        assert_eq!(retrieved_v1.encrypted_data, secret_v1.encrypted_data);
    }

    #[test]
    fn test_get_secret_by_version_not_found() {
        let repo = setup_test_repo();

        let result = repo.get_secret_by_version("nonexistent-key", 1);
        assert!(result.is_err());

        match result.unwrap_err() {
            SealboxError::SecretNotFound(key) => assert_eq!(key, "nonexistent-key"),
            _ => panic!("Expected SecretNotFound error"),
        }
    }

    #[test]
    fn test_delete_secret_by_version() {
        let repo = setup_test_repo();
        let master_key = create_test_master_key();

        let secret_key = "test-secret";

        // Create multiple versions
        let _secret_v1 = repo
            .create_new_version(
                secret_key,
                &supplied("data version 1"),
                master_key.clone(),
                None,
                None,
                false,
            )
            .expect("Should create version 1");

        let secret_v2 = repo
            .create_new_version(
                secret_key,
                &supplied("data version 2"),
                master_key,
                None,
                None,
                false,
            )
            .expect("Should create version 2");

        // Delete version 1
        repo.delete_secret_by_version(secret_key, 1)
            .expect("Should delete version 1");

        // Version 1 should be gone
        let result = repo.get_secret_by_version(secret_key, 1);
        assert!(result.is_err());

        // Version 2 should still exist and be the latest
        let latest = repo
            .get_secret(secret_key)
            .expect("Should get latest version");
        assert_eq!(latest.version, 2);
        assert_eq!(latest.encrypted_data, secret_v2.encrypted_data);
    }

    #[test]
    fn test_delete_secret_by_version_not_found() {
        let repo = setup_test_repo();

        let result = repo.delete_secret_by_version("nonexistent-key", 1);
        assert!(result.is_err());

        match result.unwrap_err() {
            SealboxError::SecretNotFound(key) => assert_eq!(key, "nonexistent-key"),
            _ => panic!("Expected SecretNotFound error"),
        }
    }

    #[test]
    fn test_secret_with_ttl() {
        let repo = setup_test_repo();
        let master_key = create_test_master_key();

        let ttl = Some(3600i64); // 1 hour

        // Create secret with TTL
        let secret = repo
            .create_new_version(
                "ttl-secret",
                &supplied("temporary-data"),
                master_key,
                ttl,
                None,
                false,
            )
            .expect("Should create secret with TTL");

        assert!(secret.expires_at.is_some());
        let expected_expiry = secret.created_at + 3600;
        assert_eq!(secret.expires_at, Some(expected_expiry));

        // Retrieve and verify TTL is preserved
        let retrieved = repo
            .get_secret("ttl-secret")
            .expect("Should retrieve secret");
        assert_eq!(retrieved.expires_at, secret.expires_at);
    }

    #[test]
    fn test_expired_secret_not_retrievable() {
        let repo = setup_test_repo();
        let master_key = create_test_master_key();

        // Create a secret that expires immediately (TTL = 1 second)
        let _secret = repo
            .create_new_version(
                "expired-secret",
                &supplied("temporary-data"),
                master_key,
                Some(1i64), // 1 second
                None,
                false,
            )
            .expect("Should create secret with short TTL");

        // Wait for the secret to expire
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Try to retrieve the expired secret
        let result = repo.get_secret("expired-secret");
        assert!(result.is_err());

        match result.unwrap_err() {
            SealboxError::SecretNotFound(key) => assert_eq!(key, "expired-secret"),
            _ => panic!("Expected SecretNotFound error"),
        }
    }

    #[test]
    fn test_expired_secret_by_version_not_retrievable() {
        let repo = setup_test_repo();
        let master_key = create_test_master_key();

        // Create a secret that expires immediately
        let secret = repo
            .create_new_version(
                "expired-secret-v",
                &supplied("temporary-data"),
                master_key,
                Some(1i64), // 1 second
                None,
                false,
            )
            .expect("Should create secret with short TTL");

        // Wait for the secret to expire
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Try to retrieve the expired secret by version
        let result = repo.get_secret_by_version("expired-secret-v", secret.version);
        assert!(result.is_err());

        match result.unwrap_err() {
            SealboxError::SecretNotFound(key) => assert_eq!(key, "expired-secret-v"),
            _ => panic!("Expected SecretNotFound error"),
        }
    }

    #[test]
    fn test_cleanup_expired_secrets() {
        let repo = setup_test_repo();
        let master_key = create_test_master_key();

        // Create several secrets: some expired, some not
        let _expired1 = repo
            .create_new_version(
                "expired1",
                &supplied("data1"),
                master_key.clone(),
                Some(1i64), // 1 second
                None,
                false,
            )
            .expect("Should create expired secret 1");

        let _expired2 = repo
            .create_new_version(
                "expired2",
                &supplied("data2"),
                master_key.clone(),
                Some(1i64), // 1 second
                None,
                false,
            )
            .expect("Should create expired secret 2");

        let _permanent = repo
            .create_new_version(
                "permanent",
                &supplied("permanent-data"),
                master_key.clone(),
                None, // No TTL
                None,
                false,
            )
            .expect("Should create permanent secret");

        let _long_lived = repo
            .create_new_version(
                "long-lived",
                &supplied("long-data"),
                master_key,
                Some(3600i64), // 1 hour
                None,
                false,
            )
            .expect("Should create long-lived secret");

        // Wait for short-lived secrets to expire
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Run cleanup
        let deleted_count = repo
            .cleanup_expired_secrets()
            .expect("Should cleanup expired secrets");

        // Should have deleted 2 expired secrets
        assert_eq!(deleted_count, 2);

        // Verify that permanent and long-lived secrets are still retrievable
        let permanent = repo
            .get_secret("permanent")
            .expect("Permanent secret should still exist");
        assert_eq!(permanent.key, "permanent");

        let long_lived = repo
            .get_secret("long-lived")
            .expect("Long-lived secret should still exist");
        assert_eq!(long_lived.key, "long-lived");

        // Verify expired secrets are gone
        let expired1_result = repo.get_secret("expired1");
        assert!(expired1_result.is_err());

        let expired2_result = repo.get_secret("expired2");
        assert!(expired2_result.is_err());
    }

    #[test]
    fn test_cleanup_no_expired_secrets() {
        let repo = setup_test_repo();
        let master_key = create_test_master_key();

        // Create only non-expired secrets
        let _permanent = repo
            .create_new_version(
                "permanent",
                &supplied("data"),
                master_key.clone(),
                None,
                None,
                false,
            )
            .expect("Should create permanent secret");

        let _long_lived = repo
            .create_new_version(
                "long-lived",
                &supplied("data"),
                master_key,
                Some(3600i64),
                None,
                false,
            )
            .expect("Should create long-lived secret");

        // Run cleanup
        let deleted_count = repo
            .cleanup_expired_secrets()
            .expect("Should cleanup expired secrets");

        // Should have deleted 0 secrets
        assert_eq!(deleted_count, 0);

        // All secrets should still be retrievable
        repo.get_secret("permanent")
            .expect("Permanent secret should still exist");
        repo.get_secret("long-lived")
            .expect("Long-lived secret should still exist");
    }

    #[test]
    fn test_list_secrets() {
        let repo = setup_test_repo();
        let master_key = create_test_master_key();

        // Create several secrets
        let _secret1 = repo
            .create_new_version(
                "secret1",
                &supplied("data1"),
                master_key.clone(),
                None,
                None,
                false,
            )
            .expect("Should create secret1");

        let _secret2 = repo
            .create_new_version(
                "secret2",
                &supplied("data2"),
                master_key.clone(),
                Some(3600),
                None,
                false,
            )
            .expect("Should create secret2 with TTL");

        let _secret3 = repo
            .create_new_version(
                "secret3",
                &supplied("data3"),
                master_key.clone(),
                None,
                None,
                false,
            )
            .expect("Should create secret3");

        // Create multiple versions of secret1
        let _secret1_v2 = repo
            .create_new_version(
                "secret1",
                &supplied("data1-v2"),
                master_key,
                None,
                None,
                false,
            )
            .expect("Should create secret1 version 2");

        // List all secrets
        let secret_list = repo.list_secrets().expect("Should list secrets");

        // Should return 3 unique secrets (secret1, secret2, secret3)
        assert_eq!(secret_list.len(), 3);

        // Find secret1 - should have version 2 (latest)
        let secret1_info = secret_list
            .iter()
            .find(|s| s.key == "secret1")
            .expect("Should find secret1");
        assert_eq!(secret1_info.version, 2);

        // Find secret2 - should have TTL set
        let secret2_info = secret_list
            .iter()
            .find(|s| s.key == "secret2")
            .expect("Should find secret2");
        assert!(secret2_info.expires_at.is_some());

        // All secrets should have valid timestamps
        for secret_info in &secret_list {
            assert!(secret_info.created_at > 0);
            assert!(secret_info.updated_at > 0);
            assert!(secret_info.updated_at >= secret_info.created_at);
        }
    }

    #[test]
    fn test_list_secrets_excludes_expired() {
        let repo = setup_test_repo();
        let master_key = create_test_master_key();

        // Create a secret that expires immediately
        let _expired_secret = repo
            .create_new_version(
                "expired-secret",
                &supplied("temporary-data"),
                master_key.clone(),
                Some(1i64), // 1 second
                None,
                false,
            )
            .expect("Should create expired secret");

        // Create a permanent secret
        let _permanent_secret = repo
            .create_new_version(
                "permanent-secret",
                &supplied("permanent-data"),
                master_key,
                None,
                None,
                false,
            )
            .expect("Should create permanent secret");

        // Wait for the secret to expire
        std::thread::sleep(std::time::Duration::from_secs(2));

        // List secrets should only return the permanent one
        let secret_list = repo.list_secrets().expect("Should list secrets");

        assert_eq!(secret_list.len(), 1);
        assert_eq!(secret_list[0].key, "permanent-secret");
    }
    #[test]
    fn test_rekey_secrets_moves_every_secret_to_the_new_key() {
        let repo = setup_test_repo();

        let (old_private_pem, old_public) =
            generate_key_pair().expect("Should generate old key pair");
        let old_private = crate::crypto::master_key::PrivateMasterKey::from_str(&old_private_pem)
            .expect("Should parse the old private key");
        let old_key = MasterKey::new(old_public).expect("Should create old master key");
        let (_, new_public) = generate_key_pair().expect("Should generate new key pair");
        let new_key = MasterKey::new(new_public).expect("Should create new master key");

        let s1 = repo
            .create_new_version(
                "s1",
                &supplied("d1"),
                old_key.clone(),
                None,
                Some(3600),
                false,
            )
            .expect("Should create s1");
        repo.create_new_version("s2", &supplied("d2"), old_key.clone(), None, None, false)
            .expect("Should create s2");

        let failed = repo
            .rekey_secrets(&old_key.id, &old_private, &new_key.id, &new_key.public_key)
            .expect("Should rekey");

        assert!(failed.is_empty(), "no secret should fail to rekey");

        // A rekey re-encrypts the data key and does not change the value (CONTEXT.md draws
        // exactly that line), so it must not touch `updated_at` — a rotation interval is measured
        // from that timestamp, and bumping it here would quietly settle every secret in the store
        // that was due.
        let after = repo.get_secret("s1").expect("Should read s1 back");
        assert_eq!(after.updated_at, s1.updated_at, "a rekey is not a rotation");
        assert_eq!(after.rotate_after, Some(3600), "and it keeps the policy");
        for key in ["s1", "s2"] {
            let secret = repo.get_secret(key).expect("Should read secret");
            assert_eq!(secret.master_key_id, new_key.id);
        }
    }

    #[test]
    fn test_rekey_secrets_with_wrong_private_key_changes_nothing() {
        let repo = setup_test_repo();

        let (_, old_public) = generate_key_pair().expect("Should generate old key pair");
        let old_key = MasterKey::new(old_public).expect("Should create old master key");
        let (unrelated_private_pem, _) =
            generate_key_pair().expect("Should generate unrelated pair");
        let unrelated_private =
            crate::crypto::master_key::PrivateMasterKey::from_str(&unrelated_private_pem)
                .expect("Should parse the unrelated private key");
        let (_, new_public) = generate_key_pair().expect("Should generate new key pair");
        let new_key = MasterKey::new(new_public).expect("Should create new master key");

        repo.create_new_version("s1", &supplied("d1"), old_key.clone(), None, None, false)
            .expect("Should create s1");
        repo.create_new_version("s2", &supplied("d2"), old_key.clone(), None, None, false)
            .expect("Should create s2");

        let failed = repo
            .rekey_secrets(
                &old_key.id,
                &unrelated_private,
                &new_key.id,
                &new_key.public_key,
            )
            .expect("Should report failures rather than error");

        assert_eq!(failed.len(), 2, "both secrets fail with an unrelated key");

        // All or nothing: not one secret may have moved.
        for key in ["s1", "s2"] {
            let secret = repo.get_secret(key).expect("Should read secret");
            assert_eq!(
                secret.master_key_id, old_key.id,
                "a failed rekey must leave every secret on its original master key"
            );
        }
    }
}
