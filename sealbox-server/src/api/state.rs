use std::sync::{Arc, Mutex};
use tracing::info;

use crate::{
    config::SealboxConfig,
    error::Result,
    repo::{
        HealthRepo, MasterKeyRepo, SecretRepo, SqliteHealthRepo, SqliteMasterKeyRepo,
        SqliteSecretRepo, SqliteTenantRepo, TenantRepo, backup_before_migration,
        create_db_connection, run_migrations,
    },
};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Arc<SealboxConfig>,
    pub(crate) conn_pool: Arc<Mutex<rusqlite::Connection>>,
    pub(crate) health_repo: Arc<dyn HealthRepo>,
    pub(crate) secret_repo: Arc<dyn SecretRepo>,
    pub(crate) master_key_repo: Arc<dyn MasterKeyRepo>,
    pub(crate) tenant_repo: Arc<dyn TenantRepo>,
}

impl AppState {
    pub fn new(config: &SealboxConfig) -> Result<Self> {
        let conn = create_db_connection(&config.store_path)?;

        if let Some(backup_path) = backup_before_migration(&conn, &config.store_path)? {
            info!(
                backup_path = %backup_path.display(),
                "preserved pre-tenant Sealbox database backup"
            );
        }
        run_migrations(&conn)?;

        let state = Self {
            config: Arc::new(config.clone()),
            conn_pool: Arc::new(Mutex::new(conn)),
            health_repo: Arc::new(SqliteHealthRepo {}),
            secret_repo: Arc::new(SqliteSecretRepo {}),
            master_key_repo: Arc::new(SqliteMasterKeyRepo {}),
            tenant_repo: Arc::new(SqliteTenantRepo {}),
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
