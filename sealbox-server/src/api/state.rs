use std::sync::{Arc, Mutex};
use tracing::info;

use crate::{
    config::SealboxConfig,
    error::Result,
    repo::{
        HealthRepo, MasterKeyRepo, SecretRepo, SqliteHealthRepo, SqliteMasterKeyRepo,
        SqliteSecretRepo, create_db_connection,
    },
};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Arc<SealboxConfig>,
    pub(crate) health_repo: Arc<dyn HealthRepo>,
    pub(crate) secret_repo: Arc<dyn SecretRepo>,
    pub(crate) master_key_repo: Arc<dyn MasterKeyRepo>,
}

impl AppState {
    pub fn new(config: &SealboxConfig) -> Result<Self> {
        let conn = create_db_connection(&config.store_path)?;

        SqliteSecretRepo::init_table(&conn)?;
        SqliteMasterKeyRepo::init_table(&conn)?;

        // One connection, shared by the repositories. Nothing above this layer holds it:
        // a database lock has no business in an HTTP handler.
        let conn = Arc::new(Mutex::new(conn));

        let state = Self {
            config: Arc::new(config.clone()),
            health_repo: Arc::new(SqliteHealthRepo::new(conn.clone())),
            secret_repo: Arc::new(SqliteSecretRepo::new(conn.clone())),
            master_key_repo: Arc::new(SqliteMasterKeyRepo::new(conn)),
        };

        // Perform startup cleanup of expired secrets
        state.startup_cleanup()?;

        Ok(state)
    }

    /// Clean up expired secrets during application startup
    fn startup_cleanup(&self) -> Result<()> {
        info!("Performing startup cleanup of expired secrets...");
        let deleted_count = self.secret_repo.cleanup_expired_secrets()?;
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
