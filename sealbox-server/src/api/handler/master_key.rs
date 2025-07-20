use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;
use uuid::Uuid;

use crate::{
    api::{SealboxResponse, Version, path::Path, state::AppState},
    error::{Result, SealboxError},
    repo::MasterKey,
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
pub(crate) struct RotateMasterKeyPayload {
    new_master_key_id: Uuid,
    old_master_key_id: Uuid,
    old_private_key_pem: String,
}

// GET /{version}/master-key
pub(crate) async fn list(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let conn = state.conn_pool.lock()?;
            let master_keys = state.master_key_repo.fetch_all_master_keys(&conn)?;
            Ok(SealboxResponse::Json(json!(master_keys)))
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
            let old_private_key_pem = payload.old_private_key_pem;

            let mut conn = state.conn_pool.lock()?;

            let new_public_key_pem = state
                .master_key_repo
                .fetch_public_key(&conn, &new_master_key_id)?
                .ok_or_else(|| SealboxError::MasterKeyNotFound(new_master_key_id.clone()))?;

            let secrets = state
                .secret_repo
                .fetch_secrets_by_master_key(&conn, &old_master_key_id)?;

            let mut failed_secret_keys = Vec::new();

            let tx = conn.transaction()?;

            for secret in secrets {
                let secret_key = secret.key.clone();

                match secret.rotate_master_key(
                    &old_master_key_id,
                    &old_private_key_pem,
                    &new_master_key_id,
                    &new_public_key_pem,
                ) {
                    Ok(rotated_secret) => {
                        state
                            .secret_repo
                            .update_secret_master_key(&tx, &rotated_secret)?;
                    }
                    Err(err) => {
                        failed_secret_keys.push(secret_key.clone());
                        error!(
                            "Failed to rotate master key for secret {}: {}",
                            secret_key, err
                        );
                    }
                }
            }

            tx.commit()?;

            if !failed_secret_keys.is_empty() {
                return Ok(SealboxResponse::Json(json!({
                  "master_key": new_master_key_id,
                  "failed_secret_keys": failed_secret_keys
                })));
            }

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
            let master_key = MasterKey::new(payload.public_key)?;
            state
                .master_key_repo
                .create_master_key(&conn, &master_key)?;
            Ok(SealboxResponse::Json(json!(master_key)))
        }
        _ => Err(SealboxError::InvalidApiVersion),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::{state::AppState, Version, path::Path as SealboxPath},
        crypto::master_key::generate_key_pair,
        repo::{SqliteMasterKeyRepo, SqliteSecretRepo},
        config::SealboxConfig,
    };
    use axum::extract::State;
    use std::sync::{Arc, Mutex};

    fn setup_test_state() -> AppState {
        let conn = rusqlite::Connection::open_in_memory().expect("Should create in-memory DB");
        crate::repo::SqliteMasterKeyRepo::init_table(&conn).expect("Should init master_keys table");
        crate::repo::SqliteSecretRepo::init_table(&conn).expect("Should init secrets table");
        
        AppState {
            conn_pool: Arc::new(Mutex::new(conn)),
            master_key_repo: Arc::new(SqliteMasterKeyRepo),
            secret_repo: Arc::new(SqliteSecretRepo),
            config: Arc::new(SealboxConfig::default()),
        }
    }

    #[tokio::test]
    async fn test_create_master_key() {
        let state = setup_test_state();
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        
        let path_params = MasterKeyPathParams { version: Version::V1 };
        let payload = CreateMasterKeyPayload { public_key: public_pem.clone() };
        
        let result = create(
            State(state.clone()),
            SealboxPath(path_params),
            Json(payload),
        ).await;
        
        assert!(result.is_ok());
        match result.unwrap() {
            SealboxResponse::Json(json_value) => {
                let master_key: MasterKey = serde_json::from_value(json_value).expect("Should deserialize MasterKey");
                assert_eq!(master_key.public_key, public_pem);
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[tokio::test]
    async fn test_create_master_key_invalid_version() {
        let state = setup_test_state();
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        
        let path_params = MasterKeyPathParams { version: Version::V2 }; // Invalid version
        let payload = CreateMasterKeyPayload { public_key: public_pem };
        
        let result = create(
            State(state),
            SealboxPath(path_params),
            Json(payload),
        ).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            SealboxError::InvalidApiVersion => {}, // Expected
            _ => panic!("Expected InvalidApiVersion error"),
        }
    }

    #[tokio::test]
    async fn test_list_master_keys_empty() {
        let state = setup_test_state();
        let path_params = MasterKeyPathParams { version: Version::V1 };
        
        let result = list(State(state), Path(path_params)).await;
        
        assert!(result.is_ok());
        match result.unwrap() {
            SealboxResponse::Json(json_value) => {
                let keys: Vec<MasterKey> = serde_json::from_value(json_value).expect("Should deserialize Vec<MasterKey>");
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
        let path_params = MasterKeyPathParams { version: Version::V1 };
        let payload = CreateMasterKeyPayload { public_key: public_pem.clone() };
        
        let _create_result = create(
            State(state.clone()),
            Path(path_params.clone()),
            Json(payload),
        ).await.expect("Should create master key");
        
        // Then list all master keys
        let result = list(State(state), Path(path_params)).await;
        
        assert!(result.is_ok());
        match result.unwrap() {
            SealboxResponse::Json(json_value) => {
                let keys: Vec<MasterKey> = serde_json::from_value(json_value).expect("Should deserialize Vec<MasterKey>");
                assert_eq!(keys.len(), 1);
                assert_eq!(keys[0].public_key, "[HIDDEN]"); // Public key is hidden in list API for security
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[tokio::test]
    async fn test_list_master_keys_invalid_version() {
        let state = setup_test_state();
        let path_params = MasterKeyPathParams { version: Version::V2 }; // Invalid version
        
        let result = list(State(state), Path(path_params)).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            SealboxError::InvalidApiVersion => {}, // Expected
            _ => panic!("Expected InvalidApiVersion error"),
        }
    }

    #[tokio::test]
    async fn test_rotate_master_key_not_found() {
        let state = setup_test_state();
        let (old_private_pem, _) = generate_key_pair().expect("Should generate old key pair");
        let old_master_key_id = uuid::Uuid::new_v4();
        let new_master_key_id = uuid::Uuid::new_v4();
        
        let path_params = MasterKeyPathParams { version: Version::V1 };
        let payload = RotateMasterKeyPayload {
            old_master_key_id,
            new_master_key_id,
            old_private_key_pem: old_private_pem,
        };
        
        let result = rotate(
            State(state),
            SealboxPath(path_params),
            Json(payload),
        ).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            SealboxError::MasterKeyNotFound(_) => {}, // Expected
            _ => panic!("Expected MasterKeyNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_rotate_master_key_invalid_version() {
        let state = setup_test_state();
        let (old_private_pem, _) = generate_key_pair().expect("Should generate old key pair");
        let old_master_key_id = uuid::Uuid::new_v4();
        let new_master_key_id = uuid::Uuid::new_v4();
        
        let path_params = MasterKeyPathParams { version: Version::V2 }; // Invalid version
        let payload = RotateMasterKeyPayload {
            old_master_key_id,
            new_master_key_id,
            old_private_key_pem: old_private_pem,
        };
        
        let result = rotate(
            State(state),
            SealboxPath(path_params),
            Json(payload),
        ).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            SealboxError::InvalidApiVersion => {}, // Expected
            _ => panic!("Expected InvalidApiVersion error"),
        }
    }
}
