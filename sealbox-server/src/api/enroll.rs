use axum::extract::{Json, State};
use rand::{Rng, distributions::Alphanumeric};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    api::{SealboxResponse, Version, path::Path as SealboxPath, state::AppState},
    error::{Result, SealboxError},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct EnrollPathParams {
    version: Version,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct EnrollCodePathParams {
    version: Version,
    code: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ApprovePayload {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct EnrollBeginResponse {
    code: String,
    verify_url: String,
    expires_at: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct EnrollStatusResponse {
    code: String,
    status: String, // Pending | Approved | Expired
    name: Option<String>,
    description: Option<String>,
    expires_at: i64,
}

fn generate_code() -> String {
    let left: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .filter(|c| c.is_ascii_alphanumeric())
        .take(4)
        .map(|c| (c as char).to_ascii_uppercase())
        .collect();
    let right: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .filter(|c| c.is_ascii_alphanumeric())
        .take(4)
        .map(|c| (c as char).to_ascii_uppercase())
        .collect();
    format!("{left}-{right}")
}

// POST /v1/enroll
pub(crate) async fn begin_enrollment(
    State(state): State<AppState>,
    SealboxPath(_params): SealboxPath<EnrollPathParams>,
) -> Result<SealboxResponse> {
    let code = generate_code();
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let ttl = 10 * 60; // 10 minutes
    let expires_at = now + ttl;

    state
        .enroll_repo
        .create(&state.pool, &code, expires_at)
        .await?;

    let verify_url = format!("{}/enroll/{}", state.config.listen_addr, code);
    Ok(SealboxResponse::Json(json!(EnrollBeginResponse {
        code,
        verify_url,
        expires_at,
    })))
}

// GET /v1/enroll/{code}
pub(crate) async fn check_enrollment(
    State(state): State<AppState>,
    SealboxPath(params): SealboxPath<EnrollCodePathParams>,
) -> Result<SealboxResponse> {
    let rec = state.enroll_repo.get(&state.pool, &params.code).await?;
    match rec {
        None => Err(SealboxError::InvalidInput("Invalid code".into())),
        Some(r) => Ok(SealboxResponse::Json(json!(EnrollStatusResponse {
            code: r.code,
            status: r.status,
            name: r.name,
            description: r.description,
            expires_at: r.expires_at,
        }))),
    }
}

// PUT /v1/enroll/{code}/approve
pub(crate) async fn approve_enrollment(
    State(state): State<AppState>,
    SealboxPath(params): SealboxPath<EnrollCodePathParams>,
    Json(payload): Json<ApprovePayload>,
) -> Result<SealboxResponse> {
    state
        .enroll_repo
        .approve(
            &state.pool,
            &params.code,
            payload.name.clone(),
            payload.description.clone(),
        )
        .await?;

    Ok(SealboxResponse::Json(json!({
        "code": params.code,
        "approved": true,
        "name": payload.name,
        "description": payload.description,
    })))
}
