//! Recovery: the server's master key, encrypted under a key the server does not hold.
//!
//! The master key is the only thing that can read the store, and replication covers the database
//! and not the key — so without this a lost volume means a healthy-looking backup that decrypts to
//! nothing.
//!
//! No new mechanism. A recovery key is a master key with `server_held = 0` — the cold path from
//! ADR 0001 — and a blob is envelope-encrypted exactly as a secret is.

use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    api::{SealboxResponse, path::Path, state::AppState},
    crypto::{data_key::DataKey, master_key::PublicMasterKey},
    error::{Result, SealboxError},
    repo::{AuditOutcome, MasterKey, NewAuditRecord, RecoveryBlob},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterRecoveryPayload {
    /// The **public** half only. Sending the private half would defeat the entire point: the blob
    /// is safe to store anywhere precisely because the server cannot open it.
    public_key: String,
    description: Option<String>,
}

/// POST /v1/recovery — register a recovery key and take the first blob under it.
pub(crate) async fn register(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<crate::repo::Identity>,
    Json(payload): Json<RegisterRecoveryPayload>,
) -> Result<SealboxResponse> {
    // Rejects a private key outright: someone pasting the wrong half must not have it silently
    // accepted and stored.
    if payload.public_key.contains("PRIVATE KEY") {
        return Err(SealboxError::InvalidRequest(
            "that is a private key. Send the public half — the server must not be able to open \
             its own recovery blob, which is what makes the blob safe to store anywhere."
                .to_string(),
        ));
    }
    payload.public_key.parse::<PublicMasterKey>()?;

    let mut key = MasterKey::new(payload.public_key)?;
    key.description = payload
        .description
        .or_else(|| Some("recovery key".to_string()));
    state.master_key_repo.create_master_key(&key)?;

    let blob = state.refresh_recovery_blob(&key)?;

    state.audit_repo.append(&NewAuditRecord {
        identity: Some(identity.name),
        action: "recovery.register".to_string(),
        resource: Some(key.id.to_string()),
        outcome: AuditOutcome::Allowed,
        detail: Some(format!(
            "master key {} is now recoverable with this key",
            blob.master_key_fingerprint
        )),
    })?;

    Ok(SealboxResponse::Json(json!({
        "recovery_key_id": key.id,
        "master_key_fingerprint": blob.master_key_fingerprint,
        "note": "Verify it: fetch the blob and decrypt it with the private half you kept. An \
                 unverified backup is reliably not a backup."
    })))
}

/// GET /v1/recovery — every blob, or one of them.
///
/// Safe to hand out to an admin and safe for them to store anywhere: without the private half a
/// blob yields nothing.
pub(crate) async fn list(State(state): State<AppState>) -> Result<SealboxResponse> {
    Ok(SealboxResponse::Json(
        json!({ "blobs": state.recovery_repo.list()? }),
    ))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct RecoveryPathParams {
    id: uuid::Uuid,
}

/// GET /v1/recovery/{id}
pub(crate) async fn export(
    State(state): State<AppState>,
    Path(params): Path<RecoveryPathParams>,
) -> Result<SealboxResponse> {
    let blob = state
        .recovery_repo
        .get(&params.id)?
        .ok_or_else(|| SealboxError::InvalidRequest("no recovery blob for that key".into()))?;
    Ok(SealboxResponse::Json(json!(blob)))
}

/// DELETE /v1/recovery/{id} — retire a recovery key.
///
/// Separate from registering another, so that "add the new one" and "retire the old one" cannot be
/// confused for each other.
pub(crate) async fn remove(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<crate::repo::Identity>,
    Path(params): Path<RecoveryPathParams>,
) -> Result<SealboxResponse> {
    state.recovery_repo.remove(&params.id)?;

    state.audit_repo.append(&NewAuditRecord {
        identity: Some(identity.name),
        action: "recovery.remove".to_string(),
        resource: Some(params.id.to_string()),
        outcome: AuditOutcome::Allowed,
        detail: Some("that key can no longer recover this server".to_string()),
    })?;

    Ok(SealboxResponse::Ok)
}

impl AppState {
    /// Encrypt the current master key under one recovery key and store the result.
    ///
    /// The payload is the master key file's bytes, so a restore is a file copy after decryption
    /// rather than a re-serialisation that has to agree with whatever wrote it.
    pub(crate) fn refresh_recovery_blob(&self, recovery_key: &MasterKey) -> Result<RecoveryBlob> {
        let path = self.config.master_key_paths.first().ok_or_else(|| {
            SealboxError::ConfigError("no server master key is configured".to_string())
        })?;
        let master_key_pem = std::fs::read(path).map_err(|e| {
            SealboxError::ConfigError(format!("Cannot read the server master key at {path}: {e}"))
        })?;

        let data_key = DataKey::new();
        let encrypted_data = data_key.encrypt(&master_key_pem)?;
        let public: PublicMasterKey = recovery_key.public_key.parse()?;
        let encrypted_data_key = public.encrypt(data_key.as_bytes())?;

        let blob = RecoveryBlob {
            recovery_key_id: recovery_key.id,
            encrypted_data,
            encrypted_data_key,
            master_key_fingerprint: crate::api::state::fingerprint(
                &recovery_key_fingerprint_source(self)?,
            ),
            created_at: time::OffsetDateTime::now_utc().unix_timestamp(),
        };
        self.recovery_repo.store(&blob)?;
        Ok(blob)
    }

    /// Re-make every blob. Called whenever the server's master key changes: a backup that quietly
    /// stops matching what it is meant to restore is worse than no backup, and remembering to
    /// refresh it is exactly what nobody should have to do (ADR 0013).
    pub(crate) fn refresh_every_recovery_blob(&self) -> Result<usize> {
        let recovery_keys: Vec<MasterKey> = self
            .master_key_repo
            .fetch_all_master_keys()?
            .into_iter()
            .filter(|k| !k.server_held)
            .collect();

        let mut refreshed = 0;
        for key in recovery_keys {
            // Only keys that actually hold a blob: a cold key registered for secrets is not a
            // recovery key, and giving it one would hand it the master key it was kept away from.
            if self.recovery_repo.get(&key.id)?.is_none() {
                continue;
            }
            // Re-read it: the listing deliberately returns `[HIDDEN]` in place of the public key,
            // so the row from `fetch_all_master_keys` cannot be encrypted to.
            let key = self
                .master_key_repo
                .fetch_master_key(&key.id)?
                .ok_or_else(|| {
                    SealboxError::InvalidRequest(format!("recovery key {} vanished", key.id))
                })?;
            self.refresh_recovery_blob(&key)?;
            refreshed += 1;
        }
        Ok(refreshed)
    }
}

/// The public half of the current server master key, which is what the fingerprint names. Reading
/// it from the loaded key rather than the file keeps the fingerprint the same one startup logs.
fn recovery_key_fingerprint_source(state: &AppState) -> Result<String> {
    let current = state.master_key_repo.get_valid_master_key()?;
    Ok(current.public_key)
}
