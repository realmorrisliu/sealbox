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

/// GET /v1/secrets/{secret_key}[?version=N] — metadata, never ciphertext.
///
/// The stored row holds `encrypted_data` and `encrypted_data_key`, and this handler used to
/// return both. Nothing needs them over HTTP: the runner receives plaintext (the server decrypts
/// before dispatch) and rekey happens server-side. What returning them cost was that any agent
/// could carry away ciphertext for every secret and keep it against the day a master key leaks.
///
/// There is deliberately no parameter and no role that produces it, because a way to get it is a
/// way for something to be misconfigured into getting it. A **cold** secret — one under a master
/// key the server does not hold — is read offline, from a copy of the database and the key, with
/// no server involved. That is also the only thing that works at the moment a cold secret is
/// actually wanted, which is when the server is gone.
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

    Ok(SealboxResponse::Json(json!({
        "key": secret.key,
        "version": secret.version,
        "master_key_id": secret.master_key_id,
        "created_at": secret.created_at,
        "updated_at": secret.updated_at,
        "expires_at": secret.expires_at,
        "metadata": secret.metadata,
        "rotate_after": secret.rotate_after,
        "rotate_due_at": secret.rotate_after.map(|after| secret.updated_at + after),
    })))
}

/// Either supply a value or ask for one to be generated — never both, and never neither.
/// Unknown fields are rejected so a typo in `generate` does not silently store nothing.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaveSecretPayload {
    secret: Option<String>,
    generate: Option<GenerateSpec>,
    ttl: Option<i64>,
    /// Seconds this value should stand before it is rotated again. Recorded, never acted on.
    rotate_after: Option<i64>,
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
    let rotate_after = payload.rotate_after;
    let value = payload.value()?;

    let secret = state.secret_repo.create_new_version(
        &params.secret_key(),
        &value,
        master_key,
        ttl,
        rotate_after,
        false,
    )?;

    // Metadata only. Returning the ciphertext and the encrypted data key would hand every caller
    // the material to decrypt with, given a master key — for no reason: the caller asked to store
    // a value, and the answer is which version it became.
    Ok(SealboxResponse::Json(json!({
        "key": secret.key,
        "version": secret.version,
        "created_at": secret.created_at,
        "expires_at": secret.expires_at,
        "rotate_after": secret.rotate_after,
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
    /// Only secrets past their declared rotation interval.
    overdue: Option<bool>,
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

    let mut secrets = state.secret_repo.list_secrets()?;

    // Filtered at read time from what is already there, so nothing has to be swept and nothing
    // can disagree: a secret stops being overdue by being rotated, because the only thing making
    // it overdue is the timestamp a rotation moves.
    if query.overdue.unwrap_or(false) {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        secrets.retain(|s| s.rotate_due_at.is_some_and(|due| due < now));
    }

    Ok(SealboxResponse::Json(json!({ "secrets": secrets })))
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RotatePayload {
    /// The grant that will make some upstream accept the new value.
    via: String,
    /// Store what the grant printed instead of what was generated — for values that are composed
    /// (a URL with a percent-encoded password) or issued upstream.
    #[serde(default)]
    from_output: bool,
    #[serde(default)]
    params: std::collections::BTreeMap<String, String>,
    /// Generation parameters for the new value. The caller chooses the shape, never the value.
    #[serde(default)]
    generate: Option<GenerateSpec>,
}

/// POST /v1/secrets/{key}/rotate — operator and above.
///
/// Rotating changes a stored value, which is what `set` requires the operator role for. An agent
/// may *run* a grant that reads secrets; changing one is a different thing.
pub(crate) async fn rotate(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<crate::repo::Identity>,
    Path(params): Path<SecretPathParams>,
    Json(payload): Json<RotatePayload>,
) -> Result<SealboxResponse> {
    let key = params.secret_key();

    let grant = state
        .grant_repo
        .get(&payload.via)?
        .ok_or_else(|| SealboxError::GrantNotFound(payload.via.clone()))?;

    // Refuse to rotate something that does not exist: there would be no previous value to fall
    // back to, which is the property that makes a failed rotation safe.
    let current = state.secret_repo.get_secret(&key)?;

    // The server generates the value. A caller supplying one is refused rather than honoured.
    let master_key = state.master_key_repo.get_valid_master_key()?;
    let spec = payload.generate.unwrap_or(GenerateSpec {
        kind: crate::repo::GenerateKind::Password,
        length: None,
    });
    let pending = state.secret_repo.create_new_version(
        &key,
        &SecretValue::Generated(spec),
        master_key,
        None,
        // Carried forward. Losing the policy at the first rotation that honoured it would be the
        // worst possible moment to lose it.
        current.rotate_after,
        true,
    )?;

    let rotation = crate::repo::Rotation {
        secret: key.clone(),
        version: pending.version,
        capture: payload.from_output,
    };
    let job = state.job_repo.submit(
        &grant.name,
        &grant.runner,
        &payload.params,
        &identity.name,
        Some(&rotation),
    )?;

    Ok(SealboxResponse::Json(json!({
        "job": job.id,
        "secret": key,
        "pending_version": pending.version,
        "note": "The new value was generated on the server and is not returned to anyone. It \
                 becomes current only if the grant succeeds."
    })))
}
