use sqlx::SqlitePool;

use crate::{error::Result, repo::HealthRepo};

#[derive(Debug, Clone)]
pub(crate) struct SqliteHealthRepo;

#[async_trait::async_trait]
impl HealthRepo for SqliteHealthRepo {
    async fn check_health(&self, pool: &SqlitePool) -> Result<bool> {
        // Simple health check by running a basic query
        let _result: (i32,) = sqlx::query_as("SELECT 1").fetch_one(pool).await?;

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let repo = SqliteHealthRepo;

        let result = repo.check_health(&pool).await.unwrap();
        assert!(result);
    }
}
