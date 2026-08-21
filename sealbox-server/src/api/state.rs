use std::sync::{Arc, Mutex};
use tracing::info;

use std::str::FromStr;

use crate::{
    config::SealboxConfig,
    crypto::master_key::PrivateMasterKey,
    error::{Result, SealboxError},
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

        state.register_server_master_key()?;

        // Perform startup cleanup of expired secrets
        state.startup_cleanup()?;

        Ok(state)
    }

    /// Load the server's own master key and make sure it is registered.
    ///
    /// A missing or unreadable file is fatal rather than a cue to generate one: silently
    /// creating a key when the path is wrong would leave every existing secret cold, encrypted
    /// under a key nobody has, and the failure would only surface later on a read.
    fn register_server_master_key(&self) -> Result<()> {
        let path = &self.config.master_key_path;
        let pem = std::fs::read_to_string(path).map_err(|e| {
            SealboxError::ConfigError(format!(
                "Cannot read the server master key at {path}: {e}. Generate one and point \
                 SEALBOX_MASTER_KEY_PATH at it; sealbox will not create it for you, because \
                 doing so on a mistyped path would silently make every stored secret cold."
            ))
        })?;

        let private_key = PrivateMasterKey::from_str(&pem)?;
        let public_pem = private_key.public_key_pem()?;
        let key = self.master_key_repo.ensure_server_held(&public_pem)?;
        info!("Server master key ready: {}", key.id);
        Ok(())
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
