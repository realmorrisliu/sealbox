use std::collections::{BTreeMap, HashSet};

use axum::{
    Extension,
    extract::{Json, State},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    api::{SealboxResponse, path::Path, state::AppState},
    error::{Result, SealboxError},
    repo::{Grant, Identity, Implementation, KNOWN_ADAPTERS},
};

/// Flat on the wire, because that is how a human writes it in TOML. Turned into the enum
/// immediately, where "both" and "neither" stop being expressible.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateGrantPayload {
    name: String,
    runner: String,
    #[serde(default)]
    secrets: BTreeMap<String, String>,
    /// Secrets that must be a file rather than an environment variable.
    #[serde(default)]
    files: BTreeMap<String, String>,
    #[serde(default)]
    then: Vec<String>,

    adapter: Option<String>,
    config: Option<serde_json::Value>,
    script: Option<String>,
    command: Option<Vec<String>>,
}

impl CreateGrantPayload {
    fn into_grant(self, created_by: &str) -> Result<Grant> {
        let implementation = match (self.adapter, self.script) {
            (Some(_), Some(_)) => {
                return Err(SealboxError::InvalidRequest(
                    "a grant names either an adapter or a script, not both".to_string(),
                ));
            }
            (None, None) => {
                return Err(SealboxError::InvalidRequest(
                    "a grant must name either an adapter or a script".to_string(),
                ));
            }
            (Some(adapter), None) => {
                if self.command.is_some() {
                    return Err(SealboxError::InvalidRequest(
                        "`command` belongs to a script; an adapter is configured with `config`"
                            .to_string(),
                    ));
                }
                Implementation::Adapter {
                    adapter,
                    config: self.config.unwrap_or(serde_json::Value::Null),
                }
            }
            (None, Some(script)) => {
                let command = self.command.ok_or_else(|| {
                    SealboxError::InvalidRequest(
                        "a script needs a `command`: the argv it is invoked with".to_string(),
                    )
                })?;
                if command.is_empty() {
                    return Err(SealboxError::InvalidRequest(
                        "`command` cannot be empty".to_string(),
                    ));
                }
                Implementation::Script { script, command }
            }
        };

        Ok(Grant {
            name: self.name,
            implementation,
            runner: self.runner,
            secrets: self.secrets,
            files: self.files,
            then: self.then,
            created_at: time::OffsetDateTime::now_utc().unix_timestamp(),
            created_by: created_by.to_string(),
        })
    }
}

/// POST /v1/grants — admin only. Does not create the grant: it stages it for approval.
///
/// The grant exists once a human has signed for it on a page the server rendered. Creating it
/// here and calling the signature a formality would put the decision back in a terminal, whose
/// output an agent writes.
pub(crate) async fn create(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Json(payload): Json<CreateGrantPayload>,
) -> Result<SealboxResponse> {
    let name = payload.name.clone();
    let payload = serde_json::to_value(payload)
        .map_err(|e| SealboxError::InvalidRequest(format!("grant payload: {e}")))?;

    // Validate now so a mistake is caught before anyone is asked to approve it. The grant itself
    // is created only after the signature.
    let candidate: CreateGrantPayload = serde_json::from_value(payload.clone())
        .map_err(|e| SealboxError::InvalidRequest(format!("grant payload: {e}")))?;
    let grant = candidate.into_grant(&identity.name)?;
    validate_secret_names_are_literal(&grant)?;
    validate_adapter(&grant)?;
    validate_secrets_exist(&state, &grant)?;
    validate_chain(&state, &grant)?;
    if state.grant_repo.get(&grant.name)?.is_some() {
        return Err(SealboxError::GrantAlreadyExists(grant.name));
    }

    let id = state
        .passkey
        .stash_approval(crate::api::passkey::PendingApproval {
            payload,
            requested_by: identity.name.clone(),
        });

    Ok(SealboxResponse::Json(json!({
        "pending_approval": id,
        "grant": name,
        "approve_at": format!("{}/approve/{id}", state.config.public_url),
        "note": "Open that URL — on this machine or on your phone — and approve it with your \
                 passkey. What the page shows is what you sign; a terminal cannot promise that."
    })))
}

/// Shared by the direct path and the passkey approval, so a grant approved through a signature
/// goes through exactly the same validation as one created directly.
pub(crate) fn create_from_payload(
    state: &AppState,
    payload: serde_json::Value,
    created_by: &str,
) -> Result<Grant> {
    let payload: CreateGrantPayload = serde_json::from_value(payload)
        .map_err(|e| SealboxError::InvalidRequest(format!("grant payload: {e}")))?;
    let grant = payload.into_grant(created_by)?;

    validate_secret_names_are_literal(&grant)?;
    validate_adapter(&grant)?;
    validate_secrets_exist(state, &grant)?;
    validate_chain(state, &grant)?;

    state.grant_repo.create(&grant)?;
    Ok(grant)
}

/// A parameterised secret name would let a caller choose which credential the grant reaches,
/// which would undo the guarantee that the declaration *is* the boundary. Parameters belong in
/// the implementation's arguments; two environments are two grants.
fn validate_secret_names_are_literal(grant: &Grant) -> Result<()> {
    for (injected_as, secret) in grant.all_declared() {
        if secret.contains('{') || secret.contains('}') {
            return Err(SealboxError::InvalidRequest(format!(
                "secret name `{secret}` (declared as `{injected_as}`) contains a parameter. \
                 Declared secrets must be named literally, or a caller could choose which \
                 credential this grant reaches — write one grant per set of secrets."
            )));
        }
    }
    Ok(())
}

fn validate_adapter(grant: &Grant) -> Result<()> {
    if let Implementation::Adapter { adapter, config } = &grant.implementation {
        if !KNOWN_ADAPTERS.contains(&adapter.as_str()) {
            return Err(SealboxError::InvalidRequest(format!(
                "unknown adapter `{adapter}`. Known adapters: {}",
                KNOWN_ADAPTERS.join(", ")
            )));
        }
        // The configuration is checked here rather than at execution, because a human is here
        // and can fix it. At three in the morning, mid-rotation, nobody is.
        crate::repo::adapter::validate_config(adapter, config)?;
    }
    Ok(())
}

fn validate_secrets_exist(state: &AppState, grant: &Grant) -> Result<()> {
    let existing: HashSet<String> = state
        .secret_repo
        .list_secrets()?
        .into_iter()
        .map(|s| s.key)
        .collect();

    for (injected_as, secret) in grant.all_declared() {
        if !existing.contains(secret) {
            return Err(SealboxError::InvalidRequest(format!(
                "grant declares secret `{secret}` (as `{injected_as}`), which does not exist"
            )));
        }
    }
    Ok(())
}

/// Walk the chain from this grant. Reaching the grant itself is a cycle.
///
/// A depth limit would also catch cycles, but it would refuse a long legitimate chain with a
/// message about depth — describing the symptom rather than the mistake.
fn validate_chain(state: &AppState, grant: &Grant) -> Result<()> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut pending: Vec<String> = grant.then.clone();

    while let Some(name) = pending.pop() {
        if name == grant.name {
            return Err(SealboxError::InvalidRequest(format!(
                "chain returns to `{}`, which would never terminate",
                grant.name
            )));
        }
        if !seen.insert(name.clone()) {
            continue;
        }
        let next = state.grant_repo.get(&name)?.ok_or_else(|| {
            SealboxError::InvalidRequest(format!(
                "chain names grant `{name}`, which does not exist"
            ))
        })?;
        pending.extend(next.then);
    }
    Ok(())
}

/// GET /v1/grants — any authenticated identity, so an agent can see what it may invoke and draft
/// a proposal for a human.
pub(crate) async fn list(State(state): State<AppState>) -> Result<SealboxResponse> {
    let grants = state.grant_repo.list()?;
    Ok(SealboxResponse::Json(json!({ "grants": grants })))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct GrantPathParams {
    name: String,
}

/// GET /v1/grants/{name}
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(params): Path<GrantPathParams>,
) -> Result<SealboxResponse> {
    let grant = state
        .grant_repo
        .get(&params.name)?
        .ok_or_else(|| SealboxError::GrantNotFound(params.name.clone()))?;
    Ok(SealboxResponse::Json(json!(grant)))
}

/// DELETE /v1/grants/{name} — admin only.
///
/// There is no update: replacing a grant is a removal and a creation, which puts the new
/// declaration in front of a human exactly as the first one was. An update endpoint would be the
/// natural place for a capability to widen quietly.
pub(crate) async fn remove(
    State(state): State<AppState>,
    Path(params): Path<GrantPathParams>,
) -> Result<SealboxResponse> {
    state.grant_repo.remove(&params.name)?;
    Ok(SealboxResponse::Ok)
}
