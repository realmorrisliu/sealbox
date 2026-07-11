use axum::{
    Extension,
    extract::{Json, State},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tracing::error;
use uuid::Uuid;

use crate::{
    api::{SealboxResponse, Version, auth::TenantPrincipal, path::Path, state::AppState},
    error::{Result, SealboxError},
    repo::{LEGACY_TENANT_ID, MasterKey, MasterKeyStatus},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct MasterKeyPathParams {
    version: Version,
}

impl MasterKeyPathParams {
    fn version(&self) -> Version {
        self.version.clone()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct MasterKeyIdPathParams {
    version: Version,
    master_key_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct TenantMasterKeyIdPathParams {
    master_key_id: Uuid,
}

impl MasterKeyIdPathParams {
    fn version(&self) -> Version {
        self.version.clone()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct RotateMasterKeyPayload {
    new_master_key_id: Uuid,
    old_master_key_id: Uuid,
    updates: Vec<RewrappedSecretDataKey>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct RewrappedSecretDataKey {
    namespace: String,
    key: String,
    version: i32,
    encrypted_data_key: Vec<u8>,
}

// GET /{version}/master-key
pub(crate) async fn list(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let master_keys = state
                .master_key_repo
                .fetch_all_master_keys(&conn, LEGACY_TENANT_ID)?;
            Ok(SealboxResponse::Json(json!({ "master_keys": master_keys })))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

// GET /{version}/master-key/active
pub(crate) async fn active(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let master_key = state
                .master_key_repo
                .get_valid_master_key(&conn, LEGACY_TENANT_ID)?;
            Ok(SealboxResponse::Json(json!(master_key)))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

// GET /{version}/master-key/by-id/{master_key_id}
pub(crate) async fn get(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyIdPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let master_key = state
                .master_key_repo
                .fetch_master_key(&conn, LEGACY_TENANT_ID, &params.master_key_id)?
                .ok_or(SealboxError::MasterKeyNotFound(params.master_key_id))?;
            Ok(SealboxResponse::Json(json!(master_key)))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

// GET /{version}/master-key/by-id/{master_key_id}/secrets
pub(crate) async fn secrets(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyIdPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let _master_key = state
                .master_key_repo
                .fetch_master_key(&conn, LEGACY_TENANT_ID, &params.master_key_id)?
                .ok_or(SealboxError::MasterKeyNotFound(params.master_key_id))?;
            let secrets = state.secret_repo.fetch_secrets_by_master_key(
                &conn,
                LEGACY_TENANT_ID,
                &params.master_key_id,
            )?;
            Ok(SealboxResponse::Json(json!({ "secrets": secrets })))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

// PUT /{version}/master-key
pub(crate) async fn rotate(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyPathParams>,
    Json(payload): Json<RotateMasterKeyPayload>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let new_master_key_id = payload.new_master_key_id;
            let old_master_key_id = payload.old_master_key_id;
            let updates = payload.updates;

            if old_master_key_id == new_master_key_id {
                return Err(SealboxError::InvalidRequest(
                    "old and new master key ids must be different".to_string(),
                ));
            }

            let mut conn = state.conn_pool.lock()?;

            let _old_master_key = state
                .master_key_repo
                .fetch_master_key(&conn, LEGACY_TENANT_ID, &old_master_key_id)?
                .ok_or(SealboxError::MasterKeyNotFound(old_master_key_id))?;
            let _new_master_key = state
                .master_key_repo
                .fetch_master_key(&conn, LEGACY_TENANT_ID, &new_master_key_id)?
                .ok_or(SealboxError::MasterKeyNotFound(new_master_key_id))?;

            let secrets = state.secret_repo.fetch_secrets_by_master_key(
                &conn,
                LEGACY_TENANT_ID,
                &old_master_key_id,
            )?;

            if updates.len() != secrets.len() {
                return Err(SealboxError::InvalidRequest(format!(
                    "expected {} rewrapped data keys, got {}",
                    secrets.len(),
                    updates.len()
                )));
            }

            let mut updates_by_secret = HashMap::new();
            for update in updates {
                let key = (update.namespace, update.key, update.version);
                if updates_by_secret
                    .insert(key, update.encrypted_data_key)
                    .is_some()
                {
                    return Err(SealboxError::InvalidRequest(
                        "duplicate rewrapped secret update".to_string(),
                    ));
                }
            }

            let tx = conn.transaction()?;

            for secret in secrets {
                let update_key = (secret.namespace.clone(), secret.key.clone(), secret.version);
                let Some(encrypted_data_key) = updates_by_secret.remove(&update_key) else {
                    return Err(SealboxError::InvalidRequest(format!(
                        "missing rewrapped data key for {} version {}",
                        secret.key, secret.version
                    )));
                };
                let mut rotated_secret = secret;
                rotated_secret.encrypted_data_key = encrypted_data_key;
                rotated_secret.master_key_id = new_master_key_id;
                rotated_secret.updated_at = time::OffsetDateTime::now_utc().unix_timestamp();
                if let Err(err) = state
                    .secret_repo
                    .update_secret_master_key(&tx, &rotated_secret)
                {
                    error!(
                        "Failed to update rewrapped data key for secret {}: {}",
                        rotated_secret.key, err
                    );
                    return Err(err);
                }
            }

            tx.execute(
                "UPDATE master_keys SET status = ?1 WHERE namespace = ?2 AND id = ?3",
                rusqlite::params![
                    MasterKeyStatus::Retired,
                    LEGACY_TENANT_ID,
                    old_master_key_id
                ],
            )?;
            tx.execute(
                "UPDATE master_keys SET status = ?1 WHERE namespace = ?2 AND id = ?3",
                rusqlite::params![MasterKeyStatus::Active, LEGACY_TENANT_ID, new_master_key_id],
            )?;

            tx.commit()?;

            Ok(SealboxResponse::Json(
                json!({ "master_key": new_master_key_id }),
            ))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CreateMasterKeyPayload {
    public_key: String,
}

// POST /{version}/master-key
pub(crate) async fn create(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyPathParams>,
    Json(payload): Json<CreateMasterKeyPayload>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let mut master_key = MasterKey::new(payload.public_key)?;
            if state
                .master_key_repo
                .get_valid_master_key(&conn, LEGACY_TENANT_ID)
                .is_ok()
            {
                master_key.status = MasterKeyStatus::Retired;
            }
            state
                .master_key_repo
                .create_master_key(&conn, &master_key)?;
            Ok(SealboxResponse::Json(json!(master_key)))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

pub(crate) async fn list_v2(
    State(state): State<AppState>,
    Extension(principal): Extension<TenantPrincipal>,
) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    let keys = state
        .master_key_repo
        .fetch_all_master_keys(&conn, &principal.tenant_id)?;
    Ok(SealboxResponse::Json(json!({ "master_keys": keys })))
}

pub(crate) async fn active_v2(
    State(state): State<AppState>,
    Extension(principal): Extension<TenantPrincipal>,
) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    let key = state
        .master_key_repo
        .get_valid_master_key(&conn, &principal.tenant_id)?;
    Ok(SealboxResponse::Json(json!(key)))
}

pub(crate) async fn get_v2(
    State(state): State<AppState>,
    Extension(principal): Extension<TenantPrincipal>,
    Path(params): Path<TenantMasterKeyIdPathParams>,
) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    let key = state
        .master_key_repo
        .fetch_master_key(&conn, &principal.tenant_id, &params.master_key_id)?
        .ok_or(SealboxError::MasterKeyNotFound(params.master_key_id))?;
    Ok(SealboxResponse::Json(json!(key)))
}

pub(crate) async fn secrets_v2(
    State(state): State<AppState>,
    Extension(principal): Extension<TenantPrincipal>,
    Path(params): Path<TenantMasterKeyIdPathParams>,
) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    state
        .master_key_repo
        .fetch_master_key(&conn, &principal.tenant_id, &params.master_key_id)?
        .ok_or(SealboxError::MasterKeyNotFound(params.master_key_id))?;
    let secrets = state.secret_repo.fetch_secrets_by_master_key(
        &conn,
        &principal.tenant_id,
        &params.master_key_id,
    )?;
    Ok(SealboxResponse::Json(json!({ "secrets": secrets })))
}

pub(crate) async fn create_v2(
    State(state): State<AppState>,
    Extension(principal): Extension<TenantPrincipal>,
    Json(payload): Json<CreateMasterKeyPayload>,
) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock()?;
    let mut key = MasterKey::new_in_namespace(principal.tenant_id.clone(), payload.public_key)?;
    if state
        .master_key_repo
        .get_valid_master_key(&conn, &principal.tenant_id)
        .is_ok()
    {
        key.status = MasterKeyStatus::Retired;
    }
    state.master_key_repo.create_master_key(&conn, &key)?;
    Ok(SealboxResponse::Json(json!(key)))
}

pub(crate) async fn rotate_v2(
    State(state): State<AppState>,
    Extension(principal): Extension<TenantPrincipal>,
    Json(payload): Json<RotateMasterKeyPayload>,
) -> Result<SealboxResponse> {
    rotate_for_namespace(&state, &principal.tenant_id, payload)
}

fn rotate_for_namespace(
    state: &AppState,
    namespace: &str,
    payload: RotateMasterKeyPayload,
) -> Result<SealboxResponse> {
    let new_master_key_id = payload.new_master_key_id;
    let old_master_key_id = payload.old_master_key_id;
    if old_master_key_id == new_master_key_id {
        return Err(SealboxError::InvalidRequest(
            "old and new master key ids must be different".to_string(),
        ));
    }
    let mut conn = state.conn_pool.lock()?;
    state
        .master_key_repo
        .fetch_master_key(&conn, namespace, &old_master_key_id)?
        .ok_or(SealboxError::MasterKeyNotFound(old_master_key_id))?;
    state
        .master_key_repo
        .fetch_master_key(&conn, namespace, &new_master_key_id)?
        .ok_or(SealboxError::MasterKeyNotFound(new_master_key_id))?;
    let secrets =
        state
            .secret_repo
            .fetch_secrets_by_master_key(&conn, namespace, &old_master_key_id)?;
    if payload.updates.len() != secrets.len() {
        return Err(SealboxError::InvalidRequest(format!(
            "expected {} rewrapped data keys, got {}",
            secrets.len(),
            payload.updates.len()
        )));
    }
    let mut updates_by_secret = HashMap::new();
    for update in payload.updates {
        if update.namespace != namespace {
            return Err(SealboxError::InvalidRequest(
                "rewrapped secret namespace does not match authenticated tenant".to_string(),
            ));
        }
        let key = (update.namespace, update.key, update.version);
        if updates_by_secret
            .insert(key, update.encrypted_data_key)
            .is_some()
        {
            return Err(SealboxError::InvalidRequest(
                "duplicate rewrapped secret update".to_string(),
            ));
        }
    }
    let tx = conn.transaction()?;
    for secret in secrets {
        let update_key = (secret.namespace.clone(), secret.key.clone(), secret.version);
        let Some(encrypted_data_key) = updates_by_secret.remove(&update_key) else {
            return Err(SealboxError::InvalidRequest(format!(
                "missing rewrapped data key for {} version {}",
                secret.key, secret.version
            )));
        };
        let mut rotated = secret;
        rotated.encrypted_data_key = encrypted_data_key;
        rotated.master_key_id = new_master_key_id;
        rotated.updated_at = time::OffsetDateTime::now_utc().unix_timestamp();
        state.secret_repo.update_secret_master_key(&tx, &rotated)?;
    }
    tx.execute(
        "UPDATE master_keys SET status = ?1 WHERE namespace = ?2 AND id = ?3",
        rusqlite::params![MasterKeyStatus::Retired, namespace, old_master_key_id],
    )?;
    tx.execute(
        "UPDATE master_keys SET status = ?1 WHERE namespace = ?2 AND id = ?3",
        rusqlite::params![MasterKeyStatus::Active, namespace, new_master_key_id],
    )?;
    tx.commit()?;
    Ok(SealboxResponse::Json(
        json!({ "master_key": new_master_key_id }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::{Version, path::Path as SealboxPath, state::AppState},
        config::SealboxConfig,
        crypto::master_key::generate_key_pair,
        repo::{SqliteHealthRepo, SqliteMasterKeyRepo, SqliteSecretRepo, SqliteTenantRepo},
    };
    use axum::extract::State;
    use std::sync::{Arc, Mutex};

    fn setup_test_state() -> AppState {
        let conn = rusqlite::Connection::open_in_memory().expect("Should create in-memory DB");
        crate::repo::SqliteMasterKeyRepo::init_table(&conn).expect("Should init master_keys table");
        crate::repo::SqliteSecretRepo::init_table(&conn).expect("Should init secrets table");
        crate::repo::SqliteTenantRepo::init_tables(&conn).expect("Should init tenants tables");

        AppState {
            conn_pool: Arc::new(Mutex::new(conn)),
            master_key_repo: Arc::new(SqliteMasterKeyRepo),
            secret_repo: Arc::new(SqliteSecretRepo),
            health_repo: Arc::new(SqliteHealthRepo),
            tenant_repo: Arc::new(SqliteTenantRepo),
            config: Arc::new(SealboxConfig::default()),
        }
    }

    #[tokio::test]
    async fn test_create_master_key() {
        let state = setup_test_state();
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");

        let path_params = MasterKeyPathParams {
            version: Version::V1,
        };
        let payload = CreateMasterKeyPayload {
            public_key: public_pem.clone(),
        };

        let result = create(
            State(state.clone()),
            SealboxPath(path_params),
            Json(payload),
        )
        .await;

        assert!(result.is_ok());
        match result.unwrap() {
            SealboxResponse::Json(json_value) => {
                let master_key: MasterKey =
                    serde_json::from_value(json_value).expect("Should deserialize MasterKey");
                assert_eq!(master_key.public_key, public_pem);
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[tokio::test]
    async fn test_create_master_key_invalid_version() {
        let state = setup_test_state();
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");

        let path_params = MasterKeyPathParams {
            version: Version::V2,
        }; // Invalid version
        let payload = CreateMasterKeyPayload {
            public_key: public_pem,
        };

        let result = create(State(state), SealboxPath(path_params), Json(payload)).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SealboxError::InvalidApiVersion => {} // Expected
            _ => panic!("Expected InvalidApiVersion error"),
        }
    }

    #[tokio::test]
    async fn test_list_master_keys_empty() {
        let state = setup_test_state();
        let path_params = MasterKeyPathParams {
            version: Version::V1,
        };

        let result = list(State(state), Path(path_params)).await;

        assert!(result.is_ok());
        match result.unwrap() {
            SealboxResponse::Json(json_value) => {
                let keys = json_value
                    .get("master_keys")
                    .and_then(|value| value.as_array())
                    .expect("Should include master_keys");
                assert_eq!(keys.len(), 0);
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[tokio::test]
    async fn test_list_master_keys_with_data() {
        let state = setup_test_state();
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");

        // First create a master key
        let path_params = MasterKeyPathParams {
            version: Version::V1,
        };
        let payload = CreateMasterKeyPayload {
            public_key: public_pem.clone(),
        };

        let _create_result = create(
            State(state.clone()),
            Path(path_params.clone()),
            Json(payload),
        )
        .await
        .expect("Should create master key");

        // Then list all master keys
        let result = list(State(state), Path(path_params)).await;

        assert!(result.is_ok());
        match result.unwrap() {
            SealboxResponse::Json(json_value) => {
                let keys: Vec<MasterKey> = serde_json::from_value(
                    json_value
                        .get("master_keys")
                        .expect("Should include master_keys")
                        .clone(),
                )
                .expect("Should deserialize Vec<MasterKey>");
                assert_eq!(keys.len(), 1);
                assert_eq!(keys[0].public_key, "[HIDDEN]"); // Public key is hidden in list API for security
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[tokio::test]
    async fn test_active_master_key_returns_public_key() {
        let state = setup_test_state();
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let path_params = MasterKeyPathParams {
            version: Version::V1,
        };

        create(
            State(state.clone()),
            Path(path_params.clone()),
            Json(CreateMasterKeyPayload {
                public_key: public_pem.clone(),
            }),
        )
        .await
        .expect("Should create master key");

        let result = active(State(state), Path(path_params))
            .await
            .expect("Should fetch active master key");

        match result {
            SealboxResponse::Json(json_value) => {
                let master_key: MasterKey =
                    serde_json::from_value(json_value).expect("Should deserialize MasterKey");
                assert_eq!(master_key.public_key, public_pem);
                assert!(matches!(master_key.status, MasterKeyStatus::Active));
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[tokio::test]
    async fn test_create_second_master_key_is_retired() {
        let state = setup_test_state();
        let (_, first_public_pem) = generate_key_pair().expect("Should generate first key pair");
        let (_, second_public_pem) = generate_key_pair().expect("Should generate second key pair");
        let path_params = MasterKeyPathParams {
            version: Version::V1,
        };

        create(
            State(state.clone()),
            Path(path_params.clone()),
            Json(CreateMasterKeyPayload {
                public_key: first_public_pem,
            }),
        )
        .await
        .expect("Should create first master key");

        let result = create(
            State(state),
            Path(path_params),
            Json(CreateMasterKeyPayload {
                public_key: second_public_pem,
            }),
        )
        .await
        .expect("Should create second master key");

        match result {
            SealboxResponse::Json(json_value) => {
                let master_key: MasterKey =
                    serde_json::from_value(json_value).expect("Should deserialize MasterKey");
                assert!(matches!(master_key.status, MasterKeyStatus::Retired));
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[tokio::test]
    async fn test_list_master_keys_invalid_version() {
        let state = setup_test_state();
        let path_params = MasterKeyPathParams {
            version: Version::V2,
        }; // Invalid version

        let result = list(State(state), Path(path_params)).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SealboxError::InvalidApiVersion => {} // Expected
            _ => panic!("Expected InvalidApiVersion error"),
        }
    }

    #[tokio::test]
    async fn test_rotate_master_key_not_found() {
        let state = setup_test_state();
        let old_master_key_id = uuid::Uuid::new_v4();
        let new_master_key_id = uuid::Uuid::new_v4();

        let path_params = MasterKeyPathParams {
            version: Version::V1,
        };
        let payload = RotateMasterKeyPayload {
            old_master_key_id,
            new_master_key_id,
            updates: Vec::new(),
        };

        let result = rotate(State(state), SealboxPath(path_params), Json(payload)).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SealboxError::MasterKeyNotFound(_) => {} // Expected
            _ => panic!("Expected MasterKeyNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_rotate_master_key_invalid_version() {
        let state = setup_test_state();
        let old_master_key_id = uuid::Uuid::new_v4();
        let new_master_key_id = uuid::Uuid::new_v4();

        let path_params = MasterKeyPathParams {
            version: Version::V2,
        }; // Invalid version
        let payload = RotateMasterKeyPayload {
            old_master_key_id,
            new_master_key_id,
            updates: Vec::new(),
        };

        let result = rotate(State(state), SealboxPath(path_params), Json(payload)).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SealboxError::InvalidApiVersion => {} // Expected
            _ => panic!("Expected InvalidApiVersion error"),
        }
    }
}
