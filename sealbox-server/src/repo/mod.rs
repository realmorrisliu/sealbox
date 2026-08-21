use std::str::FromStr;

use rusqlite::{ToSql, types::FromSql};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    crypto::{
        data_key::DataKey,
        master_key::{PrivateMasterKey, PublicMasterKey},
    },
    error::{Result, SealboxError},
};

pub(crate) use self::sqlite::{
    SqliteHealthRepo, SqliteMasterKeyRepo, SqliteSecretRepo, create_db_connection,
};

mod sqlite;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretInfo {
    pub key: String,             // Secret key identifier
    pub version: i32,            // Latest version number
    pub created_at: i64,         // Creation timestamp (Unix time)
    pub updated_at: i64,         // Last update timestamp (Unix time)
    pub expires_at: Option<i64>, // Expiry timestamp (Unix time), optional for TTL
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub key: String,                 // Secret key identifier
    pub version: i32,                // Version number, incremented on each insert
    pub encrypted_data: Vec<u8>,     // The encrypted secret value
    pub encrypted_data_key: Vec<u8>, // The data key encrypted with user's public key
    pub master_key_id: Uuid,         // References master_keys.id (public key used)
    pub created_at: i64,             // Creation timestamp (Unix time)
    pub updated_at: i64,             // Last update timestamp (Unix time)
    pub expires_at: Option<i64>,     // Expiry timestamp (Unix time), optional for TTL
    pub metadata: Option<String>,    // Optional metadata in serialized format
}

impl Secret {
    /// Creates a new `Secret` instance by encrypting the provided data with a randomly generated data key,
    /// and then encrypting that data key with the provided master key's public key.
    ///
    /// # Arguments
    ///
    /// * `key` - The identifier for the secret.
    /// * `data` - The plaintext data to be encrypted and stored.
    /// * `master_key` - The `MasterKey` used to encrypt the data key.
    ///
    /// # Returns
    ///
    /// Returns a `Result<Self>` containing the new `Secret` on success, or a `SealboxError` on failure.
    ///
    /// # Logic
    ///
    /// 1. Converts the input data to bytes.
    /// 2. Generates a random data key for encrypting the secret data.
    /// 3. Encrypts the secret data using the generated data key.
    /// 4. Encrypts the data key using the provided master key's public key.
    /// 5. Sets the creation and update timestamps to the current time.
    /// 6. Constructs and returns the new `Secret` instance.
    pub(crate) fn new(
        key: &str,
        data: &str,
        master_key: MasterKey,
        version: i32,
        ttl: Option<i64>,
    ) -> Result<Self> {
        let data_bytes = data.as_bytes();

        let data_key = DataKey::new();
        let encrypted_data = data_key.encrypt(data_bytes)?;

        let pub_key = PublicMasterKey::from_str(&master_key.public_key)?;
        let encrypted_data_key = pub_key.encrypt(data_key.as_bytes())?;

        let now_timestamp = time::OffsetDateTime::now_utc().unix_timestamp();

        let expires_at = ttl.map(|ttl| now_timestamp + ttl);

        Ok(Self {
            key: key.to_string(),
            version,
            encrypted_data,
            encrypted_data_key,
            master_key_id: master_key.id,
            created_at: now_timestamp,
            updated_at: now_timestamp,
            expires_at,
            metadata: None,
        })
    }

    pub(crate) fn rekey(
        self,
        old_master_key_id: &Uuid,
        old_private_key: &PrivateMasterKey,
        new_master_key_id: &Uuid,
        new_public_key_pem: &str,
    ) -> Result<Self> {
        let mut secret = self.clone();

        if secret.master_key_id == *new_master_key_id {
            return Ok(secret);
        }

        if secret.master_key_id != *old_master_key_id {
            return Err(SealboxError::MasterKeyMismatch(
                secret.key,
                old_master_key_id.to_string(),
                secret.master_key_id.to_string(),
            ));
        }

        let new_pub_key = PublicMasterKey::from_str(new_public_key_pem)?;

        let data_key = old_private_key.decrypt(&secret.encrypted_data_key)?;
        let new_encrypted_data_key = new_pub_key.encrypt(&data_key)?;

        secret.encrypted_data_key = new_encrypted_data_key;
        secret.master_key_id = *new_master_key_id;
        secret.updated_at = time::OffsetDateTime::now_utc().unix_timestamp();

        Ok(secret)
    }
}

pub(crate) trait SecretRepo: Send + Sync {
    /// Get latest secret with atomic lazy cleanup
    fn get_secret(&self, key: &str) -> Result<Secret>;
    /// Get specific version secret with atomic lazy cleanup
    fn get_secret_by_version(&self, key: &str, version: i32) -> Result<Secret>;
    fn create_new_version(
        &self,
        key: &str,
        data: &str,
        master_key: MasterKey,
        ttl: Option<i64>,
    ) -> Result<Secret>;
    fn delete_secret_by_version(&self, key: &str, version: i32) -> Result<()>;

    /// Rekey every secret under `old_master_key_id`, atomically. Returns the keys of secrets
    /// that could not be rekeyed; if any fail, nothing is committed.
    fn rekey_secrets(
        &self,
        old_master_key_id: &Uuid,
        old_private_key: &PrivateMasterKey,
        new_master_key_id: &Uuid,
        new_public_key_pem: &str,
    ) -> Result<Vec<String>>;
    /// Batch delete all expired secrets and return the count of deleted records.
    fn cleanup_expired_secrets(&self) -> Result<usize>;
    /// List all secrets with basic information (key, latest version, timestamps)
    fn list_secrets(&self) -> Result<Vec<SecretInfo>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MasterKeyStatus {
    Active,
    Retired,
    Disabled,
}
impl ToSql for MasterKeyStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            MasterKeyStatus::Active => Ok(rusqlite::types::ToSqlOutput::from("Active")),
            MasterKeyStatus::Retired => Ok(rusqlite::types::ToSqlOutput::from("Retired")),
            MasterKeyStatus::Disabled => Ok(rusqlite::types::ToSqlOutput::from("Disabled")),
        }
    }
}
impl FromSql for MasterKeyStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str() {
            Ok("Active") => Ok(MasterKeyStatus::Active),
            Ok("Retired") => Ok(MasterKeyStatus::Retired),
            Ok("Disabled") => Ok(MasterKeyStatus::Disabled),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

/// MasterKey struct, represents a row in the master_keys table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterKey {
    pub id: Uuid,                    // Unique identifier (e.g., UUID)
    pub public_key: String,          // Public key (PEM format)
    pub created_at: i64,             // Creation timestamp (Unix time)
    pub status: MasterKeyStatus,     // Status: Active/Retired/Disabled
    pub description: Option<String>, // Optional description
    pub metadata: Option<String>,    // Optional metadata
    /// Whether the server holds this key's private half. Secrets encrypted under a key with
    /// `server_held = false` are *cold*: the server cannot decrypt them under any
    /// circumstances, including rekey (ADR 0001).
    pub server_held: bool,
}

impl MasterKey {
    /// A key registered by a client: the server has only the public half, so secrets under it
    /// are cold.
    pub(crate) fn new(public_key: String) -> Result<Self> {
        Self::with_server_held(public_key, false)
    }

    /// A key whose private half the server holds, making secrets under it broker-serviceable.
    pub(crate) fn server_held(public_key: String) -> Result<Self> {
        Self::with_server_held(public_key, true)
    }

    fn with_server_held(public_key: String, server_held: bool) -> Result<Self> {
        Ok(MasterKey {
            id: Uuid::new_v4(),
            public_key,
            created_at: time::OffsetDateTime::now_utc().unix_timestamp(),
            status: MasterKeyStatus::Active,
            description: None,
            metadata: None,
            server_held,
        })
    }
}

/// MasterKeyRepo trait for managing master_keys table
pub(crate) trait MasterKeyRepo: Send + Sync {
    fn create_master_key(&self, key: &MasterKey) -> Result<()>;
    fn fetch_all_master_keys(&self) -> Result<Vec<MasterKey>>;

    /// Fetch a master key by id, including whether the server holds its private half.
    fn fetch_master_key(&self, master_key_id: &Uuid) -> Result<Option<MasterKey>>;

    /// The current key new secrets are encrypted under. Always server-held.
    fn get_valid_master_key(&self) -> Result<MasterKey>;

    /// Register one of the server's own master keys if not already present, and set its status.
    /// Idempotent, and matched on the public key so a restart does not create a duplicate.
    fn ensure_server_held(
        &self,
        public_key_pem: &str,
        status: MasterKeyStatus,
    ) -> Result<MasterKey>;
}

pub(crate) trait HealthRepo: Send + Sync {
    fn check_health(&self) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::master_key::generate_key_pair;

    #[test]
    fn test_master_key_new() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem.clone()).expect("Should create master key");

        assert_eq!(master_key.public_key, public_pem);
        assert!(matches!(master_key.status, MasterKeyStatus::Active));
        assert!(master_key.description.is_none());
        assert!(master_key.metadata.is_none());
        assert!(master_key.created_at > 0);
    }

    #[test]
    fn test_master_key_status_serialization() {
        // Test ToSql conversion
        let _active_sql = MasterKeyStatus::Active
            .to_sql()
            .expect("Should convert to SQL");
        let _retired_sql = MasterKeyStatus::Retired
            .to_sql()
            .expect("Should convert to SQL");
        let _disabled_sql = MasterKeyStatus::Disabled
            .to_sql()
            .expect("Should convert to SQL");

        // Just test that conversion works without errors
        // Test placeholder - functionality verified by other tests
    }

    #[test]
    fn test_secret_new() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let secret_key = "test-secret";
        let secret_data = "This is secret data";
        let version = 1;
        let ttl = Some(3600); // 1 hour

        let secret = Secret::new(secret_key, secret_data, master_key.clone(), version, ttl)
            .expect("Should create secret");

        assert_eq!(secret.key, secret_key);
        assert_eq!(secret.version, version);
        assert_eq!(secret.master_key_id, master_key.id);
        assert!(secret.expires_at.is_some());
        assert!(secret.created_at > 0);
        assert_eq!(secret.created_at, secret.updated_at);
        assert!(!secret.encrypted_data.is_empty());
        assert!(!secret.encrypted_data_key.is_empty());
        assert!(secret.metadata.is_none());
    }

    #[test]
    fn test_secret_new_without_ttl() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let secret = Secret::new("test-key", "test-data", master_key, 1, None)
            .expect("Should create secret");

        assert!(secret.expires_at.is_none());
    }

    #[test]
    fn test_secret_encryption_is_different() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let secret_data = "Same secret data";

        let secret1 = Secret::new("key1", secret_data, master_key.clone(), 1, None)
            .expect("Should create first secret");
        let secret2 = Secret::new("key2", secret_data, master_key, 2, None)
            .expect("Should create second secret");

        // Even with same data, encrypted results should be different due to random data keys
        assert_ne!(secret1.encrypted_data, secret2.encrypted_data);
        assert_ne!(secret1.encrypted_data_key, secret2.encrypted_data_key);
    }

    #[test]
    fn test_secret_rekey() {
        let (old_private_pem, old_public_pem) =
            generate_key_pair().expect("Should generate old key pair");
        let (_, new_public_pem) = generate_key_pair().expect("Should generate new key pair");

        let old_master_key = MasterKey::new(old_public_pem).expect("Should create old master key");
        let new_master_key = MasterKey::new(new_public_pem).expect("Should create new master key");

        let original_secret =
            Secret::new("test-key", "secret-data", old_master_key.clone(), 1, None)
                .expect("Should create secret");

        let original_created_at = original_secret.created_at;
        let original_encrypted_data = original_secret.encrypted_data.clone();
        let original_encrypted_data_key = original_secret.encrypted_data_key.clone();

        let rekeyed_secret = original_secret
            .rekey(
                &old_master_key.id,
                &PrivateMasterKey::from_str(&old_private_pem).expect("Should parse"),
                &new_master_key.id,
                &new_master_key.public_key,
            )
            .expect("Should rekey");

        // Rekeying should update master key ID and encrypted data key
        assert_eq!(rekeyed_secret.master_key_id, new_master_key.id);
        assert_ne!(
            rekeyed_secret.encrypted_data_key,
            original_encrypted_data_key
        );
        assert_eq!(rekeyed_secret.encrypted_data, original_encrypted_data); // Data itself unchanged
        assert!(rekeyed_secret.updated_at >= original_created_at);
    }

    #[test]
    fn test_secret_rekey_same_key() {
        let (private_pem, public_pem) = generate_key_pair().expect("Should generate key pair");
        let private_key =
            PrivateMasterKey::from_str(&private_pem).expect("Should parse private key");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let original_secret = Secret::new("test-key", "secret-data", master_key.clone(), 1, None)
            .expect("Should create secret");

        // Rekeying to the same key should return the secret unchanged
        let rekeyed_secret = original_secret
            .clone()
            .rekey(
                &master_key.id,
                &private_key,
                &master_key.id,
                &master_key.public_key,
            )
            .expect("Should handle same rekeying");

        assert_eq!(rekeyed_secret.master_key_id, original_secret.master_key_id);
        assert_eq!(
            rekeyed_secret.encrypted_data_key,
            original_secret.encrypted_data_key
        );
    }

    #[test]
    fn test_secret_rekey_wrong_old_key() {
        let (old_private_pem, old_public_pem) =
            generate_key_pair().expect("Should generate old key pair");
        let (_, new_public_pem) = generate_key_pair().expect("Should generate new key pair");
        let (_, wrong_public_pem) = generate_key_pair().expect("Should generate wrong key pair");

        let old_master_key = MasterKey::new(old_public_pem).expect("Should create old master key");
        let new_master_key = MasterKey::new(new_public_pem).expect("Should create new master key");
        let wrong_master_key =
            MasterKey::new(wrong_public_pem).expect("Should create wrong master key");

        let original_secret = Secret::new("test-key", "secret-data", old_master_key, 1, None)
            .expect("Should create secret");

        // Trying to rekey with wrong old key ID should fail
        let result = original_secret.rekey(
            &wrong_master_key.id, // Wrong old key ID
            &PrivateMasterKey::from_str(&old_private_pem).expect("Should parse"),
            &new_master_key.id,
            &new_master_key.public_key,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            SealboxError::MasterKeyMismatch(_, _, _) => {} // Expected
            _ => panic!("Expected MasterKeyMismatch error"),
        }
    }

    #[test]
    fn test_secret_rekey_with_a_private_key_that_does_not_match() {
        // A malformed private key is no longer expressible: `rekey` takes a parsed
        // `PrivateMasterKey`, so the parse either succeeded before this point or there is
        // nothing to call. What remains possible is a well-formed key that is simply the
        // wrong one.
        let (_, old_public_pem) = generate_key_pair().expect("Should generate old key pair");
        let (_, new_public_pem) = generate_key_pair().expect("Should generate new key pair");
        let (unrelated_pem, _) = generate_key_pair().expect("Should generate unrelated pair");
        let unrelated_key =
            PrivateMasterKey::from_str(&unrelated_pem).expect("Should parse private key");

        let old_master_key = MasterKey::new(old_public_pem).expect("Should create old master key");
        let new_master_key = MasterKey::new(new_public_pem).expect("Should create new master key");

        let original_secret =
            Secret::new("test-key", "secret-data", old_master_key.clone(), 1, None)
                .expect("Should create secret");

        let result = original_secret.rekey(
            &old_master_key.id,
            &unrelated_key,
            &new_master_key.id,
            &new_master_key.public_key,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_ttl_calculation() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let ttl_seconds = 7200i64; // 2 hours
        let secret = Secret::new("test-key", "test-data", master_key, 1, Some(ttl_seconds))
            .expect("Should create secret");

        let expected_expiry = secret.created_at + ttl_seconds;
        assert_eq!(secret.expires_at, Some(expected_expiry));
    }
}
