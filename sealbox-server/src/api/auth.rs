use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use crate::{
    api::state::AppState,
    error::{Result, SealboxError},
    repo::{AuditOutcome, Identity, NewAuditRecord, Role},
};

/// Authenticate the caller, run the request, and record the attempt.
///
/// Authentication and audit are one layer because they need the same two things at opposite
/// ends: the identity is resolved here, and the outcome is only known after the inner layers
/// have run. Splitting them would mean either resolving the token twice or leaving refused
/// requests unrecorded — and a refused request is exactly what an injected agent produces.
pub(crate) async fn authenticate_and_audit(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response> {
    let action = format!("{} {}", request.method(), request.uri().path());
    let resource = resource_from_path(request.uri().path());

    let token = bearer_token(&request);
    let identity = match token {
        Some(token) => state.identity_repo.find_by_token(&token)?,
        None => None,
    };

    let Some(identity) = identity else {
        record(
            &state,
            NewAuditRecord {
                identity: None,
                action,
                resource,
                outcome: AuditOutcome::Unauthenticated,
                detail: None,
            },
        )?;
        return Err(SealboxError::Unauthorized);
    };

    let name = identity.name.clone();
    let mut request = request;
    request.extensions_mut().insert(identity);

    let response = next.run(request).await;

    let outcome = match response.status() {
        StatusCode::FORBIDDEN => AuditOutcome::Forbidden,
        status if status.is_success() => AuditOutcome::Allowed,
        _ => AuditOutcome::Failed,
    };
    record(
        &state,
        NewAuditRecord {
            identity: Some(name),
            action,
            resource,
            // The status only. Never a body, which could carry a value.
            detail: Some(response.status().as_u16().to_string()),
            outcome,
        },
    )?;

    Ok(response)
}

/// A failed audit write fails the request. An action that happened with no record is the one
/// outcome this capability exists to prevent, and audit shares a database with everything else,
/// so a write that fails here was going to fail anyway.
fn record(state: &AppState, record: NewAuditRecord) -> Result<()> {
    state.audit_repo.append(&record)
}

macro_rules! role_gate {
    ($name:ident, $role:expr, $doc:expr) => {
        #[doc = $doc]
        pub(crate) async fn $name(request: Request, next: Next) -> Result<Response> {
            let identity = request
                .extensions()
                .get::<Identity>()
                .ok_or(SealboxError::Unauthorized)?;

            if identity.role < $role {
                // Distinct from unauthorized: the caller proved who they are and it was not
                // enough. Telling the two apart lets a caller fix the right problem.
                return Err(SealboxError::Forbidden);
            }

            Ok(next.run(request).await)
        }
    };
}

role_gate!(
    require_agent,
    Role::Agent,
    "Admits any authenticated identity — every role is at or above Agent."
);
role_gate!(
    require_operator,
    Role::Operator,
    "Admits Operator and Admin."
);
role_gate!(require_admin, Role::Admin, "Admits Admin only.");

/// The thing being acted on, taken from the path where it already is.
fn resource_from_path(path: &str) -> Option<String> {
    let mut parts = path.trim_start_matches('/').split('/');
    let _version = parts.next()?;
    let collection = parts.next()?;
    match parts.next() {
        Some(name) if !name.is_empty() => Some(format!("{collection}/{name}")),
        _ => Some(collection.to_string()),
    }
}

fn bearer_token(request: &Request) -> Option<String> {
    request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
}
