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
    pub(crate) conn_pool: Arc<Mutex<rusqlite::Connection>>,
    pub(crate) secret_repo: Arc<dyn SecretRepo>,
    pub(crate) master_key_repo: Arc<dyn MasterKeyRepo>,
}

impl AppState {
    pub fn new(config: &SealboxConfig) -> Result<Self> {
        let conn = create_db_connection(&config.store_path)?;

        SqliteSecretRepo::init_table(&conn)?;
        SqliteMasterKeyRepo::init_table(&conn)?;

        Ok(Self {
            conn_pool: Arc::new(Mutex::new(conn)),
            secret_repo: Arc::new(SqliteSecretRepo {}),
            master_key_repo: Arc::new(SqliteMasterKeyRepo {}),
        })
    }
}
