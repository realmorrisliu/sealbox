use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;
use uuid::Uuid;

use crate::{
    api::{SealboxResponse, Version, path::Path, state::AppState},
    error::{Result, SealboxError},
    repo::MasterKey,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct MasterKeyPathParams {
    version: Version,
}

impl MasterKeyPathParams {
    fn version(&self) -> Version {
        self.version.clone()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct RotateMasterKeyPayload {
    new_master_key_id: Uuid,
    old_master_key_id: Uuid,
    old_private_key_pem: String,
}

// GET /{version}/master-key
pub(crate) async fn list(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let master_keys = state.master_key_repo.fetch_all_master_keys(&conn)?;
            Ok(SealboxResponse::Json(json!({"master_keys": master_keys})))
        }
        _ => Err(SealboxError::InvalidVersion),
    }
}

// PUT /{version}/master-key
pub(crate) async fn rotate(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyPathParams>,
    Json(payload): Json<RotateMasterKeyPayload>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let new_master_key_id = payload.new_master_key_id;
            let old_master_key_id = payload.old_master_key_id;
            let old_private_key_pem = payload.old_private_key_pem;

            let mut conn = state.conn_pool.lock()?;

            let new_public_key_pem = state
                .master_key_repo
                .fetch_public_key(&conn, &new_master_key_id)?
                .ok_or_else(|| SealboxError::MasterKeyNotFound(new_master_key_id.clone()))?;

            let secrets = state
                .secret_repo
                .fetch_secrets_by_master_key(&conn, &old_master_key_id)?;

            let mut failed_secret_keys = Vec::new();

            let tx = conn.transaction()?;

            for secret in secrets {
                let secret_key = secret.key.clone();

                match secret.rotate_master_key(
                    &old_master_key_id,
                    &old_private_key_pem,
                    &new_master_key_id,
                    &new_public_key_pem,
                ) {
                    Ok(rotated_secret) => {
                        state
                            .secret_repo
                            .update_secret_master_key(&tx, &rotated_secret)?;
                    }
                    Err(err) => {
                        failed_secret_keys.push(secret_key.clone());
                        error!(
                            "Failed to rotate master key for secret {}: {}",
                            secret_key, err
                        );
                    }
                }
            }

            tx.commit()?;

            if !failed_secret_keys.is_empty() {
                return Ok(SealboxResponse::Json(json!({
                  "master_key": new_master_key_id,
                  "failed_secret_keys": failed_secret_keys
                })));
            }

            Ok(SealboxResponse::Json(
                json!({ "master_key": new_master_key_id }),
            ))
        }
        _ => Err(SealboxError::InvalidVersion),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CreateMasterKeyPayload {
    public_key: String,
}

// POST /{version}/master-key
pub(crate) async fn create(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyPathParams>,
    Json(payload): Json<CreateMasterKeyPayload>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let master_key = MasterKey::new(payload.public_key)?;
            state
                .master_key_repo
                .create_master_key(&conn, &master_key)?;
            Ok(SealboxResponse::Json(json!({ "master_key": master_key })))
        }
        _ => Err(SealboxError::InvalidVersion),
    }
}
