use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::{Result, SealboxError},
    repo::{SecretClientKeyAssociation, SecretClientKeyRepo},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteSecretClientKeyRepo;

#[async_trait::async_trait]
impl SecretClientKeyRepo for SqliteSecretClientKeyRepo {
    async fn init_table(pool: &SqlitePool) -> Result<()>
    where
        Self: Sized,
    {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS secret_client_keys (
                secret_key TEXT NOT NULL,
                secret_version INTEGER NOT NULL,
                client_key_id BLOB NOT NULL,
                encrypted_data_key BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (secret_key, secret_version, client_key_id)
            )",
        )
        .execute(pool)
        .await?;

        // Create indexes for performance optimization
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_secret_client_keys_client_key 
             ON secret_client_keys(client_key_id, secret_key)",
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_secret_client_keys_secret_client 
             ON secret_client_keys(secret_key, secret_version, client_key_id)",
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_secret_client_keys_created_at 
             ON secret_client_keys(created_at)",
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn create_association(
        &self,
        pool: &SqlitePool,
        secret_key: &str,
        secret_version: i32,
        client_key_id: &Uuid,
        encrypted_data_key: &[u8],
    ) -> Result<()> {
        let created_at = time::OffsetDateTime::now_utc().unix_timestamp();

        sqlx::query(
            "INSERT INTO secret_client_keys (
                secret_key, secret_version, client_key_id,
                encrypted_data_key, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(secret_key)
        .bind(secret_version)
        .bind(client_key_id)
        .bind(encrypted_data_key)
        .bind(created_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn get_associations_for_secret(
        &self,
        pool: &SqlitePool,
        secret_key: &str,
        secret_version: i32,
    ) -> Result<Vec<SecretClientKeyAssociation>> {
        let associations: Vec<SecretClientKeyAssociation> = sqlx::query_as(
            "SELECT secret_key, secret_version, client_key_id, encrypted_data_key, created_at 
             FROM secret_client_keys 
             WHERE secret_key = ?1 AND secret_version = ?2",
        )
        .bind(secret_key)
        .bind(secret_version)
        .fetch_all(pool)
        .await?;

        Ok(associations)
    }

    async fn get_association(
        &self,
        pool: &SqlitePool,
        secret_key: &str,
        secret_version: i32,
        client_key_id: &Uuid,
    ) -> Result<Option<SecretClientKeyAssociation>> {
        let association: Option<SecretClientKeyAssociation> = sqlx::query_as(
            "SELECT secret_key, secret_version, client_key_id, encrypted_data_key, created_at 
             FROM secret_client_keys 
             WHERE secret_key = ?1 AND secret_version = ?2 AND client_key_id = ?3",
        )
        .bind(secret_key)
        .bind(secret_version)
        .bind(client_key_id)
        .fetch_optional(pool)
        .await?;

        Ok(association)
    }

    async fn remove_association(
        &self,
        pool: &SqlitePool,
        secret_key: &str,
        secret_version: i32,
        client_key_id: &Uuid,
    ) -> Result<()> {
        let result = sqlx::query(
            "DELETE FROM secret_client_keys 
             WHERE secret_key = ?1 AND secret_version = ?2 AND client_key_id = ?3",
        )
        .bind(secret_key)
        .bind(secret_version)
        .bind(client_key_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(SealboxError::ClientKeyNotFound(*client_key_id));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Should create in-memory DB");
        SqliteSecretClientKeyRepo::init_table(&pool)
            .await
            .expect("Should initialize tables");
        pool
    }

    #[tokio::test]
    async fn test_init_table() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        SqliteSecretClientKeyRepo::init_table(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn test_create_and_get_association() {
        let pool = setup_test_db().await;
        let repo = SqliteSecretClientKeyRepo;

        let secret_key = "test-secret";
        let secret_version = 1;
        let client_key_id = Uuid::new_v4();
        let encrypted_data_key = b"encrypted-data-key";

        repo.create_association(
            &pool,
            secret_key,
            secret_version,
            &client_key_id,
            encrypted_data_key,
        )
        .await
        .unwrap();

        let association = repo
            .get_association(&pool, secret_key, secret_version, &client_key_id)
            .await
            .unwrap();

        assert!(association.is_some());
        let association = association.unwrap();
        assert_eq!(association.secret_key, secret_key);
        assert_eq!(association.secret_version, secret_version);
        assert_eq!(association.client_key_id, client_key_id);
        assert_eq!(association.encrypted_data_key, encrypted_data_key);
    }
}
