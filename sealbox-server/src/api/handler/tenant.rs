use axum::extract::{Json, Path, State};
use serde::Deserialize;
use serde_json::json;
use tracing::info;
use uuid::Uuid;

use crate::{
    api::{SealboxResponse, state::AppState},
    error::Result,
    repo::TenantStatus,
};

#[derive(Debug, Deserialize)]
pub(crate) struct CreateTenantPayload {
    display_name: Option<String>,
    token_label: Option<String>,
    token_expires_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateTokenPayload {
    label: Option<String>,
    expires_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TenantPath {
    tenant_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenPath {
    tenant_id: String,
    token_id: Uuid,
}

pub(crate) async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateTenantPayload>,
) -> Result<SealboxResponse> {
    let mut conn = state.conn_pool.lock()?;
    let tx = conn.transaction()?;
    let tenant = state
        .tenant_repo
        .create_tenant(&tx, clean_optional(payload.display_name))?;
    let issued = state.tenant_repo.issue_token(
        &tx,
        &tenant.id,
        clean_optional(payload.token_label),
        payload.token_expires_at,
    )?;
    tx.commit()?;
    info!(
        tenant_id = tenant.id,
        token_id = %issued.metadata.id,
        "created tenant and initial API token"
    );
    Ok(SealboxResponse::Json(json!({
        "tenant": tenant,
        "token": issued.token,
        "token_metadata": issued.metadata,
    })))
}

pub(crate) async fn list(State(state): State<AppState>) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    let tenants = state.tenant_repo.list_tenants(&conn)?;
    Ok(SealboxResponse::Json(json!({ "tenants": tenants })))
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Path(path): Path<TenantPath>,
) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    let tenant = state
        .tenant_repo
        .get_tenant(&conn, &path.tenant_id)?
        .ok_or(crate::error::SealboxError::TenantNotFound(path.tenant_id))?;
    Ok(SealboxResponse::Json(json!({ "tenant": tenant })))
}

pub(crate) async fn suspend(
    State(state): State<AppState>,
    Path(path): Path<TenantPath>,
) -> Result<SealboxResponse> {
    set_status(state, path.tenant_id, TenantStatus::Suspended)
}

pub(crate) async fn resume(
    State(state): State<AppState>,
    Path(path): Path<TenantPath>,
) -> Result<SealboxResponse> {
    set_status(state, path.tenant_id, TenantStatus::Active)
}

fn set_status(state: AppState, tenant_id: String, status: TenantStatus) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    let tenant = state
        .tenant_repo
        .set_tenant_status(&conn, &tenant_id, status)?;
    info!(tenant_id = tenant.id, status = ?tenant.status, "updated tenant status");
    Ok(SealboxResponse::Json(json!({ "tenant": tenant })))
}

pub(crate) async fn create_token(
    State(state): State<AppState>,
    Path(path): Path<TenantPath>,
    Json(payload): Json<CreateTokenPayload>,
) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    let issued = state.tenant_repo.issue_token(
        &conn,
        &path.tenant_id,
        clean_optional(payload.label),
        payload.expires_at,
    )?;
    info!(
        tenant_id = path.tenant_id,
        token_id = %issued.metadata.id,
        "created tenant API token"
    );
    Ok(SealboxResponse::Json(json!({
        "token": issued.token,
        "token_metadata": issued.metadata,
    })))
}

pub(crate) async fn list_tokens(
    State(state): State<AppState>,
    Path(path): Path<TenantPath>,
) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    let tokens = state.tenant_repo.list_tokens(&conn, &path.tenant_id)?;
    Ok(SealboxResponse::Json(json!({ "tokens": tokens })))
}

pub(crate) async fn revoke_token(
    State(state): State<AppState>,
    Path(path): Path<TokenPath>,
) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    state
        .tenant_repo
        .revoke_token(&conn, &path.tenant_id, &path.token_id)?;
    info!(
        tenant_id = path.tenant_id,
        token_id = %path.token_id,
        "revoked tenant API token"
    );
    Ok(SealboxResponse::Ok)
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}
