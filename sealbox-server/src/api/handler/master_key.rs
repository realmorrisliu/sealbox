use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    api::{SealboxResponse, state::AppState},
    error::{Result, SealboxError},
    repo::MasterKey,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
/// Unknown fields are rejected rather than ignored. A client still sending
/// `old_private_key_pem` — an old CLI, a script, a copied example — gets a clear failure
/// instead of silently transmitting its private key to a server that quietly discards it.
#[serde(deny_unknown_fields)]
pub(crate) struct RekeyPayload {
    new_master_key_id: Uuid,
    old_master_key_id: Uuid,
}

// GET /{version}/master-key
pub(crate) async fn list(State(state): State<AppState>) -> Result<SealboxResponse> {
    let master_keys = state.master_key_repo.fetch_all_master_keys()?;
    Ok(SealboxResponse::Json(json!(master_keys)))
}

// PUT /{version}/master-key
pub(crate) async fn rekey(
    State(state): State<AppState>,
    Json(payload): Json<RekeyPayload>,
) -> Result<SealboxResponse> {
    let new_master_key_id = payload.new_master_key_id;
    let old_master_key_id = payload.old_master_key_id;

    // The private half comes from the server's own key files, never from the caller. A key the
    // server does not hold is cold: its secrets cannot be decrypted here by anyone, which is
    // the point of the distinction (ADR 0001).
    let old_private_key = state
        .server_keys
        .get(&old_master_key_id)
        .ok_or(SealboxError::MasterKeyNotServerHeld(old_master_key_id))?;

    let new_key = state
        .master_key_repo
        .fetch_master_key(&new_master_key_id)?
        .ok_or(SealboxError::MasterKeyNotFound(new_master_key_id))?;

    // Rekeying onto a cold key would make every affected secret unreadable by the server. There
    // is no use for that yet, and the cost of doing it by accident is total.
    if !new_key.server_held {
        return Err(SealboxError::MasterKeyNotServerHeld(new_master_key_id));
    }

    let failed_secret_keys = state.secret_repo.rekey_secrets(
        &old_master_key_id,
        old_private_key,
        &new_master_key_id,
        &new_key.public_key,
    )?;

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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CreateMasterKeyPayload {
    public_key: String,
}

// POST /{version}/master-key
pub(crate) async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateMasterKeyPayload>,
) -> Result<SealboxResponse> {
    let master_key = MasterKey::new(payload.public_key)?;
    state.master_key_repo.create_master_key(&master_key)?;
    Ok(SealboxResponse::Json(json!(master_key)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::state::AppState,
        config::SealboxConfig,
        crypto::master_key::generate_key_pair,
        repo::{SqliteHealthRepo, SqliteMasterKeyRepo, SqliteSecretRepo},
    };
    use axum::extract::State;
    use std::sync::{Arc, Mutex};

    fn setup_test_state() -> AppState {
        let conn = rusqlite::Connection::open_in_memory().expect("Should create in-memory DB");
        crate::repo::SqliteMasterKeyRepo::init_table(&conn).expect("Should init master_keys table");
        crate::repo::SqliteSecretRepo::init_table(&conn).expect("Should init secrets table");

        let conn = Arc::new(Mutex::new(conn));
        AppState {
            master_key_repo: Arc::new(SqliteMasterKeyRepo::new(conn.clone())),
            secret_repo: Arc::new(SqliteSecretRepo::new(conn.clone())),
            health_repo: Arc::new(SqliteHealthRepo::new(conn.clone())),
            identity_repo: Arc::new(crate::repo::SqliteIdentityRepo::new(conn.clone())),
            audit_repo: Arc::new(crate::repo::SqliteAuditRepo::new(conn)),
            config: Arc::new(SealboxConfig::default()),
            server_keys: Arc::new(std::collections::HashMap::new()),
            bootstrap_deadline: std::time::Instant::now(),
        }
    }

    #[tokio::test]
    async fn test_create_master_key() {
        let state = setup_test_state();
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");

        let payload = CreateMasterKeyPayload {
            public_key: public_pem.clone(),
        };

        let result = create(State(state.clone()), Json(payload)).await;

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
    async fn test_list_master_keys_empty() {
        let state = setup_test_state();

        let result = list(State(state)).await;

        assert!(result.is_ok());
        match result.unwrap() {
            SealboxResponse::Json(json_value) => {
                let keys: Vec<MasterKey> =
                    serde_json::from_value(json_value).expect("Should deserialize Vec<MasterKey>");
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
        let payload = CreateMasterKeyPayload {
            public_key: public_pem.clone(),
        };

        let _create_result = create(State(state.clone()), Json(payload))
            .await
            .expect("Should create master key");

        // Then list all master keys
        let result = list(State(state)).await;

        assert!(result.is_ok());
        match result.unwrap() {
            SealboxResponse::Json(json_value) => {
                let keys: Vec<MasterKey> =
                    serde_json::from_value(json_value).expect("Should deserialize Vec<MasterKey>");
                assert_eq!(keys.len(), 1);
                assert_eq!(keys[0].public_key, "[HIDDEN]"); // Public key is hidden in list API for security
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[tokio::test]
    async fn test_rekey_refuses_a_source_key_the_server_does_not_hold() {
        // The server's key files are what supply private halves. A source key absent from them
        // is cold, and is refused before anything is read or written — there is no longer any
        // way for a caller to supply the missing key material.
        let state = setup_test_state();
        let old_master_key_id = uuid::Uuid::new_v4();
        let new_master_key_id = uuid::Uuid::new_v4();

        let payload = RekeyPayload {
            old_master_key_id,
            new_master_key_id,
        };

        let result = rekey(State(state), Json(payload)).await;

        match result.unwrap_err() {
            SealboxError::MasterKeyNotServerHeld(id) => assert_eq!(id, old_master_key_id),
            other => panic!("Expected MasterKeyNotServerHeld, got {other:?}"),
        }
    }
}
