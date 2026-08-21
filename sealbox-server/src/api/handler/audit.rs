use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::json;

use crate::{
    api::{SealboxResponse, state::AppState},
    error::Result,
    repo::AuditFilter,
};

#[derive(Debug, Deserialize)]
pub(crate) struct AuditQueryParams {
    identity: Option<String>,
    action: Option<String>,
    /// Unix timestamp; records at or after it.
    since: Option<i64>,
    limit: Option<usize>,
}

/// GET /v1/audit
///
/// Readable by every authenticated identity, including agents. Concealing the trail from an
/// agent protects nothing it could not already observe, and an agent able to check what it did
/// is more useful than one that cannot.
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(params): Query<AuditQueryParams>,
) -> Result<SealboxResponse> {
    let records = state.audit_repo.query(&AuditFilter {
        identity: params.identity,
        action: params.action,
        since: params.since,
        limit: Some(params.limit.unwrap_or(100).min(1000)),
    })?;
    Ok(SealboxResponse::Json(json!({ "audit": records })))
}
