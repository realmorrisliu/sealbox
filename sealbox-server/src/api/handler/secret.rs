use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    api::{SealboxResponse, Version, path::Path, state::AppState},
    error::{Result, SealboxError},
    repo::Secret,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct SecretPathParams {
    version: Version,
    secret_key: String,
}

impl SecretPathParams {
    fn version(&self) -> Version {
        self.version.clone()
    }
    fn secret_key(&self) -> String {
        self.secret_key.clone()
    }
}

// GET /{version}/secrets/{secret_key}
pub(crate) async fn get(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let secret = state.secret_repo.get_secret(&conn, &params.secret_key())?;
            Ok(SealboxResponse::Json(json!({"secret": secret})))
        }
        _ => Err(SealboxError::InvalidVersion),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct SaveSecretPayload {
    secret: String,
}

// PUT /{version}/secrets/{secret_key}
pub(crate) async fn save(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Json(payload): Json<SaveSecretPayload>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let master_key = state
                .master_key_repo
                .get_valid_master_key(&conn)?
                .ok_or_else(|| SealboxError::NotInitialized)?;

            let secret = Secret::new(&params.secret_key(), &payload.secret, master_key)?;
            state.secret_repo.save_secret(&conn, &secret)?;

            Ok(SealboxResponse::Ok)
        }
        _ => Err(SealboxError::InvalidVersion),
    }
}

// DELETE /{version}/secrets/{secret_key}
pub(crate) async fn delete(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            state
                .secret_repo
                .delete_secret(&conn, &params.secret_key())?;
            Ok(SealboxResponse::Ok)
        }
        _ => Err(SealboxError::InvalidVersion),
    }
}
