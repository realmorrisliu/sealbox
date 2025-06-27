use rusqlite::{ToSql, types::FromSql};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    crypto::{decrypt_data_key, encrypt_data, encrypt_data_key, generate_data_key},
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

        let data_key = generate_data_key()?;
        let encrypted_data = encrypt_data(&data_bytes, &data_key)?;
        let encrypted_data_key = encrypt_data_key(&data_key, &master_key.public_key)?;

        let now_timestamp = time::OffsetDateTime::now_utc().unix_timestamp();

        let expires_at = ttl.map(|ttl| now_timestamp + ttl);

        Ok(Self {
            namespace: String::new(),
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

    pub(crate) fn rotate_master_key(
        self,
        old_master_key_id: &Uuid,
        old_private_key_pem: &str,
        new_master_key_id: &Uuid,
        new_public_key_pem: &str,
    ) -> Result<Self> {
        let mut secret = self.clone();

        if secret.master_key_id == *new_master_key_id {
            return Ok(secret);
        }

        if secret.master_key_id != *old_master_key_id {
            return Err(SealboxError::MasterKeyNotMatch(
                secret.key,
                old_master_key_id.to_string(),
                secret.master_key_id.to_string(),
            ));
        }

        let data_key = decrypt_data_key(&secret.encrypted_data_key, &old_private_key_pem)?;
        let new_encrypted_data_key = encrypt_data_key(&data_key, &new_public_key_pem)?;

        secret.encrypted_data_key = new_encrypted_data_key;
        secret.master_key_id = new_master_key_id.clone();
        secret.updated_at = time::OffsetDateTime::now_utc().unix_timestamp();

        Ok(secret)
    }
}

pub(crate) trait SecretRepo: Send + Sync {
    fn get_secret(&self, conn: &rusqlite::Connection, key: &str) -> Result<Secret>;
    fn get_secret_by_version(
        &self,
        conn: &rusqlite::Connection,
        key: &str,
        version: i32,
    ) -> Result<Secret>;
    fn create_new_version(
        &self,
        conn: &mut rusqlite::Connection,
        key: &str,
        data: &str,
        master_key: MasterKey,
        ttl: Option<i64>,
    ) -> Result<Secret>;
    fn delete_secret_by_version(
        &self,
        conn: &rusqlite::Connection,
        key: &str,
        version: i32,
    ) -> Result<()>;

    /// Fetch all secrets using the given master_key_id.
    fn fetch_secrets_by_master_key(
        &self,
        conn: &rusqlite::Connection,
        master_key_id: &Uuid,
    ) -> Result<Vec<Secret>>;
    /// Update the master_key_id, encrypted_data_key, and updated_at fields for a list of secrets in a single transaction.
    fn update_secret_master_key(&self, conn: &rusqlite::Connection, secret: &Secret) -> Result<()>;
}

#[derive(Debug, Serialize)]
pub enum MasterKeyStatus {
    Active,
    Retired,
    Disabled,
}
impl ToSql for MasterKeyStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
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
#[derive(Debug, Serialize)]
pub struct MasterKey {
    pub id: Uuid,                    // Unique identifier (e.g., UUID)
    pub public_key: String,          // Public key (PEM format)
    pub created_at: i64,             // Creation timestamp (Unix time)
    pub status: MasterKeyStatus,     // Status: Active/Retired/Disabled
    pub description: Option<String>, // Optional description
    pub metadata: Option<String>,    // Optional metadata
}

impl MasterKey {
    pub(crate) fn new(public_key: String) -> Result<Self> {
        let id = Uuid::new_v4();
        let created_at = time::OffsetDateTime::now_utc().unix_timestamp();
        let status = MasterKeyStatus::Active;
        let description = None;
        let metadata = None;

        Ok(MasterKey {
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
    fn fetch_all_master_keys(&self, conn: &rusqlite::Connection) -> Result<Vec<MasterKey>>;

    /// Fetch the PEM-encoded public key for a given master_key_id.
    fn fetch_public_key(
        &self,
        conn: &rusqlite::Connection,
        master_key_id: &Uuid,
    ) -> Result<Option<String>>;

    /// Fetch a valid master key.
    fn get_valid_master_key(&self, conn: &rusqlite::Connection) -> Result<Option<MasterKey>>;
}
