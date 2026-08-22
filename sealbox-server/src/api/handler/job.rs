use std::collections::BTreeMap;
use std::time::Duration;

use axum::{
    Extension,
    extract::{Json, State},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    api::{SealboxResponse, path::Path, state::AppState},
    crypto::data_key::DataKey,
    error::{Result, SealboxError},
    repo::{AuditOutcome, ClaimedJob, Identity, Job, NewAuditRecord, Secret},
};

/// How long a claim request waits before returning empty, and how often it retries in between.
/// `sealbox run` is synchronous — someone is watching — so a plain multi-second poll would make
/// every invocation feel broken.
const CLAIM_TIMEOUT: Duration = Duration::from_secs(30);
const CLAIM_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubmitJobPayload {
    /// The grant to run. A caller supplies a name and parameters — never a command, a script, or
    /// a secret name (ADR 0003). Unknown fields are rejected rather than ignored, so an attempt
    /// to smuggle one in fails loudly.
    grant: String,
    #[serde(default)]
    params: BTreeMap<String, String>,
}

/// POST /v1/jobs — agent and above.
pub(crate) async fn submit(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Json(payload): Json<SubmitJobPayload>,
) -> Result<SealboxResponse> {
    let grant = state
        .grant_repo
        .get(&payload.grant)?
        .ok_or_else(|| SealboxError::GrantNotFound(payload.grant.clone()))?;

    // The runner is copied from the grant now, so a later change to the grant cannot redirect
    // work already queued.
    let job = state
        .job_repo
        .submit(&grant.name, &grant.runner, &payload.params, &identity.name)?;

    state.audit_repo.append(&NewAuditRecord {
        identity: Some(identity.name.clone()),
        action: "job.submit".to_string(),
        resource: Some(grant.name.clone()),
        outcome: AuditOutcome::Allowed,
        detail: Some(format!("job {} queued for runner {}", job.id, grant.runner)),
    })?;

    Ok(SealboxResponse::Json(json!(job)))
}

/// GET /v1/jobs/{id} — the waiting caller. Carries the exit status and output, never a value.
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(params): Path<JobPathParams>,
) -> Result<SealboxResponse> {
    let job = state
        .job_repo
        .get(params.id)?
        .ok_or_else(|| SealboxError::InvalidRequest(format!("no job {}", params.id)))?;
    Ok(SealboxResponse::Json(json!(job)))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct JobPathParams {
    id: i64,
}

/// GET /v1/jobs/claim — runner only.
///
/// Long-polls: retries the claim every 200ms until one succeeds or the timeout elapses. A sleep
/// loop rather than a notification bus — at one runner per name, this is one cheap indexed query
/// every 200ms.
///
/// > ponytail: polling loop. If the number of runners ever makes this measurable, the fix is a
/// > notify channel keyed by runner name, not a longer interval.
pub(crate) async fn claim(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
) -> Result<SealboxResponse> {
    let deadline = std::time::Instant::now() + CLAIM_TIMEOUT;

    loop {
        if let Some(job) = state.job_repo.claim_next(&identity.name)? {
            let claimed = prepare(&state, &job)?;

            state.audit_repo.append(&NewAuditRecord {
                identity: Some(identity.name.clone()),
                action: "job.claim".to_string(),
                resource: Some(job.grant.clone()),
                outcome: AuditOutcome::Allowed,
                detail: Some(format!("job {}", job.id)),
            })?;

            return Ok(SealboxResponse::Json(json!(claimed)));
        }

        if std::time::Instant::now() >= deadline {
            return Ok(SealboxResponse::Json(json!(null)));
        }
        tokio::time::sleep(CLAIM_INTERVAL).await;
    }
}

/// Assemble what the runner needs: the implementation, and the plaintext of **only** the secrets
/// the grant declares. This is the one path by which plaintext leaves the server; there is no
/// endpoint, for any role, that fetches a secret by name.
fn prepare(state: &AppState, job: &Job) -> Result<ClaimedJob> {
    let grant = state
        .grant_repo
        .get(&job.grant)?
        .ok_or_else(|| SealboxError::GrantNotFound(job.grant.clone()))?;

    let mut secrets = BTreeMap::new();
    for (injected_as, name) in &grant.secrets {
        let secret = state.secret_repo.get_secret(name)?;
        secrets.insert(injected_as.clone(), decrypt(state, &secret)?);
    }
    let mut files = BTreeMap::new();
    for (injected_as, name) in &grant.files {
        let secret = state.secret_repo.get_secret(name)?;
        files.insert(injected_as.clone(), decrypt(state, &secret)?);
    }

    Ok(ClaimedJob {
        id: job.id,
        grant: grant.name,
        params: job.params.clone(),
        implementation: grant.implementation,
        secrets,
        files,
    })
}

fn decrypt(state: &AppState, secret: &Secret) -> Result<String> {
    // A secret under a key the server does not hold is cold: unreadable here, by design and
    // without exception (ADR 0001).
    let private_key = state
        .server_keys
        .get(&secret.master_key_id)
        .ok_or(SealboxError::MasterKeyNotServerHeld(secret.master_key_id))?;

    let data_key_bytes = private_key.decrypt(&secret.encrypted_data_key)?;
    let data_key = DataKey::from_bytes(&data_key_bytes)?;
    let plaintext = data_key.decrypt(&secret.encrypted_data)?;

    String::from_utf8(plaintext)
        .map_err(|_| SealboxError::CryptoError(format!("secret `{}` is not text", secret.key)))
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReportPayload {
    exit_code: i32,
    #[serde(default)]
    output: String,
}

/// POST /v1/jobs/{id}/result — runner only, and only the runner holding the claim.
pub(crate) async fn report(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(params): Path<JobPathParams>,
    Json(payload): Json<ReportPayload>,
) -> Result<SealboxResponse> {
    let job = state.job_repo.report(
        params.id,
        &identity.name,
        payload.exit_code,
        &payload.output,
    )?;

    state.audit_repo.append(&NewAuditRecord {
        identity: Some(identity.name.clone()),
        action: "job.result".to_string(),
        resource: Some(job.grant.clone()),
        outcome: if payload.exit_code == 0 {
            AuditOutcome::Allowed
        } else {
            AuditOutcome::Failed
        },
        // The status only. Captured output can contain anything the implementation printed.
        detail: Some(format!("job {} exited {}", job.id, payload.exit_code)),
    })?;

    // A chain is driven here, not by the runner: a runner that kept itself going would keep
    // going unsupervised if it were compromised.
    if payload.exit_code == 0 {
        queue_next_in_chain(&state, &job, &identity.name)?;
    }

    Ok(SealboxResponse::Json(json!(job)))
}

fn queue_next_in_chain(state: &AppState, job: &Job, by: &str) -> Result<()> {
    let Some(grant) = state.grant_repo.get(&job.grant)? else {
        return Ok(());
    };
    let Some(next_name) = grant.then.first() else {
        return Ok(());
    };
    let Some(next) = state.grant_repo.get(next_name)? else {
        return Ok(());
    };

    let queued = state
        .job_repo
        .submit(&next.name, &next.runner, &job.params, by)?;

    state.audit_repo.append(&NewAuditRecord {
        identity: Some(by.to_string()),
        action: "job.chain".to_string(),
        resource: Some(next.name.clone()),
        outcome: AuditOutcome::Allowed,
        detail: Some(format!("job {} follows job {}", queued.id, job.id)),
    })?;
    Ok(())
}
