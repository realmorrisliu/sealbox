#[cfg(test)]
use std::str::FromStr;

use rusqlite::{ToSql, types::FromSql};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(test)]
use crate::crypto::{data_key::DataKey, master_key::PublicMasterKey};
use crate::error::Result;

pub use self::sqlite::{MigrationReport, inspect_migration_path};
pub(crate) use self::sqlite::{
    SqliteHealthRepo, SqliteMasterKeyRepo, SqliteSecretRepo, SqliteTenantRepo,
    backup_before_migration, create_db_connection, run_migrations,
};

mod sqlite;

pub const LEGACY_TENANT_ID: &str = "legacy";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretInfo {
    pub key: String,              // Secret key identifier
    pub version: i32,             // Secret version number
    pub created_at: i64,          // Creation timestamp (Unix time)
    pub updated_at: i64,          // Last update timestamp (Unix time)
    pub expires_at: Option<i64>,  // Expiry timestamp (Unix time), optional for TTL
    pub metadata: Option<String>, // Optional plaintext metadata for search/display
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub namespace: String,           // Secret namespace, used for logical grouping
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

pub(crate) struct EncryptedSecretInput {
    pub encrypted_data: Vec<u8>,
    pub encrypted_data_key: Vec<u8>,
    pub master_key_id: Uuid,
    pub ttl: Option<i64>,
    pub metadata: Option<String>,
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
    #[cfg(test)]
    pub(crate) fn new(
        namespace: &str,
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
            namespace: namespace.to_string(),
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

    pub(crate) fn from_encrypted(
        namespace: &str,
        key: &str,
        version: i32,
        input: EncryptedSecretInput,
    ) -> Result<Self> {
        let now_timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
        let expires_at = input.ttl.map(|ttl| now_timestamp + ttl);

        Ok(Self {
            namespace: namespace.to_string(),
            key: key.to_string(),
            version,
            encrypted_data: input.encrypted_data,
            encrypted_data_key: input.encrypted_data_key,
            master_key_id: input.master_key_id,
            created_at: now_timestamp,
            updated_at: now_timestamp,
            expires_at,
            metadata: input.metadata,
        })
    }
}

pub(crate) trait SecretRepo: Send + Sync {
    /// Get latest secret with atomic lazy cleanup
    fn get_secret(
        &self,
        conn: &mut rusqlite::Connection,
        namespace: &str,
        key: &str,
    ) -> Result<Secret>;
    /// Get specific version secret with atomic lazy cleanup
    fn get_secret_by_version(
        &self,
        conn: &mut rusqlite::Connection,
        namespace: &str,
        key: &str,
        version: i32,
    ) -> Result<Secret>;
    #[cfg(test)]
    fn create_new_version(
        &self,
        conn: &mut rusqlite::Connection,
        namespace: &str,
        key: &str,
        data: &str,
        master_key: MasterKey,
        ttl: Option<i64>,
    ) -> Result<Secret>;
    fn create_new_encrypted_version(
        &self,
        conn: &mut rusqlite::Connection,
        namespace: &str,
        key: &str,
        input: EncryptedSecretInput,
    ) -> Result<Secret>;
    fn delete_secret(&self, conn: &rusqlite::Connection, namespace: &str, key: &str) -> Result<()>;
    fn delete_secret_by_version(
        &self,
        conn: &rusqlite::Connection,
        namespace: &str,
        key: &str,
        version: i32,
    ) -> Result<()>;

    /// Fetch all secrets using the given master_key_id.
    fn fetch_secrets_by_master_key(
        &self,
        conn: &rusqlite::Connection,
        namespace: &str,
        master_key_id: &Uuid,
    ) -> Result<Vec<Secret>>;
    /// Update the master_key_id, encrypted_data_key, and updated_at fields for a list of secrets in a single transaction.
    fn update_secret_master_key(&self, conn: &rusqlite::Connection, secret: &Secret) -> Result<()>;
    /// Batch delete all expired secrets and return the count of deleted records.
    fn cleanup_expired_secrets(&self, conn: &rusqlite::Connection) -> Result<usize>;
    /// List all secrets with basic information (key, latest version, timestamps)
    fn list_secrets(&self, conn: &rusqlite::Connection, namespace: &str)
    -> Result<Vec<SecretInfo>>;
    /// List retained, non-expired versions for one secret key.
    fn list_secret_versions(
        &self,
        conn: &rusqlite::Connection,
        namespace: &str,
        key: &str,
    ) -> Result<Vec<SecretInfo>>;
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
    pub namespace: String,           // Tenant scope that owns this key
    pub id: Uuid,                    // Unique identifier (e.g., UUID)
    pub public_key: String,          // Public key (PEM format)
    pub created_at: i64,             // Creation timestamp (Unix time)
    pub status: MasterKeyStatus,     // Status: Active/Retired/Disabled
    pub description: Option<String>, // Optional description
    pub metadata: Option<String>,    // Optional metadata
}

impl MasterKey {
    pub(crate) fn new(public_key: String) -> Result<Self> {
        Self::new_in_namespace(LEGACY_TENANT_ID.to_string(), public_key)
    }

    pub(crate) fn new_in_namespace(namespace: String, public_key: String) -> Result<Self> {
        let id = Uuid::new_v4();
        let created_at = time::OffsetDateTime::now_utc().unix_timestamp();
        let status = MasterKeyStatus::Active;
        let description = None;
        let metadata = None;

        Ok(MasterKey {
            namespace,
            id,
            public_key,
            created_at,
            status,
            description,
            metadata,
        })
    }
}

/// MasterKeyRepo trait for managing master_keys table
pub(crate) trait MasterKeyRepo: Send + Sync {
    fn create_master_key(&self, conn: &rusqlite::Connection, key: &MasterKey) -> Result<()>;
    fn fetch_all_master_keys(
        &self,
        conn: &rusqlite::Connection,
        namespace: &str,
    ) -> Result<Vec<MasterKey>>;
    fn fetch_master_key(
        &self,
        conn: &rusqlite::Connection,
        namespace: &str,
        master_key_id: &Uuid,
    ) -> Result<Option<MasterKey>>;

    /// Fetch a valid master key.
    fn get_valid_master_key(
        &self,
        conn: &rusqlite::Connection,
        namespace: &str,
    ) -> Result<MasterKey>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TenantStatus {
    Active,
    Suspended,
}

impl ToSql for TenantStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let value = match self {
            TenantStatus::Active => "Active",
            TenantStatus::Suspended => "Suspended",
        };
        Ok(rusqlite::types::ToSqlOutput::from(value))
    }
}

impl FromSql for TenantStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str()? {
            "Active" => Ok(TenantStatus::Active),
            "Suspended" => Ok(TenantStatus::Suspended),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub display_name: Option<String>,
    pub status: TenantStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTokenMetadata {
    pub id: Uuid,
    pub tenant_id: String,
    pub label: Option<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub revoked_at: Option<i64>,
    pub last_used_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedTenant {
    pub tenant_id: String,
    pub token_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct IssuedApiToken {
    pub metadata: ApiTokenMetadata,
    pub token: String,
}

pub(crate) trait TenantRepo: Send + Sync {
    fn create_tenant(
        &self,
        conn: &rusqlite::Connection,
        display_name: Option<String>,
    ) -> Result<Tenant>;
    fn list_tenants(&self, conn: &rusqlite::Connection) -> Result<Vec<Tenant>>;
    fn get_tenant(&self, conn: &rusqlite::Connection, tenant_id: &str) -> Result<Option<Tenant>>;
    fn set_tenant_status(
        &self,
        conn: &rusqlite::Connection,
        tenant_id: &str,
        status: TenantStatus,
    ) -> Result<Tenant>;
    fn issue_token(
        &self,
        conn: &rusqlite::Connection,
        tenant_id: &str,
        label: Option<String>,
        expires_at: Option<i64>,
    ) -> Result<IssuedApiToken>;
    fn list_tokens(
        &self,
        conn: &rusqlite::Connection,
        tenant_id: &str,
    ) -> Result<Vec<ApiTokenMetadata>>;
    fn revoke_token(
        &self,
        conn: &rusqlite::Connection,
        tenant_id: &str,
        token_id: &Uuid,
    ) -> Result<()>;
    fn authenticate_token(
        &self,
        conn: &rusqlite::Connection,
        token: &str,
    ) -> Result<Option<AuthenticatedTenant>>;
}

pub(crate) trait HealthRepo: Send + Sync {
    fn check_health(&self, conn: &rusqlite::Connection) -> Result<bool>;
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

        let secret = Secret::new(
            LEGACY_TENANT_ID,
            secret_key,
            secret_data,
            master_key.clone(),
            version,
            ttl,
        )
        .expect("Should create secret");

        assert_eq!(secret.key, secret_key);
        assert_eq!(secret.version, version);
        assert_eq!(secret.master_key_id, master_key.id);
        assert!(secret.expires_at.is_some());
        assert!(secret.created_at > 0);
        assert_eq!(secret.created_at, secret.updated_at);
        assert!(!secret.encrypted_data.is_empty());
        assert!(!secret.encrypted_data_key.is_empty());
        assert_eq!(secret.namespace, LEGACY_TENANT_ID);
        assert!(secret.metadata.is_none());
    }

    #[test]
    fn test_secret_new_without_ttl() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let secret = Secret::new(
            LEGACY_TENANT_ID,
            "test-key",
            "test-data",
            master_key,
            1,
            None,
        )
        .expect("Should create secret");

        assert!(secret.expires_at.is_none());
    }

    #[test]
    fn test_secret_encryption_is_different() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let secret_data = "Same secret data";

        let secret1 = Secret::new(
            LEGACY_TENANT_ID,
            "key1",
            secret_data,
            master_key.clone(),
            1,
            None,
        )
        .expect("Should create first secret");
        let secret2 = Secret::new(LEGACY_TENANT_ID, "key2", secret_data, master_key, 2, None)
            .expect("Should create second secret");

        // Even with same data, encrypted results should be different due to random data keys
        assert_ne!(secret1.encrypted_data, secret2.encrypted_data);
        assert_ne!(secret1.encrypted_data_key, secret2.encrypted_data_key);
    }

    #[test]
    fn test_ttl_calculation() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let master_key = MasterKey::new(public_pem).expect("Should create master key");

        let ttl_seconds = 7200i64; // 2 hours
        let secret = Secret::new(
            LEGACY_TENANT_ID,
            "test-key",
            "test-data",
            master_key,
            1,
            Some(ttl_seconds),
        )
        .expect("Should create secret");

        let expected_expiry = secret.created_at + ttl_seconds;
        assert_eq!(secret.expires_at, Some(expected_expiry));
    }
}
