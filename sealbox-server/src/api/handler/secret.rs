use axum::extract::{Json, Query, State};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    api::{SealboxResponse, Version, path::Path, state::AppState},
    error::{Result, SealboxError},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct SecretPathParams {
    version: Version,
    secret_key: String,
}

impl SecretPathParams {
    fn version(&self) -> Version {
        self.version.clone()
    }
    fn secret_key(&self) -> String {
        self.secret_key.clone()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct GetSecretQueryParams {
    version: Option<i32>,
}

/// 获取秘密数据的API处理函数
///
/// # Arguments
///
/// * `state` - 应用状态，包含数据库连接池和仓库实例
/// * `params` - 路径参数，包含API版本和秘密键名
/// * `query` - 查询参数，可选的版本号用于获取特定版本
///
/// # Returns
///
/// 返回加密的秘密数据，包含加密内容和加密的数据密钥
///
/// # Errors
///
/// * `SealboxError::SecretNotFound` - 当秘密不存在时
/// * `SealboxError::InvalidApiVersion` - 当API版本不支持时
///
/// # HTTP Route
///
/// `GET /{version}/secrets/{secret_key}[?version=N]`
///
/// # Security Notes
///
/// 如果未指定版本号，返回最新版本。返回的数据仍然是加密状态，需要客户端使用对应的私钥解密。
pub(crate) async fn get(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Query(query): Query<GetSecretQueryParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let secret = match query.version {
                Some(version) => {
                    state
                        .secret_repo
                        .get_secret_by_version(&conn, &params.secret_key(), version)?
                }
                None => state.secret_repo.get_secret(&conn, &params.secret_key())?,
            };
            Ok(SealboxResponse::Json(json!(secret)))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct SaveSecretPayload {
    secret: String,
    ttl: Option<i64>,
}

// PUT /{version}/secrets/{secret_key}
pub(crate) async fn save(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Json(payload): Json<SaveSecretPayload>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let mut conn = state.conn_pool.lock()?;
            let master_key = state.master_key_repo.get_valid_master_key(&conn)?;

            let secret = state.secret_repo.create_new_version(
                &mut conn,
                &params.secret_key(),
                &payload.secret,
                master_key,
                payload.ttl,
            )?;

            Ok(SealboxResponse::Json(json!(secret)))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteSecretQueryParams {
    version: i32,
}

// DELETE /{version}/secrets/{secret_key}
pub(crate) async fn delete(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Query(query): Query<DeleteSecretQueryParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            state.secret_repo.delete_secret_by_version(
                &conn,
                &params.secret_key(),
                query.version,
            )?;
            Ok(SealboxResponse::Ok)
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}
