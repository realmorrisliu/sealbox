use std::sync::{Arc, Mutex};

use crate::{
    config::SealboxConfig,
    error::Result,
    repo::{
        MasterKeyRepo, SecretRepo, SqliteMasterKeyRepo, SqliteSecretRepo, create_db_connection,
    },
};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) secret_repo: Arc<dyn SecretRepo>,
    pub(crate) master_key_repo: Arc<dyn MasterKeyRepo>,
}

impl AppState {
    pub fn new(config: &SealboxConfig) -> Result<Self> {
        let conn = Arc::new(Mutex::new(create_db_connection(&config.store_path)?));

        let secret_repo = SqliteSecretRepo::new(conn.clone())?;
        let master_key_repo = SqliteMasterKeyRepo::new(conn.clone())?;

        Ok(Self {
            secret_repo: Arc::new(secret_repo.clone()),
            master_key_repo: Arc::new(master_key_repo.clone()),
        })
    }
}
