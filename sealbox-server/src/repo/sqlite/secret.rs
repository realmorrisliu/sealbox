use sqlx::SqlitePool;
use tracing::info;
use uuid::Uuid;

use crate::{
    error::{Result, SealboxError},
    repo::{ClientKey, Secret, SecretInfo, SecretRepo},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteSecretRepo;

impl SqliteSecretRepo {
    /// Helper function to check expiry and clean up expired secrets atomically
    async fn check_and_cleanup_expired(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        secret: &Secret,
    ) -> Result<Option<Secret>> {
        if let Some(expires_at) = secret.expires_at {
            let now = time::OffsetDateTime::now_utc().unix_timestamp();
            if expires_at < now {
                // Secret has expired, delete it atomically within transaction
                sqlx::query("DELETE FROM secrets WHERE key = ?1 AND version = ?2")
                    .bind(&secret.key)
                    .bind(secret.version)
                    .execute(&mut **tx)
                    .await?;
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
    async fn get_secret_with_query(
        &self,
        pool: &SqlitePool,
        query: &str,
        key: &str,
        version: Option<i32>,
    ) -> Result<Secret> {
        let mut tx = pool.begin().await?;

        let secret = if let Some(v) = version {
            sqlx::query_as::<_, Secret>(query)
                .bind(key)
                .bind(v)
                .fetch_optional(&mut *tx)
                .await?
        } else {
            sqlx::query_as::<_, Secret>(query)
                .bind(key)
                .fetch_optional(&mut *tx)
                .await?
        };

        match secret {
            Some(secret) => match Self::check_and_cleanup_expired(&mut tx, &secret).await? {
                Some(valid_secret) => {
                    tx.commit().await?;
                    Ok(valid_secret)
                }
                None => {
                    tx.commit().await?;
                    Err(SealboxError::SecretNotFound(key.to_string()))
                }
            },
            None => {
                tx.commit().await?;
                Err(SealboxError::SecretNotFound(key.to_string()))
            }
        }
    }
}

#[async_trait::async_trait]
impl SecretRepo for SqliteSecretRepo {
    async fn init_table(pool: &SqlitePool) -> Result<()>
    where
        Self: Sized,
    {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS secrets (
                key TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                encrypted_data BLOB NOT NULL,
                encrypted_data_key BLOB NOT NULL,
                client_key_id BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                expires_at INTEGER,
                PRIMARY KEY (key, version)
            )",
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn get_secret(&self, pool: &SqlitePool, key: &str) -> Result<Secret> {
        info!("get_secret: key={}", key);

        self.get_secret_with_query(
            pool,
            "SELECT key, version, encrypted_data, encrypted_data_key, client_key_id, created_at, updated_at, expires_at
            FROM secrets
            WHERE key = ?1
            ORDER BY version DESC
            LIMIT 1",
            key,
            None,
        ).await
    }

    async fn get_secret_by_version(
        &self,
        pool: &SqlitePool,
        key: &str,
        version: i32,
    ) -> Result<Secret> {
        info!("get_secret_by_version: key={}, version={}", key, version);

        self.get_secret_with_query(
            pool,
            "SELECT key, version, encrypted_data, encrypted_data_key, client_key_id, created_at, updated_at, expires_at
            FROM secrets
            WHERE key = ?1 AND version = ?2
            LIMIT 1",
            key,
            Some(version),
        ).await
    }

    async fn create_new_version(
        &self,
        pool: &SqlitePool,
        key: &str,
        data: &str,
        client_key: ClientKey,
        ttl: Option<i64>,
    ) -> Result<Secret> {
        info!("create_new_version");

        let mut tx = pool.begin().await?;

        let next_version = {
            let row: Option<(i32,)> =
                sqlx::query_as("SELECT COALESCE(MAX(version), 0) FROM secrets WHERE key = ?1")
                    .bind(key)
                    .fetch_optional(&mut *tx)
                    .await?;

            row.map(|(v,)| v).unwrap_or(0) + 1
        };

        let secret = Secret::new(key, data, client_key, next_version, ttl)?;

        sqlx::query(
            "INSERT INTO secrets (
              key, version, encrypted_data, encrypted_data_key,
              client_key_id, created_at, updated_at, expires_at
          ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&secret.key)
        .bind(secret.version)
        .bind(&secret.encrypted_data)
        .bind(&secret.encrypted_data_key)
        .bind(secret.client_key_id)
        .bind(secret.created_at)
        .bind(secret.updated_at)
        .bind(secret.expires_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(secret)
    }

    async fn create_new_version_multi_client(
        &self,
        pool: &SqlitePool,
        key: &str,
        data: &str,
        client_key_ids: &[Uuid],
        ttl: Option<i64>,
    ) -> Result<Secret> {
        info!("create_new_version_multi_client");

        if client_key_ids.is_empty() {
            return Err(SealboxError::InvalidInput(
                "No client keys provided".to_string(),
            ));
        }

        let mut tx = pool.begin().await?;

        // Get next version number
        let next_version = {
            let row: Option<(i32,)> =
                sqlx::query_as("SELECT COALESCE(MAX(version), 0) FROM secrets WHERE key = ?1")
                    .bind(key)
                    .fetch_optional(&mut *tx)
                    .await?;

            row.map(|(v,)| v).unwrap_or(0) + 1
        };

        // Fetch all client keys and validate they exist
        let mut client_keys = Vec::new();
        for client_key_id in client_key_ids {
            let client_key: Option<ClientKey> = sqlx::query_as(
                "SELECT id, public_key, created_at, status, description, metadata, name, last_used_at 
                FROM client_keys WHERE id = ?1 LIMIT 1"
            )
            .bind(client_key_id)
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(client_key) = client_key {
                client_keys.push(client_key);
            } else {
                return Err(SealboxError::ClientKeyNotFound(*client_key_id));
            }
        }

        // Implement true shared DataKey design
        use crate::crypto::{client_key::PublicClientKey, data_key::DataKey};
        use std::str::FromStr;

        // Generate a shared DataKey for encrypting the actual secret data
        let data_key = DataKey::new();
        let encrypted_data = data_key.encrypt(data.as_bytes())?;

        let now_timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
        let expires_at = ttl.map(|ttl| now_timestamp + ttl);

        // Create the base secret record using the first client key
        let first_client_key = &client_keys[0];
        let first_pub_key = PublicClientKey::from_str(&first_client_key.public_key)?;
        let first_encrypted_data_key = first_pub_key.encrypt(data_key.as_bytes())?;

        // Insert the secret
        sqlx::query(
            "INSERT INTO secrets (
              key, version, encrypted_data, encrypted_data_key,
              client_key_id, created_at, updated_at, expires_at
          ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(key)
        .bind(next_version)
        .bind(&encrypted_data)
        .bind(&first_encrypted_data_key)
        .bind(first_client_key.id)
        .bind(now_timestamp)
        .bind(now_timestamp)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;

        // Create associations for ALL client keys
        for client_key in &client_keys {
            let pub_key = PublicClientKey::from_str(&client_key.public_key)?;
            let encrypted_data_key = pub_key.encrypt(data_key.as_bytes())?;

            sqlx::query(
                "INSERT INTO secret_client_keys (secret_key, secret_version, client_key_id, encrypted_data_key, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5)"
            )
            .bind(key)
            .bind(next_version)
            .bind(client_key.id)
            .bind(&encrypted_data_key)
            .bind(now_timestamp)
            .execute(&mut *tx)
            .await?;
        }

        let secret = Secret {
            key: key.to_string(),
            version: next_version,
            encrypted_data,
            encrypted_data_key: first_encrypted_data_key,
            client_key_id: first_client_key.id,
            created_at: now_timestamp,
            updated_at: now_timestamp,
            expires_at,
        };

        tx.commit().await?;
        Ok(secret)
    }

    async fn delete_secret_by_version(
        &self,
        pool: &SqlitePool,
        key: &str,
        version: i32,
    ) -> Result<()> {
        info!("delete_secret_by_version");

        let result = sqlx::query("DELETE FROM secrets WHERE key = ?1 AND version = ?2")
            .bind(key)
            .bind(version)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(SealboxError::SecretNotFound(key.to_string()));
        }
        Ok(())
    }

    async fn fetch_secrets_by_client_key(
        &self,
        pool: &SqlitePool,
        client_key_id: &Uuid,
    ) -> Result<Vec<Secret>> {
        let secrets: Vec<Secret> = sqlx::query_as(
            "SELECT key, version, encrypted_data, encrypted_data_key, client_key_id, created_at, updated_at, expires_at
            FROM secrets WHERE client_key_id = ?1"
        )
        .bind(client_key_id)
        .fetch_all(pool)
        .await?;

        Ok(secrets)
    }

    async fn update_secret_client_key_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        secret: &Secret,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE secrets SET
                encrypted_data_key = ?1,
                client_key_id = ?2,
                updated_at = ?3
             WHERE key = ?4 AND version = ?5",
        )
        .bind(&secret.encrypted_data_key)
        .bind(secret.client_key_id)
        .bind(secret.updated_at)
        .bind(&secret.key)
        .bind(secret.version)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn cleanup_expired_secrets(&self, pool: &SqlitePool) -> Result<usize> {
        info!("cleanup_expired_secrets");
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        let result =
            sqlx::query("DELETE FROM secrets WHERE expires_at IS NOT NULL AND expires_at < ?1")
                .bind(now)
                .execute(pool)
                .await?;

        let deleted_count = result.rows_affected() as usize;
        info!("Cleaned up {} expired secrets", deleted_count);
        Ok(deleted_count)
    }

    async fn list_secrets(&self, pool: &SqlitePool) -> Result<Vec<SecretInfo>> {
        info!("list_secrets");
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        let secret_infos: Vec<SecretInfo> = sqlx::query_as(
            "SELECT key, MAX(version) as version, created_at, MAX(updated_at) as updated_at, expires_at
            FROM secrets
            WHERE expires_at IS NULL OR expires_at > ?1
            GROUP BY key
            ORDER BY updated_at DESC"
        )
        .bind(now)
        .fetch_all(pool)
        .await?;

        Ok(secret_infos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Should create in-memory DB");
        SqliteSecretRepo::init_table(&pool)
            .await
            .expect("Should initialize tables");
        pool
    }

    #[tokio::test]
    async fn test_init_table() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        SqliteSecretRepo::init_table(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn test_secret_not_found() {
        let pool = setup_test_db().await;
        let repo = SqliteSecretRepo;

        let result = repo.get_secret(&pool, "nonexistent").await;
        assert!(matches!(result, Err(SealboxError::SecretNotFound(_))));
    }
}
