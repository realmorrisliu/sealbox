//! Registering the issuers whose signatures sealbox will accept.
//!
//! Admin-only, because registering one says "identities from this platform may act here" — the
//! irreversible widening of authority that ADR 0013 keeps a person for. What is stored is public
//! key material, so a leak of this table lets someone verify tokens and never mint one.

use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    api::{SealboxResponse, path::Path, state::AppState, workload},
    error::{Result, SealboxError},
    repo::{AuditOutcome, Issuer, NewAuditRecord},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterIssuerPayload {
    name: String,
    url: String,
    /// The issuer's JWKS, as published. `kubectl get --raw /openid/v1/jwks` for a cluster.
    jwks: String,
}

/// POST /v1/issuers
pub(crate) async fn register(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<crate::repo::Identity>,
    Json(payload): Json<RegisterIssuerPayload>,
) -> Result<SealboxResponse> {
    // Checked while a person is here to fix it. A JWKS that does not parse becomes a runner that
    // cannot authenticate, discovered at the worst possible time.
    let keys = workload::validate_jwks(&payload.jwks)?;

    if payload.url.trim().is_empty() {
        return Err(SealboxError::InvalidRequest(
            "an issuer needs the URL its tokens carry in `iss`".to_string(),
        ));
    }

    let issuer = Issuer {
        name: payload.name.clone(),
        url: payload.url.clone(),
        jwks: payload.jwks,
        created_at: time::OffsetDateTime::now_utc().unix_timestamp(),
    };
    state.issuer_repo.register(&issuer)?;

    state.audit_repo.append(&NewAuditRecord {
        identity: Some(identity.name),
        action: "issuer.register".to_string(),
        resource: Some(issuer.name.clone()),
        outcome: AuditOutcome::Allowed,
        detail: Some(format!("{} with {keys} key(s)", issuer.url)),
    })?;

    Ok(SealboxResponse::Json(json!({
        "name": issuer.name,
        "url": issuer.url,
        "keys": keys,
    })))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct IssuerPathParams {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateKeysPayload {
    jwks: String,
}

/// PUT /v1/issuers/{name} — replace the keys, which is how a signing-key rotation lands.
pub(crate) async fn update_keys(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<crate::repo::Identity>,
    Path(params): Path<IssuerPathParams>,
    Json(payload): Json<UpdateKeysPayload>,
) -> Result<SealboxResponse> {
    let keys = workload::validate_jwks(&payload.jwks)?;
    state.issuer_repo.update_keys(&params.name, &payload.jwks)?;

    state.audit_repo.append(&NewAuditRecord {
        identity: Some(identity.name),
        action: "issuer.update".to_string(),
        resource: Some(params.name.clone()),
        outcome: AuditOutcome::Allowed,
        detail: Some(format!("now holds {keys} key(s)")),
    })?;

    Ok(SealboxResponse::Json(
        json!({ "name": params.name, "keys": keys }),
    ))
}

/// GET /v1/issuers
pub(crate) async fn list(State(state): State<AppState>) -> Result<SealboxResponse> {
    Ok(SealboxResponse::Json(
        json!({ "issuers": state.issuer_repo.list()? }),
    ))
}

/// DELETE /v1/issuers/{name}
pub(crate) async fn remove(
    State(state): State<AppState>,
    axum::Extension(identity): axum::Extension<crate::repo::Identity>,
    Path(params): Path<IssuerPathParams>,
) -> Result<SealboxResponse> {
    state.issuer_repo.remove(&params.name)?;

    state.audit_repo.append(&NewAuditRecord {
        identity: Some(identity.name),
        action: "issuer.remove".to_string(),
        resource: Some(params.name),
        outcome: AuditOutcome::Allowed,
        detail: Some("every identity bound to it stops authenticating".to_string()),
    })?;

    Ok(SealboxResponse::Ok)
}
