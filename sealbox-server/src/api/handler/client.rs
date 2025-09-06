use axum::extract::{Json, Path, State};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    api::{SealboxResponse, Version, state::AppState},
    error::{Result, SealboxError},
    repo::{ClientKey, ClientKeyStatus},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ClientPathParams {
    version: Version,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ClientIdPathParams {
    version: Version,
    client_id: String,
}

impl ClientIdPathParams {
    fn client_id(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.client_id)
            .map_err(|_| SealboxError::InvalidInput("Invalid client ID format".to_string()))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CreateClientPayload {
    pub name: Option<String>,
    pub public_key: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ClientResponse {
    pub id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub status: ClientKeyStatus,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateClientStatusPayload {
    status: ClientKeyStatus,
}

/// Register new client
/// POST /v1/clients
pub(crate) async fn create(
    State(state): State<AppState>,
    Path(_params): Path<ClientPathParams>,
    Json(payload): Json<CreateClientPayload>,
) -> Result<SealboxResponse> {
    let mut client_key = ClientKey::new_with_name(payload.public_key, payload.name)?;
    client_key.description = payload.description;

    state
        .client_key_repo
        .create_client_key(&state.pool, &client_key)
        .await?;

    let response = ClientResponse {
        id: client_key.id,
        name: client_key.name,
        description: client_key.description,
        created_at: client_key.created_at,
        last_used_at: client_key.last_used_at,
        status: client_key.status,
    };

    Ok(SealboxResponse::Json(json!(response)))
}

/// List all clients
/// GET /v1/clients
pub(crate) async fn list(
    State(state): State<AppState>,
    Path(_params): Path<ClientPathParams>,
) -> Result<SealboxResponse> {
    let client_keys = state
        .client_key_repo
        .fetch_all_client_keys(&state.pool)
        .await?;

    let clients: Vec<ClientResponse> = client_keys
        .into_iter()
        .map(|key| ClientResponse {
            id: key.id,
            name: key.name,
            description: key.description,
            created_at: key.created_at,
            last_used_at: key.last_used_at,
            status: key.status,
        })
        .collect();

    Ok(SealboxResponse::Json(json!({
        "clients": clients
    })))
}

/// Update client status (enable/disable)
/// PUT /v1/clients/{client_id}/status
pub(crate) async fn update_status(
    State(state): State<AppState>,
    Path(params): Path<ClientIdPathParams>,
    Json(payload): Json<UpdateClientStatusPayload>,
) -> Result<SealboxResponse> {
    let client_id = params.client_id()?;

    // Check if client exists
    let client = state
        .client_key_repo
        .fetch_client_key(&state.pool, &client_id)
        .await?;
    if client.is_none() {
        return Err(SealboxError::ClientKeyNotFound(client_id));
    }

    // Update status
    sqlx::query("UPDATE client_keys SET status = ?1 WHERE id = ?2")
        .bind(&payload.status)
        .bind(client_id)
        .execute(&state.pool)
        .await?;

    Ok(SealboxResponse::Json(json!({
        "client_id": client_id,
        "status": payload.status
    })))
}

#[derive(Debug, Deserialize)]
pub(crate) struct RenameClientPayload {
    name: String,
    description: Option<String>,
}

/// Rename/update client basic info
/// PUT /v1/clients/{client_id}/name
pub(crate) async fn rename(
    State(state): State<AppState>,
    Path(params): Path<ClientIdPathParams>,
    Json(payload): Json<RenameClientPayload>,
) -> Result<SealboxResponse> {
    let client_id = params.client_id()?;

    // Ensure client exists
    let exists = state
        .client_key_repo
        .fetch_client_key(&state.pool, &client_id)
        .await?;
    if exists.is_none() {
        return Err(SealboxError::ClientKeyNotFound(client_id));
    }

    sqlx::query(
        "UPDATE client_keys SET name = ?1, description = COALESCE(?2, description) WHERE id = ?3",
    )
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(client_id)
    .execute(&state.pool)
    .await?;

    Ok(SealboxResponse::Json(json!({
        "client_id": client_id,
        "name": payload.name,
        "description": payload.description,
    })))
}
#[derive(Debug, Deserialize)]
pub(crate) struct UpdatePublicKeyPayload {
    new_public_key: String,
}

/// Update client's public key (client-side rotation flow: call after replacing all associations)
/// PUT /v1/clients/{client_id}/public-key
pub(crate) async fn update_public_key(
    State(state): State<AppState>,
    Path(params): Path<ClientIdPathParams>,
    Json(payload): Json<UpdatePublicKeyPayload>,
) -> Result<SealboxResponse> {
    let client_id = params.client_id()?;

    // Ensure client exists
    let exists = state
        .client_key_repo
        .fetch_client_key(&state.pool, &client_id)
        .await?;
    if exists.is_none() {
        return Err(SealboxError::ClientKeyNotFound(client_id));
    }

    sqlx::query("UPDATE client_keys SET public_key = ?1 WHERE id = ?2")
        .bind(&payload.new_public_key)
        .bind(client_id)
        .execute(&state.pool)
        .await?;

    Ok(SealboxResponse::Json(json!({
        "client_id": client_id,
        "updated": true
    })))
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub(crate) struct ClientSecretAssociationItem {
    secret_key: String,
    secret_version: i32,
    encrypted_data_key: Vec<u8>,
}

/// List all secret associations for a client (used by client-side rotation)
/// GET /v1/clients/{client_id}/secrets
pub(crate) async fn list_client_secrets(
    State(state): State<AppState>,
    Path(params): Path<ClientIdPathParams>,
) -> Result<SealboxResponse> {
    let client_id = params.client_id()?;

    // Ensure client exists
    let exists = state
        .client_key_repo
        .fetch_client_key(&state.pool, &client_id)
        .await?;
    if exists.is_none() {
        return Err(SealboxError::ClientKeyNotFound(client_id));
    }

    let rows: Vec<ClientSecretAssociationItem> = sqlx::query_as(
        "SELECT secret_key, secret_version, encrypted_data_key
         FROM secret_client_keys WHERE client_key_id = ?1 ORDER BY secret_key, secret_version",
    )
    .bind(client_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(SealboxResponse::Json(json!({ "associations": rows })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::state::AppState,
        config::SealboxConfig,
        crypto::client_key::generate_key_pair,
        repo::{
            ClientKeyRepo, SecretClientKeyRepo, SecretRepo, SqliteClientKeyRepo, SqliteHealthRepo,
            SqliteSecretClientKeyRepo, SqliteSecretRepo,
        },
    };
    use std::sync::Arc;

    async fn setup_test_state() -> AppState {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Should create in-memory DB");
        SqliteClientKeyRepo::init_table(&pool)
            .await
            .expect("Should init client_keys table");
        SqliteSecretRepo::init_table(&pool)
            .await
            .expect("Should init secrets table");
        SqliteSecretClientKeyRepo::init_table(&pool)
            .await
            .expect("Should init secret_client_keys table");

        AppState {
            pool,
            client_key_repo: Arc::new(SqliteClientKeyRepo),
            secret_repo: Arc::new(SqliteSecretRepo),
            health_repo: Arc::new(SqliteHealthRepo),
            config: Arc::new(SealboxConfig::default()),
            secret_client_key_repo: Arc::new(SqliteSecretClientKeyRepo),
            enroll_repo: Arc::new(crate::repo::SqliteEnrollRepo),
        }
    }

    #[tokio::test]
    async fn test_create_client() {
        let state = setup_test_state().await;
        let (_, public_key_pem) = generate_key_pair().expect("Should generate key pair");

        let payload = CreateClientPayload {
            name: Some("test-client".to_string()),
            public_key: public_key_pem,
            description: Some("Test client description".to_string()),
        };

        let params = ClientPathParams {
            version: Version::V1,
        };

        let response = create(State(state), Path(params), Json(payload))
            .await
            .expect("Should create client");

        if let SealboxResponse::Json(json_value) = response {
            let client: ClientResponse =
                serde_json::from_value(json_value).expect("Should deserialize");
            assert_eq!(client.name, Some("test-client".to_string()));
            assert_eq!(
                client.description,
                Some("Test client description".to_string())
            );
            assert_eq!(client.status, ClientKeyStatus::Active);
        } else {
            panic!("Expected JSON response");
        }
    }

    #[tokio::test]
    async fn test_list_clients() {
        let state = setup_test_state().await;
        let (_, public_key_pem) = generate_key_pair().expect("Should generate key pair");

        // Create a test client first
        let client_key = ClientKey::new_with_name(public_key_pem, Some("test-client".to_string()))
            .expect("Should create client key");

        state
            .client_key_repo
            .create_client_key(&state.pool, &client_key)
            .await
            .expect("Should create client key in DB");

        let params = ClientPathParams {
            version: Version::V1,
        };

        let response = list(State(state), Path(params))
            .await
            .expect("Should list clients");

        if let SealboxResponse::Json(json_value) = response {
            let clients_data: serde_json::Value = json_value;
            let clients = clients_data["clients"]
                .as_array()
                .expect("Should have clients array");
            assert_eq!(clients.len(), 1);

            let client = &clients[0];
            assert_eq!(client["name"], "test-client");
            assert_eq!(client["status"], "Active");
        } else {
            panic!("Expected JSON response");
        }
    }

    // Note: client-side rotation flow is covered via secret handler tests to avoid cross-module private fields.

    // Note: server-side rotate_key endpoint is kept, but its e2e test is omitted here.
}
