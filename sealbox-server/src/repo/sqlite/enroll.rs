use sqlx::SqlitePool;

use crate::{
    error::Result,
    repo::{EnrollRepo, Enrollment},
};

#[derive(Debug, Clone)]
pub(crate) struct SqliteEnrollRepo;

#[async_trait::async_trait]
impl EnrollRepo for SqliteEnrollRepo {
    async fn init_table(pool: &SqlitePool) -> Result<()>
    where
        Self: Sized,
    {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS enrollments (
                code TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                name TEXT,
                description TEXT,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            )",
        )
        .execute(pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_enrollments_status ON enrollments(status)")
            .execute(pool)
            .await?;

        Ok(())
    }

    async fn create(&self, pool: &SqlitePool, code: &str, expires_at: i64) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        sqlx::query(
            "INSERT INTO enrollments (code, status, name, description, created_at, expires_at)
             VALUES (?1, 'Pending', NULL, NULL, ?2, ?3)",
        )
        .bind(code)
        .bind(now)
        .bind(expires_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn get(&self, pool: &SqlitePool, code: &str) -> Result<Option<Enrollment>> {
        // Lazy-expire
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let rec: Option<Enrollment> = sqlx::query_as(
            "SELECT code, status, name, description, created_at, expires_at FROM enrollments WHERE code = ?1",
        )
        .bind(code)
        .fetch_optional(pool)
        .await?;

        if let Some(mut r) = rec {
            if r.status == "Pending" && r.expires_at <= now {
                // Mark expired
                sqlx::query("UPDATE enrollments SET status = 'Expired' WHERE code = ?1")
                    .bind(code)
                    .execute(pool)
                    .await?;
                r.status = "Expired".to_string();
            }
            Ok(Some(r))
        } else {
            Ok(None)
        }
    }

    async fn approve(
        &self,
        pool: &SqlitePool,
        code: &str,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        // Ensure not expired
        let rec: Option<(i64, String)> =
            sqlx::query_as("SELECT expires_at, status FROM enrollments WHERE code = ?1")
                .bind(code)
                .fetch_optional(pool)
                .await?;

        match rec {
            None => return Ok(()),
            Some((expires_at, status)) => {
                if status == "Expired" || expires_at <= now {
                    // Expired, don't approve
                    return Ok(());
                }
            }
        }

        sqlx::query(
            "UPDATE enrollments SET status = 'Approved', name = ?1, description = ?2 WHERE code = ?3",
        )
        .bind(name)
        .bind(description)
        .bind(code)
        .execute(pool)
        .await?;
        Ok(())
    }
}
