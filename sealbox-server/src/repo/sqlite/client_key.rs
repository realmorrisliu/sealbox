use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::{Result, SealboxError},
    repo::{ClientKey, ClientKeyRepo},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteClientKeyRepo;

#[async_trait::async_trait]
impl ClientKeyRepo for SqliteClientKeyRepo {
    async fn init_table(pool: &SqlitePool) -> Result<()>
    where
        Self: Sized,
    {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS client_keys (
                id BLOB PRIMARY KEY,
                public_key TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL,
                description TEXT,
                version INTEGER,
                metadata TEXT,
                name TEXT,
                last_used_at INTEGER
            )",
        )
        .execute(pool)
        .await?;

        // Create indexes for performance optimization
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_client_keys_status 
             ON client_keys(status)",
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_client_keys_created_at 
             ON client_keys(created_at)",
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_client_keys_last_used_at 
             ON client_keys(last_used_at)",
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn create_client_key(&self, pool: &SqlitePool, key: &ClientKey) -> Result<()> {
        sqlx::query(
            "INSERT INTO client_keys (
                id, public_key, created_at, status,
                description, metadata, name, last_used_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(key.id)
        .bind(&key.public_key)
        .bind(key.created_at)
        .bind(&key.status)
        .bind(&key.description)
        .bind(&key.metadata)
        .bind(&key.name)
        .bind(key.last_used_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn fetch_client_key(
        &self,
        pool: &SqlitePool,
        client_key_id: &Uuid,
    ) -> Result<Option<ClientKey>> {
        let client_key: Option<ClientKey> = sqlx::query_as(
            "SELECT id, public_key, created_at, status, description, metadata, name, last_used_at 
            FROM client_keys WHERE id = ?1 LIMIT 1",
        )
        .bind(client_key_id)
        .fetch_optional(pool)
        .await?;

        Ok(client_key)
    }

    async fn fetch_all_client_keys(&self, pool: &SqlitePool) -> Result<Vec<ClientKey>> {
        let client_keys: Vec<ClientKey> = sqlx::query_as(
            "SELECT id, public_key, created_at, status, description, metadata, name, last_used_at 
            FROM client_keys ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(client_keys)
    }

    async fn fetch_public_key(
        &self,
        pool: &SqlitePool,
        client_key_id: &Uuid,
    ) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT public_key FROM client_keys WHERE id = ?1 LIMIT 1")
                .bind(client_key_id)
                .fetch_optional(pool)
                .await?;

        Ok(row.map(|(public_key,)| public_key))
    }

    async fn get_valid_client_key(&self, pool: &SqlitePool) -> Result<ClientKey> {
        let client_key: Option<ClientKey> = sqlx::query_as(
            "SELECT id, public_key, created_at, status, description, metadata, name, last_used_at 
            FROM client_keys WHERE status = 'Active' ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_optional(pool)
        .await?;

        client_key.ok_or(SealboxError::NoValidClientKey)
    }

    async fn update_last_used(&self, pool: &SqlitePool, client_key_id: &Uuid) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        sqlx::query("UPDATE client_keys SET last_used_at = ?1 WHERE id = ?2")
            .bind(now)
            .bind(client_key_id)
            .execute(pool)
            .await?;

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
        SqliteClientKeyRepo::init_table(&pool)
            .await
            .expect("Should initialize tables");
        pool
    }

    #[tokio::test]
    async fn test_init_table() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        SqliteClientKeyRepo::init_table(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn test_create_and_fetch_client_key() {
        let pool = setup_test_db().await;
        let repo = SqliteClientKeyRepo;

        let client_key = ClientKey::new("test-public-key".to_string()).unwrap();
        repo.create_client_key(&pool, &client_key).await.unwrap();

        let fetched_key = repo.fetch_client_key(&pool, &client_key.id).await.unwrap();
        assert!(fetched_key.is_some());
        assert_eq!(fetched_key.unwrap().id, client_key.id);
    }
}
