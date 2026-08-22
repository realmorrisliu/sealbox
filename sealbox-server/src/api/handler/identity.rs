use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    api::{SealboxResponse, path::Path, state::AppState},
    error::{Result, SealboxError},
    repo::{AuditOutcome, Identity, NewAuditRecord, Role},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateIdentityPayload {
    name: String,
    role: String,
    /// Bind this identity to a registered issuer instead of giving it a token. All three are
    /// required together: an issuer without a subject would accept anything that platform signs,
    /// and without an audience a token minted for something else would work here.
    issuer: Option<String>,
    subject: Option<String>,
    audience: Option<String>,
}

/// POST /v1/identities
pub(crate) async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateIdentityPayload>,
) -> Result<SealboxResponse> {
    let role: Role = payload.role.parse()?;

    // Bound to a platform rather than holding a credential. Nothing is returned, because there is
    // nothing to return — which is the whole point: no Secret to leak and nothing to rotate.
    if payload.issuer.is_some() || payload.subject.is_some() || payload.audience.is_some() {
        let (Some(issuer), Some(subject), Some(audience)) =
            (payload.issuer, payload.subject, payload.audience)
        else {
            return Err(SealboxError::InvalidRequest(
                "binding an identity to an issuer needs all three: --issuer, --subject, and \
                 --audience. An issuer without a subject would admit anything that platform \
                 signs; without an audience, a token minted for something else would work here."
                    .to_string(),
            ));
        };

        if state.issuer_repo.find_by_url(&issuer)?.is_none()
            && !state.issuer_repo.list()?.iter().any(|i| i.name == issuer)
        {
            return Err(SealboxError::InvalidRequest(format!(
                "no issuer named `{issuer}` is registered"
            )));
        }

        let identity = Identity::workload(payload.name, role, issuer, subject, audience);
        state.identity_repo.create(&identity)?;
        return Ok(SealboxResponse::Json(json!({
            "name": identity.name,
            "role": role.to_string(),
            "issuer": identity.issuer,
            "subject": identity.subject,
            "audience": identity.audience,
            "note": "No credential was issued. This identity authenticates by presenting a token \
                     its platform signs, so there is nothing to store and nothing to rotate."
        })));
    }

    let (identity, token) = Identity::new(payload.name, role)?;
    state.identity_repo.create(&identity)?;

    // An admin gets no credential at all — there is nothing to leak. It enrols an authenticator
    // instead, and authenticates by proving possession of it (ADR 0009).
    if role == Role::Admin {
        let enrolment = state.passkey.issue_enrolment(&identity.name);
        return Ok(SealboxResponse::Json(json!({
            "name": identity.name,
            "role": role.to_string(),
            "enrol_at": format!("{}/enrol/{enrolment}", state.config.public_url),
            "note": "Open that URL to register a passkey. No token is issued: an admin \
                     credential on disk is exactly what an agent on your machine would read."
        })));
    }

    // The only time the plaintext token is ever returned.
    Ok(SealboxResponse::Json(json!({
        "name": identity.name,
        "role": identity.role.to_string(),
        "token": token,
        "warning": "This token is shown once and cannot be retrieved again."
    })))
}

/// GET /v1/identities
pub(crate) async fn list(State(state): State<AppState>) -> Result<SealboxResponse> {
    let identities = state.identity_repo.list()?;
    Ok(SealboxResponse::Json(json!({ "identities": identities })))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct IdentityPathParams {
    name: String,
}

/// DELETE /v1/identities/{name}
pub(crate) async fn revoke(
    State(state): State<AppState>,
    Path(params): Path<IdentityPathParams>,
) -> Result<SealboxResponse> {
    state.identity_repo.revoke(&params.name)?;
    Ok(SealboxResponse::Ok)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BootstrapPayload {
    token: String,
    name: String,
}

/// POST /v1/bootstrap — create the first admin on a server that has none.
///
/// Three conditions, all required: no identity exists, the token matches, and the server started
/// less than the bootstrap window ago. It is not modelled as a permanent identity with a magic
/// name, because such a row would persist and have to be defended against afterwards.
pub(crate) async fn bootstrap(
    State(state): State<AppState>,
    Json(payload): Json<BootstrapPayload>,
) -> Result<SealboxResponse> {
    let refuse = |detail: &str| -> Result<()> {
        state.audit_repo.append(&NewAuditRecord {
            identity: None,
            action: "POST /v1/bootstrap".to_string(),
            resource: None,
            outcome: AuditOutcome::Unauthenticated,
            detail: Some(detail.to_string()),
        })?;
        Err(SealboxError::Unauthorized)
    };

    if state.identity_repo.any_exists()? {
        refuse("an identity already exists")?;
    }
    if std::time::Instant::now() > state.bootstrap_deadline {
        refuse("bootstrap window has closed")?;
    }
    let Some(expected) = state.config.bootstrap_token.as_deref() else {
        refuse("no bootstrap token is configured")?;
        unreachable!()
    };
    if payload.token != expected {
        refuse("bootstrap token did not match")?;
    }

    let (identity, _token) = Identity::new(payload.name, Role::Admin)?;
    state.identity_repo.create(&identity)?;
    let enrolment = state.passkey.issue_enrolment(&identity.name);

    // Recorded against an empty trail: the first entry is how the server was claimed.
    state.audit_repo.append(&NewAuditRecord {
        identity: Some(identity.name.clone()),
        action: "POST /v1/bootstrap".to_string(),
        resource: None,
        outcome: AuditOutcome::Allowed,
        detail: Some("first admin created".to_string()),
    })?;

    Ok(SealboxResponse::Json(json!({
        "name": identity.name,
        "role": identity.role.to_string(),
        "enrol_at": format!("{}/enrol/{enrolment}", state.config.public_url),
        "note": "Open that URL to register a passkey. Then unset SEALBOX_BOOTSTRAP_TOKEN — it \
                 has served its purpose and only widens exposure from here."
    })))
}
