use axum::extract::{Json, Query, State};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    api::{SealboxResponse, Version, path::Path, state::AppState},
    error::{Result, SealboxError},
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

#[derive(Debug, Deserialize)]
pub(crate) struct GetSecretQueryParams {
    version: Option<i32>,
}

// GET /{version}/secrets/{secret_key}
pub(crate) async fn get(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Query(query): Query<GetSecretQueryParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let secret = match query.version {
                Some(version) => {
                    state
                        .secret_repo
                        .get_secret_by_version(&conn, &params.secret_key(), version)?
                }
                None => state.secret_repo.get_secret(&conn, &params.secret_key())?,
            };
            Ok(SealboxResponse::Json(json!({"secret": secret})))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct SaveSecretPayload {
    secret: String,
    ttl: Option<i64>,
}

// PUT /{version}/secrets/{secret_key}
pub(crate) async fn save(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Json(payload): Json<SaveSecretPayload>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let mut conn = state.conn_pool.lock()?;
            let master_key = state
                .master_key_repo
                .get_valid_master_key(&conn)?
                .ok_or_else(|| SealboxError::NotInitialized)?;

            let secret = state.secret_repo.create_new_version(
                &mut conn,
                &params.secret_key(),
                &payload.secret,
                master_key,
                payload.ttl,
            )?;

            Ok(SealboxResponse::Json(json!(secret)))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteSecretQueryParams {
    version: i32,
}

// DELETE /{version}/secrets/{secret_key}
pub(crate) async fn delete(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Query(query): Query<DeleteSecretQueryParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            state.secret_repo.delete_secret_by_version(
                &conn,
                &params.secret_key(),
                query.version,
            )?;
            Ok(SealboxResponse::Ok)
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}
