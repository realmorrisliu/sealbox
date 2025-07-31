use std::sync::{Arc, Mutex};
use tracing::info;

use crate::{
    config::SealboxConfig,
    error::Result,
    repo::{
        MasterKeyRepo, SecretRepo, SqliteMasterKeyRepo, SqliteSecretRepo, create_db_connection,
    },
};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Arc<SealboxConfig>,
    pub(crate) conn_pool: Arc<Mutex<rusqlite::Connection>>,
    pub(crate) secret_repo: Arc<dyn SecretRepo>,
    pub(crate) master_key_repo: Arc<dyn MasterKeyRepo>,
}

impl AppState {
    pub fn new(config: &SealboxConfig) -> Result<Self> {
        let conn = create_db_connection(&config.store_path)?;

        SqliteSecretRepo::init_table(&conn)?;
        SqliteMasterKeyRepo::init_table(&conn)?;

        let state = Self {
            config: Arc::new(config.clone()),
            conn_pool: Arc::new(Mutex::new(conn)),
            secret_repo: Arc::new(SqliteSecretRepo {}),
            master_key_repo: Arc::new(SqliteMasterKeyRepo {}),
        };

        // Perform startup cleanup of expired secrets
        state.startup_cleanup()?;

        Ok(state)
    }

    /// Clean up expired secrets during application startup
    fn startup_cleanup(&self) -> Result<()> {
        info!("Performing startup cleanup of expired secrets...");
        let conn = self.conn_pool.lock()?;
        let deleted_count = self.secret_repo.cleanup_expired_secrets(&conn)?;
        if deleted_count > 0 {
            info!("Startup cleanup completed: removed {} expired secrets", deleted_count);
        } else {
            info!("Startup cleanup completed: no expired secrets found");
        }
        Ok(())
    }
}
