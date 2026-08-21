use std::sync::{Arc, Mutex};
use tracing::info;

use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

use crate::{
    config::SealboxConfig,
    crypto::master_key::PrivateMasterKey,
    error::{Result, SealboxError},
    repo::{
        HealthRepo, MasterKeyRepo, MasterKeyStatus, SecretRepo, SqliteHealthRepo,
        SqliteMasterKeyRepo, SqliteSecretRepo, create_db_connection,
    },
};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Arc<SealboxConfig>,
    pub(crate) health_repo: Arc<dyn HealthRepo>,
    pub(crate) secret_repo: Arc<dyn SecretRepo>,
    pub(crate) master_key_repo: Arc<dyn MasterKeyRepo>,
    /// Private halves of the server's own master keys, by id. Only rekey needs these; a key
    /// absent from this map is cold, and nothing can decrypt secrets under it.
    pub(crate) server_keys: Arc<HashMap<Uuid, PrivateMasterKey>>,
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
            server_keys: Arc::new(HashMap::new()),
        };

        let server_keys = state.load_server_master_keys()?;
        let state = Self {
            server_keys: Arc::new(server_keys),
            ..state
        };

        // Perform startup cleanup of expired secrets
        state.startup_cleanup()?;

        Ok(state)
    }

    /// Load the server's master keys and make sure each is registered.
    ///
    /// The first is Active — new secrets are encrypted under it. Any others are Retired: still
    /// loaded, so the secrets already under them stay readable and can be rekeyed onto the
    /// current key. Remove a path from the list once nothing references it.
    ///
    /// A missing or unreadable file is fatal rather than a cue to generate one: silently
    /// creating a key when the path is wrong would leave every existing secret cold, encrypted
    /// under a key nobody has, and the failure would only surface later on a read.
    fn load_server_master_keys(&self) -> Result<HashMap<Uuid, PrivateMasterKey>> {
        let mut loaded = HashMap::new();

        for (index, path) in self.config.master_key_paths.iter().enumerate() {
            let pem = std::fs::read_to_string(path).map_err(|e| {
                SealboxError::ConfigError(format!(
                    "Cannot read the server master key at {path}: {e}. Generate one and point \
                     SEALBOX_MASTER_KEY_PATH at it; sealbox will not create it for you, because \
                     doing so on a mistyped path would silently make every stored secret cold."
                ))
            })?;

            let private_key = PrivateMasterKey::from_str(&pem)?;
            let public_pem = private_key.public_key_pem()?;
            let status = if index == 0 {
                MasterKeyStatus::Active
            } else {
                MasterKeyStatus::Retired
            };
            let key = self
                .master_key_repo
                .ensure_server_held(&public_pem, status)?;
            info!(
                "Server master key {} loaded from {} ({})",
                key.id,
                path,
                if index == 0 { "current" } else { "retired" }
            );
            loaded.insert(key.id, private_key);
        }

        Ok(loaded)
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
