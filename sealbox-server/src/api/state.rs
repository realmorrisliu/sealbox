use std::sync::Arc;

use crate::{
    config::SealboxConfig,
    error::Result,
    repo::{SecretRepo, sqlite::SqliteSecretRepo},
};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) secret_repo: Arc<dyn SecretRepo>,
}

impl AppState {
    pub fn new(config: &SealboxConfig) -> Result<Self> {
        let secret_repo = SqliteSecretRepo::new(&config.store_path)?;
        Ok(Self {
            secret_repo: Arc::new(secret_repo.clone()),
        })
    }
}
