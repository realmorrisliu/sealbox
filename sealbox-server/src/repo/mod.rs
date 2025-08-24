use std::str::FromStr;

use rusqlite::{ToSql, types::FromSql};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    crypto::{
        client_key::{PrivateClientKey, PublicClientKey},
        data_key::DataKey,
    },
    error::{Result, SealboxError},
};

pub(crate) use self::sqlite::{
    SqliteClientKeyRepo, SqliteHealthRepo, SqliteSecretRepo, SqliteSecretClientKeyRepo, create_db_connection,
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
    pub client_key_id: Uuid,         // References client_keys.id (public key used)
    pub created_at: i64,             // Creation timestamp (Unix time)
    pub updated_at: i64,             // Last update timestamp (Unix time)
    pub expires_at: Option<i64>,     // Expiry timestamp (Unix time), optional for TTL
}

impl Secret {
    /// Creates a new `Secret` instance by encrypting the provided data with a randomly generated data key,
    /// and then encrypting that data key with the provided client key's public key.
    ///
    /// # Arguments
    ///
    /// * `key` - The identifier for the secret.
    /// * `data` - The plaintext data to be encrypted and stored.
    /// * `client_key` - The `ClientKey` used to encrypt the data key.
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
    /// 4. Encrypts the data key using the provided client key's public key.
    /// 5. Sets the creation and update timestamps to the current time.
    /// 6. Constructs and returns the new `Secret` instance.
    pub(crate) fn new(
        key: &str,
        data: &str,
        client_key: ClientKey,
        version: i32,
        ttl: Option<i64>,
    ) -> Result<Self> {
        let data_bytes = data.as_bytes();

        let data_key = DataKey::new();
        let encrypted_data = data_key.encrypt(data_bytes)?;

        let pub_key = PublicClientKey::from_str(&client_key.public_key)?;
        let encrypted_data_key = pub_key.encrypt(data_key.as_bytes())?;

        let now_timestamp = time::OffsetDateTime::now_utc().unix_timestamp();

        let expires_at = ttl.map(|ttl| now_timestamp + ttl);

        Ok(Self {
            key: key.to_string(),
            version,
            encrypted_data,
            encrypted_data_key,
            client_key_id: client_key.id,
            created_at: now_timestamp,
            updated_at: now_timestamp,
            expires_at,
        })
    }

    pub(crate) fn rotate_client_key(
        self,
        old_client_key_id: &Uuid,
        old_private_key_pem: &str,
        new_client_key_id: &Uuid,
        new_public_key_pem: &str,
    ) -> Result<Self> {
        let mut secret = self.clone();

        if secret.client_key_id == *new_client_key_id {
            return Ok(secret);
        }

        if secret.client_key_id != *old_client_key_id {
            return Err(SealboxError::ClientKeyMismatch(
                secret.key,
                old_client_key_id.to_string(),
                secret.client_key_id.to_string(),
            ));
        }

        let old_priv_key = PrivateClientKey::from_str(old_private_key_pem)?;
        let new_pub_key = PublicClientKey::from_str(new_public_key_pem)?;

        let data_key = old_priv_key.decrypt(&secret.encrypted_data_key)?;
        let new_encrypted_data_key = new_pub_key.encrypt(&data_key)?;

        secret.encrypted_data_key = new_encrypted_data_key;
        secret.client_key_id = *new_client_key_id;
        secret.updated_at = time::OffsetDateTime::now_utc().unix_timestamp();

        Ok(secret)
    }
}

pub(crate) trait SecretRepo: Send + Sync {
    /// Get latest secret with atomic lazy cleanup
    fn get_secret(&self, conn: &mut rusqlite::Connection, key: &str) -> Result<Secret>;
    /// Get specific version secret with atomic lazy cleanup
    fn get_secret_by_version(
        &self,
        conn: &mut rusqlite::Connection,
        key: &str,
        version: i32,
    ) -> Result<Secret>;
    fn create_new_version(
        &self,
        conn: &mut rusqlite::Connection,
        key: &str,
        data: &str,
        client_key: ClientKey,
        ttl: Option<i64>,
    ) -> Result<Secret>;

    /// Create new version of secret with multiple client keys
    fn create_new_version_multi_client(
        &self,
        conn: &mut rusqlite::Connection,
        key: &str,
        data: &str,
        client_key_ids: &[Uuid],
        ttl: Option<i64>,
    ) -> Result<Secret>;
    fn delete_secret_by_version(
        &self,
        conn: &rusqlite::Connection,
        key: &str,
        version: i32,
    ) -> Result<()>;

    /// Fetch all secrets using the given client_key_id.
    fn fetch_secrets_by_client_key(
        &self,
        conn: &rusqlite::Connection,
        client_key_id: &Uuid,
    ) -> Result<Vec<Secret>>;
    /// Update the client_key_id, encrypted_data_key, and updated_at fields for a list of secrets in a single transaction.
    fn update_secret_client_key(&self, conn: &rusqlite::Connection, secret: &Secret) -> Result<()>;
    /// Batch delete all expired secrets and return the count of deleted records.
    fn cleanup_expired_secrets(&self, conn: &rusqlite::Connection) -> Result<usize>;
    /// List all secrets with basic information (key, latest version, timestamps)
    fn list_secrets(&self, conn: &rusqlite::Connection) -> Result<Vec<SecretInfo>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientKeyStatus {
    Active,
}
impl ToSql for ClientKeyStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        match self {
            ClientKeyStatus::Active => Ok(rusqlite::types::ToSqlOutput::from("Active")),
        }
    }
}
impl FromSql for ClientKeyStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str() {
            Ok("Active") => Ok(ClientKeyStatus::Active),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

/// ClientKey struct, represents a row in the client_keys table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientKey {
    pub id: Uuid,                    // Unique identifier (e.g., UUID)
    pub public_key: String,          // Public key (PEM format)
    pub created_at: i64,             // Creation timestamp (Unix time)
    pub status: ClientKeyStatus,     // Status: Active
    pub description: Option<String>, // Optional description
    pub metadata: Option<String>,    // Optional metadata
    pub name: Option<String>,        // Optional client name (e.g., "morris-laptop")
}

impl ClientKey {
    pub(crate) fn new(public_key: String) -> Result<Self> {
        Self::new_with_name(public_key, None)
    }

    pub(crate) fn new_with_name(public_key: String, name: Option<String>) -> Result<Self> {
        let id = Uuid::new_v4();
        let created_at = time::OffsetDateTime::now_utc().unix_timestamp();
        let status = ClientKeyStatus::Active;
        let description = None;
        let metadata = None;

        Ok(ClientKey {
            id,
            public_key,
            created_at,
            status,
            description,
            metadata,
            name,
        })
    }
}

/// ClientKeyRepo trait for managing client_keys table
pub(crate) trait ClientKeyRepo: Send + Sync {
    fn create_client_key(&self, conn: &rusqlite::Connection, key: &ClientKey) -> Result<()>;
    fn fetch_all_client_keys(&self, conn: &rusqlite::Connection) -> Result<Vec<ClientKey>>;

    /// Fetch a specific client key by ID.
    fn fetch_client_key(
        &self,
        conn: &rusqlite::Connection,
        client_key_id: &Uuid,
    ) -> Result<Option<ClientKey>>;

    /// Fetch the PEM-encoded public key for a given client_key_id.
    fn fetch_public_key(
        &self,
        conn: &rusqlite::Connection,
        client_key_id: &Uuid,
    ) -> Result<Option<String>>;

    /// Fetch a valid client key.
    fn get_valid_client_key(&self, conn: &rusqlite::Connection) -> Result<ClientKey>;
}

pub(crate) trait HealthRepo: Send + Sync {
    fn check_health(&self, conn: &rusqlite::Connection) -> Result<bool>;
}

/// Represents an association between a secret and a client key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretClientKeyAssociation {
    pub secret_key: String,
    pub secret_version: i32,
    pub client_key_id: Uuid,
    pub encrypted_data_key: Vec<u8>,
    pub created_at: i64,
}

/// Repository for managing secret-client-key associations
pub(crate) trait SecretClientKeyRepo: Send + Sync {
    fn init_table(conn: &rusqlite::Connection) -> Result<()> where Self: Sized;
    #[allow(dead_code)] // Used only in tests
    fn create_association(
        &self,
        conn: &rusqlite::Connection,
        secret_key: &str,
        secret_version: i32,
        client_key_id: &Uuid,
        encrypted_data_key: &[u8],
    ) -> Result<()>;
    fn get_associations_for_secret(
        &self,
        conn: &rusqlite::Connection,
        secret_key: &str,
        secret_version: i32,
    ) -> Result<Vec<SecretClientKeyAssociation>>;
    fn get_association(
        &self,
        conn: &rusqlite::Connection,
        secret_key: &str,
        secret_version: i32,
        client_key_id: &Uuid,
    ) -> Result<Option<SecretClientKeyAssociation>>;
    fn remove_association(
        &self,
        conn: &rusqlite::Connection,
        secret_key: &str,
        secret_version: i32,
        client_key_id: &Uuid,
    ) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::client_key::generate_key_pair;
    use crate::repo::sqlite::SqliteSecretClientKeyRepo;

    #[test]
    fn test_client_key_new() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let client_key = ClientKey::new(public_pem.clone()).expect("Should create client key");

        assert_eq!(client_key.public_key, public_pem);
        assert!(matches!(client_key.status, ClientKeyStatus::Active));
        assert!(client_key.description.is_none());
        assert!(client_key.metadata.is_none());
        assert!(client_key.created_at > 0);
    }

    #[test]
    fn test_client_key_status_serialization() {
        // Test ToSql conversion
        let _active_sql = ClientKeyStatus::Active
            .to_sql()
            .expect("Should convert to SQL");

        // Test that conversion works without errors
    }

    #[test]
    fn test_secret_new() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let client_key = ClientKey::new(public_pem).expect("Should create client key");

        let secret_key = "test-secret";
        let secret_data = "This is secret data";
        let version = 1;
        let ttl = Some(3600); // 1 hour

        let secret = Secret::new(secret_key, secret_data, client_key.clone(), version, ttl)
            .expect("Should create secret");

        assert_eq!(secret.key, secret_key);
        assert_eq!(secret.version, version);
        assert_eq!(secret.client_key_id, client_key.id);
        assert!(secret.expires_at.is_some());
        assert!(secret.created_at > 0);
        assert_eq!(secret.created_at, secret.updated_at);
        assert!(!secret.encrypted_data.is_empty());
        assert!(!secret.encrypted_data_key.is_empty());
    }

    #[test]
    fn test_secret_new_without_ttl() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let client_key = ClientKey::new(public_pem).expect("Should create client key");

        let secret = Secret::new("test-key", "test-data", client_key, 1, None)
            .expect("Should create secret");

        assert!(secret.expires_at.is_none());
    }

    #[test]
    fn test_secret_encryption_is_different() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let client_key = ClientKey::new(public_pem).expect("Should create client key");

        let secret_data = "Same secret data";

        let secret1 = Secret::new("key1", secret_data, client_key.clone(), 1, None)
            .expect("Should create first secret");
        let secret2 = Secret::new("key2", secret_data, client_key, 2, None)
            .expect("Should create second secret");

        // Even with same data, encrypted results should be different due to random data keys
        assert_ne!(secret1.encrypted_data, secret2.encrypted_data);
        assert_ne!(secret1.encrypted_data_key, secret2.encrypted_data_key);
    }

    #[test]
    fn test_secret_rotate_client_key() {
        let (old_private_pem, old_public_pem) =
            generate_key_pair().expect("Should generate old key pair");
        let (_, new_public_pem) = generate_key_pair().expect("Should generate new key pair");

        let old_client_key = ClientKey::new(old_public_pem).expect("Should create old client key");
        let new_client_key = ClientKey::new(new_public_pem).expect("Should create new client key");

        let original_secret =
            Secret::new("test-key", "secret-data", old_client_key.clone(), 1, None)
                .expect("Should create secret");

        let original_created_at = original_secret.created_at;
        let original_encrypted_data = original_secret.encrypted_data.clone();
        let original_encrypted_data_key = original_secret.encrypted_data_key.clone();

        let rotated_secret = original_secret
            .rotate_client_key(
                &old_client_key.id,
                &old_private_pem,
                &new_client_key.id,
                &new_client_key.public_key,
            )
            .expect("Should rotate client key");

        // Key rotation should update client key ID and encrypted data key
        assert_eq!(rotated_secret.client_key_id, new_client_key.id);
        assert_ne!(
            rotated_secret.encrypted_data_key,
            original_encrypted_data_key
        );
        assert_eq!(rotated_secret.encrypted_data, original_encrypted_data); // Data itself unchanged
        assert!(rotated_secret.updated_at >= original_created_at);
    }

    #[test]
    fn test_secret_rotate_client_key_same_key() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let client_key = ClientKey::new(public_pem).expect("Should create client key");

        let original_secret = Secret::new("test-key", "secret-data", client_key.clone(), 1, None)
            .expect("Should create secret");

        // Rotating to the same key should return the secret unchanged
        let rotated_secret = original_secret
            .clone()
            .rotate_client_key(
                &client_key.id,
                "dummy-private-key",
                &client_key.id,
                &client_key.public_key,
            )
            .expect("Should handle same key rotation");

        assert_eq!(rotated_secret.client_key_id, original_secret.client_key_id);
        assert_eq!(
            rotated_secret.encrypted_data_key,
            original_secret.encrypted_data_key
        );
    }

    #[test]
    fn test_secret_rotate_client_key_wrong_old_key() {
        let (old_private_pem, old_public_pem) =
            generate_key_pair().expect("Should generate old key pair");
        let (_, new_public_pem) = generate_key_pair().expect("Should generate new key pair");
        let (_, wrong_public_pem) = generate_key_pair().expect("Should generate wrong key pair");

        let old_client_key = ClientKey::new(old_public_pem).expect("Should create old client key");
        let new_client_key = ClientKey::new(new_public_pem).expect("Should create new client key");
        let wrong_client_key =
            ClientKey::new(wrong_public_pem).expect("Should create wrong client key");

        let original_secret = Secret::new("test-key", "secret-data", old_client_key, 1, None)
            .expect("Should create secret");

        // Trying to rotate with wrong old key ID should fail
        let result = original_secret.rotate_client_key(
            &wrong_client_key.id, // Wrong old key ID
            &old_private_pem,
            &new_client_key.id,
            &new_client_key.public_key,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            SealboxError::ClientKeyMismatch(_, _, _) => {} // Expected
            _ => panic!("Expected ClientKeyMismatch error"),
        }
    }

    #[test]
    fn test_secret_rotate_client_key_invalid_private_key() {
        let (_, old_public_pem) = generate_key_pair().expect("Should generate old key pair");
        let (_, new_public_pem) = generate_key_pair().expect("Should generate new key pair");

        let old_client_key = ClientKey::new(old_public_pem).expect("Should create old client key");
        let new_client_key = ClientKey::new(new_public_pem).expect("Should create new client key");

        let original_secret =
            Secret::new("test-key", "secret-data", old_client_key.clone(), 1, None)
                .expect("Should create secret");

        // Invalid private key should cause rotation to fail
        let result = original_secret.rotate_client_key(
            &old_client_key.id,
            "invalid-private-key",
            &new_client_key.id,
            &new_client_key.public_key,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_ttl_calculation() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let client_key = ClientKey::new(public_pem).expect("Should create client key");

        let ttl_seconds = 7200i64; // 2 hours
        let secret = Secret::new("test-key", "test-data", client_key, 1, Some(ttl_seconds))
            .expect("Should create secret");

        let expected_expiry = secret.created_at + ttl_seconds;
        assert_eq!(secret.expires_at, Some(expected_expiry));
    }

    #[test]
    fn test_client_key_with_name_field() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let mut client_key = ClientKey::new(public_pem).expect("Should create client key");

        // This should fail compilation until we add name field to ClientKey
        client_key.name = Some("test-laptop".to_string());
        assert_eq!(client_key.name, Some("test-laptop".to_string()));

        // Test that name is optional
        let client_key_no_name = ClientKey::new_with_name(
            "-----BEGIN RSA PUBLIC KEY-----\nMIIBCgKCAQEA...\n-----END RSA PUBLIC KEY-----"
                .to_string(),
            None,
        )
        .expect("Should create client key without name");
        assert_eq!(client_key_no_name.name, None);

        // Test that name is stored correctly
        let client_key_with_name = ClientKey::new_with_name(
            "-----BEGIN RSA PUBLIC KEY-----\nMIIBCgKCAQEA...\n-----END RSA PUBLIC KEY-----"
                .to_string(),
            Some("my-laptop".to_string()),
        )
        .expect("Should create client key with name");
        assert_eq!(client_key_with_name.name, Some("my-laptop".to_string()));
    }

    #[test]
    fn test_secret_with_multiple_client_keys_concept() {
        let (_, public_pem1) = generate_key_pair().expect("Should generate key pair 1");
        let (_, public_pem2) = generate_key_pair().expect("Should generate key pair 2");

        let client_key1 = ClientKey::new(public_pem1).expect("Should create client key 1");
        let _client_key2 = ClientKey::new(public_pem2).expect("Should create client key 2");

        // Current behavior: can only create secret with single client key
        let secret = Secret::new("test-key", "test-data", client_key1.clone(), 1, None)
            .expect("Should create secret with single client key");

        assert_eq!(secret.client_key_id, client_key1.id);

        // TODO: Later we want to create secret with multiple client keys
        // let client_key_ids = vec![client_key1.id, client_key2.id];
        // let multi_secret = Secret::new_with_multiple_keys("multi-key", "data", client_key_ids, 1, None);
        // assert!(multi_secret.is_ok());
    }

    #[test]
    fn test_secret_client_keys_table_operations() {
        let conn = rusqlite::Connection::open_in_memory().expect("Should create in-memory DB");

        // Initialize tables
        SqliteClientKeyRepo::init_table(&conn).expect("Should init client_keys table");
        SqliteSecretRepo::init_table(&conn).expect("Should init secrets table");

        // This should fail until we implement secret_client_keys table
        SqliteSecretClientKeyRepo::init_table(&conn).expect("Should init secret_client_keys table");

        // Create test client keys
        let client_key_repo = SqliteClientKeyRepo;
        let client_key1 = ClientKey::new_with_name(
            "-----BEGIN RSA PUBLIC KEY-----\ntest1\n-----END RSA PUBLIC KEY-----".to_string(),
            Some("laptop-1".to_string()),
        )
        .expect("Should create client key 1");
        let client_key2 = ClientKey::new_with_name(
            "-----BEGIN RSA PUBLIC KEY-----\ntest2\n-----END RSA PUBLIC KEY-----".to_string(),
            Some("laptop-2".to_string()),
        )
        .expect("Should create client key 2");

        client_key_repo
            .create_client_key(&conn, &client_key1)
            .expect("Should store client key 1");
        client_key_repo
            .create_client_key(&conn, &client_key2)
            .expect("Should store client key 2");

        // Test secret-client-key associations
        let secret_client_key_repo = SqliteSecretClientKeyRepo;
        let secret_key = "test-multi-secret";
        let secret_version = 1;
        let data_key_1 = vec![1, 2, 3, 4]; // Simulated encrypted data key for client_key1
        let data_key_2 = vec![5, 6, 7, 8]; // Simulated encrypted data key for client_key2

        // This should fail until we implement the methods
        secret_client_key_repo
            .create_association(
                &conn,
                secret_key,
                secret_version,
                &client_key1.id,
                &data_key_1,
            )
            .expect("Should create association 1");

        secret_client_key_repo
            .create_association(
                &conn,
                secret_key,
                secret_version,
                &client_key2.id,
                &data_key_2,
            )
            .expect("Should create association 2");

        // Test querying associations
        let associations = secret_client_key_repo
            .get_associations_for_secret(&conn, secret_key, secret_version)
            .expect("Should get associations for secret");

        assert_eq!(associations.len(), 2);
        assert!(
            associations
                .iter()
                .any(|a| a.client_key_id == client_key1.id)
        );
        assert!(
            associations
                .iter()
                .any(|a| a.client_key_id == client_key2.id)
        );

        // Test getting specific association
        let association = secret_client_key_repo
            .get_association(&conn, secret_key, secret_version, &client_key1.id)
            .expect("Should get specific association")
            .expect("Association should exist");

        assert_eq!(association.encrypted_data_key, data_key_1);
        assert_eq!(association.client_key_id, client_key1.id);
    }

    #[test]
    fn test_multi_client_key_backward_compatibility() {
        // This test ensures our changes don't break existing functionality

        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let client_key = ClientKey::new(public_pem).expect("Should create client key");

        // Current single-client-key functionality should continue working
        let secret = Secret::new("compat-test", "secret-data", client_key.clone(), 1, None)
            .expect("Should create secret with single client key");

        assert_eq!(secret.key, "compat-test");
        assert_eq!(secret.client_key_id, client_key.id);
        assert!(!secret.encrypted_data.is_empty());
        assert!(!secret.encrypted_data_key.is_empty());

        // This should pass immediately, ensuring backward compatibility
    }
}
