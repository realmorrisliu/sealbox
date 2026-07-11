use axum::{
    Extension,
    extract::{Json, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    api::{SealboxResponse, Version, auth::TenantPrincipal, path::Path, state::AppState},
    error::{Result, SealboxError},
    repo::{EncryptedSecretInput, LEGACY_TENANT_ID},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct TenantSecretPathParams {
    secret_key: String,
}

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

/// API handler function for retrieving secret data
///
/// # Arguments
///
/// * `state` - Application state containing database connection pool and repository instances
/// * `params` - Path parameters containing API version and secret key name
/// * `query` - Query parameters with optional version number for retrieving specific version
///
/// # Returns
///
/// Returns encrypted secret data containing encrypted content and encrypted data key
///
/// # Errors
///
/// * `SealboxError::SecretNotFound` - When the secret does not exist
/// * `SealboxError::InvalidApiVersion` - When the API version is not supported
///
/// # HTTP Route
///
/// `GET /{version}/secrets/{secret_key}[?version=N]`
///
/// # Security Notes
///
/// If no version number is specified, returns the latest version. The returned data is still encrypted and requires the client to decrypt it using the corresponding private key.
pub(crate) async fn get(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Query(query): Query<GetSecretQueryParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let mut conn = state.conn_pool.lock()?;

            let secret = match query.version {
                Some(version) => state.secret_repo.get_secret_by_version(
                    &mut conn,
                    LEGACY_TENANT_ID,
                    &params.secret_key(),
                    version,
                )?,
                None => state.secret_repo.get_secret(
                    &mut conn,
                    LEGACY_TENANT_ID,
                    &params.secret_key(),
                )?,
            };

            Ok(SealboxResponse::Json(json!(secret)))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct SaveSecretPayload {
    encrypted_data: Vec<u8>,
    encrypted_data_key: Vec<u8>,
    master_key_id: Uuid,
    ttl: Option<i64>,
    metadata: Option<String>,
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
                .get_valid_master_key(&conn, LEGACY_TENANT_ID)?;

            if payload.master_key_id != master_key.id {
                return Err(SealboxError::InvalidRequest(format!(
                    "payload master_key_id {} is not the active master key",
                    payload.master_key_id
                )));
            }

            let secret = state.secret_repo.create_new_encrypted_version(
                &mut conn,
                LEGACY_TENANT_ID,
                &params.secret_key(),
                EncryptedSecretInput {
                    encrypted_data: payload.encrypted_data,
                    encrypted_data_key: payload.encrypted_data_key,
                    master_key_id: payload.master_key_id,
                    ttl: payload.ttl,
                    metadata: payload.metadata,
                },
            )?;

            Ok(SealboxResponse::Json(json!(secret)))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteSecretQueryParams {
    version: Option<i32>,
}

// DELETE /{version}/secrets/{secret_key}[?version=N]
pub(crate) async fn delete(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Query(query): Query<DeleteSecretQueryParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            match query.version {
                Some(version) => {
                    state.secret_repo.delete_secret_by_version(
                        &conn,
                        LEGACY_TENANT_ID,
                        &params.secret_key(),
                        version,
                    )?;
                }
                None => {
                    state.secret_repo.delete_secret(
                        &conn,
                        LEGACY_TENANT_ID,
                        &params.secret_key(),
                    )?;
                }
            }
            Ok(SealboxResponse::Ok)
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ListSecretsPathParams {
    version: Version,
}

impl ListSecretsPathParams {
    fn version(&self) -> Version {
        self.version.clone()
    }
}

/// API handler function for listing all secrets
///
/// # Arguments
///
/// * `state` - Application state containing database connection pool and repository instances
/// * `params` - Path parameters containing API version
///
/// # Returns
///
/// Returns a list of secrets with basic information (key, version, timestamps)
///
/// # Errors
///
/// * `SealboxError::InvalidApiVersion` - When the API version is not supported
///
/// # HTTP Route
///
/// `GET /{version}/secrets`
///
/// # Security Notes
///
/// Returns only metadata about secrets, not the encrypted content. Automatically filters out expired secrets.
pub(crate) async fn list(
    State(state): State<AppState>,
    Path(params): Path<ListSecretsPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let secrets = state.secret_repo.list_secrets(&conn, LEGACY_TENANT_ID)?;
            Ok(SealboxResponse::Json(json!({ "secrets": secrets })))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

/// API handler function for listing retained versions for one secret key
///
/// Returns metadata only. Encrypted data and encrypted data keys are not included.
///
/// # HTTP Route
///
/// `GET /{version}/secrets/{secret_key}/history`
pub(crate) async fn history(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let versions = state.secret_repo.list_secret_versions(
                &conn,
                LEGACY_TENANT_ID,
                &params.secret_key(),
            )?;
            Ok(SealboxResponse::Json(json!({ "versions": versions })))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

pub(crate) async fn get_v2(
    State(state): State<AppState>,
    Extension(principal): Extension<TenantPrincipal>,
    Path(params): Path<TenantSecretPathParams>,
    Query(query): Query<GetSecretQueryParams>,
) -> Result<SealboxResponse> {
    tracing::debug!(
        tenant_id = principal.tenant_id,
        token_id = %principal.token_id,
        key = params.secret_key,
        "retrieving tenant secret"
    );
    let mut conn = state.conn_pool.lock()?;
    let secret = match query.version {
        Some(version) => state.secret_repo.get_secret_by_version(
            &mut conn,
            &principal.tenant_id,
            &params.secret_key,
            version,
        )?,
        None => {
            state
                .secret_repo
                .get_secret(&mut conn, &principal.tenant_id, &params.secret_key)?
        }
    };
    Ok(SealboxResponse::Json(json!(secret)))
}

pub(crate) async fn save_v2(
    State(state): State<AppState>,
    Extension(principal): Extension<TenantPrincipal>,
    Path(params): Path<TenantSecretPathParams>,
    Json(payload): Json<SaveSecretPayload>,
) -> Result<SealboxResponse> {
    let mut conn = state.conn_pool.lock()?;
    let master_key = state
        .master_key_repo
        .get_valid_master_key(&conn, &principal.tenant_id)?;
    if payload.master_key_id != master_key.id {
        return Err(SealboxError::InvalidRequest(format!(
            "payload master_key_id {} is not the active master key for this tenant",
            payload.master_key_id
        )));
    }
    let secret = state.secret_repo.create_new_encrypted_version(
        &mut conn,
        &principal.tenant_id,
        &params.secret_key,
        EncryptedSecretInput {
            encrypted_data: payload.encrypted_data,
            encrypted_data_key: payload.encrypted_data_key,
            master_key_id: payload.master_key_id,
            ttl: payload.ttl,
            metadata: payload.metadata,
        },
    )?;
    Ok(SealboxResponse::Json(json!(secret)))
}

pub(crate) async fn delete_v2(
    State(state): State<AppState>,
    Extension(principal): Extension<TenantPrincipal>,
    Path(params): Path<TenantSecretPathParams>,
    Query(query): Query<DeleteSecretQueryParams>,
) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    match query.version {
        Some(version) => state.secret_repo.delete_secret_by_version(
            &conn,
            &principal.tenant_id,
            &params.secret_key,
            version,
        )?,
        None => state
            .secret_repo
            .delete_secret(&conn, &principal.tenant_id, &params.secret_key)?,
    }
    Ok(SealboxResponse::Ok)
}

pub(crate) async fn list_v2(
    State(state): State<AppState>,
    Extension(principal): Extension<TenantPrincipal>,
) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    let secrets = state
        .secret_repo
        .list_secrets(&conn, &principal.tenant_id)?;
    Ok(SealboxResponse::Json(json!({ "secrets": secrets })))
}

pub(crate) async fn history_v2(
    State(state): State<AppState>,
    Extension(principal): Extension<TenantPrincipal>,
    Path(params): Path<TenantSecretPathParams>,
) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    let versions =
        state
            .secret_repo
            .list_secret_versions(&conn, &principal.tenant_id, &params.secret_key)?;
    Ok(SealboxResponse::Json(json!({
        "key": params.secret_key,
        "versions": versions,
    })))
}
