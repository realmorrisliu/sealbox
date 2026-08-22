use axum::extract::{Json, Query, State};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    api::{SealboxResponse, path::Path, state::AppState},
    error::{Result, SealboxError},
    repo::{GenerateSpec, SecretValue},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct SecretPathParams {
    secret_key: String,
}

impl SecretPathParams {
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
/// * `params` - Path parameters containing the secret key name
/// * `query` - Query parameters with optional version number for retrieving specific version
///
/// # Returns
///
/// Returns encrypted secret data containing encrypted content and encrypted data key
///
/// # Errors
///
/// * `SealboxError::SecretNotFound` - When the secret does not exist
///
/// # HTTP Route
///
/// `GET /v1/secrets/{secret_key}[?version=N]`
///
/// # Security Notes
///
/// If no version number is specified, returns the latest version. The returned data is still encrypted and requires the client to decrypt it using the corresponding private key.
pub(crate) async fn get(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Query(query): Query<GetSecretQueryParams>,
) -> Result<SealboxResponse> {
    let secret = match query.version {
        Some(version) => state
            .secret_repo
            .get_secret_by_version(&params.secret_key(), version)?,
        None => state.secret_repo.get_secret(&params.secret_key())?,
    };

    Ok(SealboxResponse::Json(json!(secret)))
}

/// Either supply a value or ask for one to be generated — never both, and never neither.
/// Unknown fields are rejected so a typo in `generate` does not silently store nothing.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaveSecretPayload {
    secret: Option<String>,
    generate: Option<GenerateSpec>,
    ttl: Option<i64>,
}

impl SaveSecretPayload {
    fn value(self) -> Result<SecretValue> {
        match (self.secret, self.generate) {
            (Some(_), Some(_)) => Err(SealboxError::InvalidRequest(
                "supply either `secret` or `generate`, not both".to_string(),
            )),
            (None, None) => Err(SealboxError::InvalidRequest(
                "supply either `secret` or `generate`".to_string(),
            )),
            (Some(secret), None) => Ok(SecretValue::Supplied(secret)),
            (None, Some(spec)) => Ok(SecretValue::Generated(spec)),
        }
    }
}

// PUT /v1/secrets/{secret_key}
pub(crate) async fn save(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Json(payload): Json<SaveSecretPayload>,
) -> Result<SealboxResponse> {
    let master_key = state.master_key_repo.get_valid_master_key()?;

    let ttl = payload.ttl;
    let value = payload.value()?;

    let secret =
        state
            .secret_repo
            .create_new_version(&params.secret_key(), &value, master_key, ttl)?;

    // Metadata only. Returning the ciphertext and the encrypted data key would hand every caller
    // the material to decrypt with, given a master key — for no reason: the caller asked to store
    // a value, and the answer is which version it became.
    Ok(SealboxResponse::Json(json!({
        "key": secret.key,
        "version": secret.version,
        "created_at": secret.created_at,
        "expires_at": secret.expires_at,
    })))
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteSecretQueryParams {
    version: i32,
}

// DELETE /v1/secrets/{secret_key}
pub(crate) async fn delete(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Query(query): Query<DeleteSecretQueryParams>,
) -> Result<SealboxResponse> {
    state
        .secret_repo
        .delete_secret_by_version(&params.secret_key(), query.version)?;
    Ok(SealboxResponse::Ok)
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListSecretsQueryParams {
    /// When set, return the grants that may use this secret instead of the secret list.
    uses: Option<String>,
}

/// API handler function for listing all secrets
///
/// # Arguments
///
/// * `state` - Application state containing database connection pool and repository instances
///
/// # Returns
///
/// Returns a list of secrets with basic information (key, version, timestamps)
///
/// # Errors
///
///
/// # HTTP Route
///
/// `GET /v1/secrets`
///
/// # Security Notes
///
/// Returns only metadata about secrets, not the encrypted content. Automatically filters out expired secrets.
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListSecretsQueryParams>,
) -> Result<SealboxResponse> {
    // `?uses=` answers the question no other secret manager can: everything this credential can
    // do here. A filter over grants rather than a maintained reverse index — there will be tens
    // of grants, and an index that disagreed with the grants themselves would be worse than a
    // scan for an answer people act on.
    if let Some(secret) = query.uses {
        let grants: Vec<String> = state
            .grant_repo
            .list()?
            .into_iter()
            .filter(|g| g.declares(&secret))
            .map(|g| g.name)
            .collect();
        return Ok(SealboxResponse::Json(
            json!({ "secret": secret, "used_by": grants }),
        ));
    }

    let secrets = state.secret_repo.list_secrets()?;
    Ok(SealboxResponse::Json(json!({ "secrets": secrets })))
}
