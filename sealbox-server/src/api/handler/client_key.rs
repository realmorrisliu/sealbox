use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};
use serde_json::json;
// use tracing::error;
use uuid::Uuid;

use crate::{
    api::{SealboxResponse, Version, path::Path, state::AppState},
    error::Result,
    repo::ClientKey,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ClientKeyPathParams {
    version: Version,
}

impl ClientKeyPathParams {
    // version() method removed since we simplified to single version API
}

// GET /{version}/client-key
pub(crate) async fn list(
    State(state): State<AppState>,
    Path(_params): Path<ClientKeyPathParams>,
) -> Result<SealboxResponse> {
    let client_keys = state
        .client_key_repo
        .fetch_all_client_keys(&state.pool)
        .await?;
    Ok(SealboxResponse::Json(json!(client_keys)))
}

#[derive(Debug, Serialize, Deserialize)]
struct ClientKeyCreateResponse {
    id: Uuid,
    public_key: String,
    created_at: i64,
    status: crate::repo::ClientKeyStatus,
    description: Option<String>,
    metadata: Option<String>,
    name: Option<String>,
    last_used_at: Option<i64>,
}

impl From<ClientKey> for ClientKeyCreateResponse {
    fn from(client_key: ClientKey) -> Self {
        Self {
            id: client_key.id,
            public_key: client_key.public_key, // Show actual public key in create response
            created_at: client_key.created_at,
            status: client_key.status,
            description: client_key.description,
            metadata: client_key.metadata,
            name: client_key.name,
            last_used_at: client_key.last_used_at,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CreateClientKeyPayload {
    public_key: String,
}

// POST /{version}/client-key
pub(crate) async fn create(
    State(state): State<AppState>,
    Path(_params): Path<ClientKeyPathParams>,
    Json(payload): Json<CreateClientKeyPayload>,
) -> Result<SealboxResponse> {
    let client_key = ClientKey::new(payload.public_key)?;
    state
        .client_key_repo
        .create_client_key(&state.pool, &client_key)
        .await?;

    let response = ClientKeyCreateResponse::from(client_key);
    Ok(SealboxResponse::Json(json!(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::{Version, path::Path as SealboxPath, state::AppState},
        config::SealboxConfig,
        crypto::client_key::generate_key_pair,
        repo::{
            ClientKeyRepo, SecretClientKeyRepo, SecretRepo, SqliteClientKeyRepo, SqliteHealthRepo,
            SqliteSecretRepo,
        },
    };
    use axum::extract::State;
    use std::sync::Arc;

    async fn setup_test_state() -> AppState {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Should create in-memory DB");
        crate::repo::SqliteClientKeyRepo::init_table(&pool)
            .await
            .expect("Should init client_keys table");
        crate::repo::SqliteSecretRepo::init_table(&pool)
            .await
            .expect("Should init secrets table");
        crate::repo::SqliteSecretClientKeyRepo::init_table(&pool)
            .await
            .expect("Should init secret_client_keys table");

        AppState {
            pool,
            client_key_repo: Arc::new(SqliteClientKeyRepo),
            secret_repo: Arc::new(SqliteSecretRepo),
            health_repo: Arc::new(SqliteHealthRepo),
            config: Arc::new(SealboxConfig::default()),
            secret_client_key_repo: Arc::new(crate::repo::SqliteSecretClientKeyRepo),
            enroll_repo: Arc::new(crate::repo::SqliteEnrollRepo),
        }
    }

    #[tokio::test]
    async fn test_create_client_key() {
        let state = setup_test_state().await;
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");

        let path_params = ClientKeyPathParams {
            version: Version::V1,
        };
        let payload = CreateClientKeyPayload {
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
                let response: ClientKeyCreateResponse = serde_json::from_value(json_value)
                    .expect("Should deserialize ClientKeyCreateResponse");
                assert_eq!(response.public_key, public_pem);
            }
            _ => panic!("Expected JSON response"),
        }
    }

    // Note: Version validation tests removed since we simplified to single version API

    #[tokio::test]
    async fn test_list_client_keys_empty() {
        let state = setup_test_state().await;
        let path_params = ClientKeyPathParams {
            version: Version::V1,
        };

        let result = list(State(state), Path(path_params)).await;

        assert!(result.is_ok());
        match result.unwrap() {
            SealboxResponse::Json(json_value) => {
                let keys: Vec<ClientKey> =
                    serde_json::from_value(json_value).expect("Should deserialize Vec<ClientKey>");
                assert_eq!(keys.len(), 0);
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[tokio::test]
    async fn test_list_client_keys_with_data() {
        let state = setup_test_state().await;
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");

        // First create a client key
        let path_params = ClientKeyPathParams {
            version: Version::V1,
        };
        let payload = CreateClientKeyPayload {
            public_key: public_pem.clone(),
        };

        let _create_result = create(
            State(state.clone()),
            Path(path_params.clone()),
            Json(payload),
        )
        .await
        .expect("Should create client key");

        // Then list all client keys
        let result = list(State(state), Path(path_params)).await;

        assert!(result.is_ok());
        match result.unwrap() {
            SealboxResponse::Json(json_value) => {
                let keys: Vec<ClientKey> =
                    serde_json::from_value(json_value).expect("Should deserialize Vec<ClientKey>");
                assert_eq!(keys.len(), 1);
                assert_eq!(keys[0].public_key, "[HIDDEN]"); // Public key is hidden in list API for security
            }
            _ => panic!("Expected JSON response"),
        }
    }

    // Note: Version validation test removed since we simplified to single version API

    // Server-side rotation endpoint removed in favor of client-side rotation flow.

    // Note: Version validation test removed since we simplified to single version API
}
