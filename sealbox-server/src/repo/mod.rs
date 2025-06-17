use serde::Serialize;

use crate::{
    crypto::{
        decrypt_data_key, encrypt_data, encrypt_data_key, generate_data_key,
        generate_master_key_pair,
    },
    error::{Result, SealboxError},
};

pub(crate) use self::sqlite::{SqliteMasterKeyRepo, SqliteSecretRepo, create_db_connection};

mod sqlite;

#[derive(Debug, Serialize, Clone)]
pub struct Secret {
    pub namespace: String,           // Secret namespace, used for logical grouping
    pub key: String,                 // Secret key identifier
    pub version: i32,                // Version number, incremented on each insert
    pub encrypted_data: Vec<u8>,     // The encrypted secret value
    pub encrypted_data_key: Vec<u8>, // The data key encrypted with user's public key
    pub master_key_id: String,       // References master_keys.id (public key used)
    pub created_at: i64,             // Creation timestamp (Unix time)
    pub updated_at: i64,             // Last update timestamp (Unix time)
    pub expires_at: Option<i64>,     // Expiry timestamp (Unix time), optional for TTL
    pub metadata: Option<String>,    // Optional metadata in serialized format
}

impl Secret {
    pub(crate) fn new(key: &str, data: &str, master_key: MasterKey) -> Result<Self> {
        let data_bytes = data.as_bytes();

        let data_key = generate_data_key()?;
        let encrypted_data_key = encrypt_data_key(&data_key, &master_key.public_key)?;
        let encrypted_data = encrypt_data(&data_bytes, &encrypted_data_key)?;

        let now_timestamp = time::OffsetDateTime::now_utc().unix_timestamp();

        Ok(Self {
            namespace: String::new(),
            key: key.to_string(),
            version: 1,
            encrypted_data,
            encrypted_data_key,
            master_key_id: master_key.id,
            created_at: now_timestamp,
            updated_at: now_timestamp,
            expires_at: None,
            metadata: None,
        })
    }

    pub(crate) fn rotate_master_key(
        self,
        old_master_key_id: &str,
        old_private_key_pem: &str,
        new_master_key_id: &str,
        new_public_key_pem: &str,
    ) -> Result<Self> {
        let mut secret = self.clone();

        if secret.master_key_id == new_master_key_id {
            return Ok(secret);
        }

        if secret.master_key_id != old_master_key_id {
            return Err(SealboxError::MasterKeyNotMatch(
                secret.key,
                old_master_key_id.to_string(),
                secret.master_key_id,
            ));
        }

        let data_key = decrypt_data_key(&secret.encrypted_data_key, &old_private_key_pem)?;
        let new_encrypted_data_key = encrypt_data_key(&data_key, &new_public_key_pem)?;

        secret.encrypted_data_key = new_encrypted_data_key;
        secret.master_key_id = new_master_key_id.to_string();
        secret.updated_at = time::OffsetDateTime::now_utc().unix_timestamp();

        Ok(secret)
    }
}

pub(crate) trait SecretRepo: Send + Sync {
    fn get_secret(&self, key: &str) -> Result<Secret>;
    fn save_secret(&self, secret: &Secret) -> Result<()>;
    fn delete_secret(&self, key: &str) -> Result<()>;

    /// Fetch all secrets using the given master_key_id.
    fn fetch_secrets_by_master_key(&self, master_key_id: &str) -> Result<Vec<Secret>>;
    /// Update the master_key_id, encrypted_data_key, and updated_at fields for a secret.
    fn update_secret_master_key(&self, secret: &Secret) -> Result<()>;
}

/// MasterKey struct, represents a row in the master_keys table
#[derive(Debug, Serialize)]
pub struct MasterKey {
    pub id: String,                  // Unique identifier (e.g., UUID)
    pub public_key: String,          // Public key (PEM format)
    pub created_at: i64,             // Creation timestamp (Unix time)
    pub status: String,              // Status: active/retired/disabled
    pub description: Option<String>, // Optional description
    pub version: Option<i32>,        // Optional version
    pub metadata: Option<String>,    // Optional metadata
}

impl MasterKey {
    pub(crate) fn create_key_pair() -> Result<(Self, String)> {
        let (private_key, public_key) = generate_master_key_pair()?;
        let id = "UUID::new_v4().to_string()".to_string();
        let created_at = time::OffsetDateTime::now_utc().unix_timestamp();
        let status = "active".to_string();
        let description = None;
        let version = None;
        let metadata = None;

        Ok((
            MasterKey {
                id,
                public_key,
                created_at,
                status,
                description,
                version,
                metadata,
            },
            private_key,
        ))
    }
}

/// MasterKeyRepo trait for managing master_keys table
pub(crate) trait MasterKeyRepo: Send + Sync {
    fn create_master_key(&self, key: &MasterKey) -> Result<()>;
    fn delete_master_key(&self, id: &str) -> Result<()>;

    /// Fetch the PEM-encoded public key for a given master_key_id.
    fn fetch_public_key(&self, master_key_id: &str) -> Result<Option<String>>;

    /// Fetch a valid master key.
    fn get_valid_master_key(&self) -> Result<Option<MasterKey>>;
}
