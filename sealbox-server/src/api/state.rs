use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::info;

use crate::{
    config::SealboxConfig,
    error::Result,
    repo::{
        ClientKeyRepo, EnrollRepo, HealthRepo, SecretClientKeyRepo, SecretRepo,
        SqliteClientKeyRepo, SqliteHealthRepo, SqliteSecretClientKeyRepo, SqliteSecretRepo,
        create_db_pool,
    },
};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Arc<SealboxConfig>,
    pub(crate) pool: SqlitePool,
    pub(crate) health_repo: Arc<dyn HealthRepo>,
    pub(crate) secret_repo: Arc<dyn SecretRepo>,
    pub(crate) client_key_repo: Arc<dyn ClientKeyRepo>,
    pub(crate) secret_client_key_repo: Arc<dyn SecretClientKeyRepo>,
    pub(crate) enroll_repo: Arc<dyn crate::repo::EnrollRepo>,
}

impl AppState {
    pub async fn new(config: &SealboxConfig) -> Result<Self> {
        let pool = create_db_pool(&config.store_path).await?;

        SqliteSecretRepo::init_table(&pool).await?;
        SqliteClientKeyRepo::init_table(&pool).await?;
        SqliteSecretClientKeyRepo::init_table(&pool).await?;
        crate::repo::SqliteEnrollRepo::init_table(&pool).await?;

        let state = Self {
            config: Arc::new(config.clone()),
            pool,
            health_repo: Arc::new(SqliteHealthRepo {}),
            secret_repo: Arc::new(SqliteSecretRepo {}),
            client_key_repo: Arc::new(SqliteClientKeyRepo {}),
            secret_client_key_repo: Arc::new(SqliteSecretClientKeyRepo {}),
            enroll_repo: Arc::new(crate::repo::SqliteEnrollRepo {}),
        };

        // Perform startup cleanup of expired secrets
        state.startup_cleanup().await?;

        Ok(state)
    }

    /// Clean up expired secrets during application startup
    async fn startup_cleanup(&self) -> Result<()> {
        info!("Performing startup cleanup of expired secrets...");
        let deleted_count = self.secret_repo.cleanup_expired_secrets(&self.pool).await?;
        if deleted_count > 0 {
            info!(
                "Startup cleanup completed: removed {} expired secrets",
                deleted_count
            );
        } else {
            info!("Startup cleanup completed: no expired secrets found");
        }
        Ok(())
    }
}
