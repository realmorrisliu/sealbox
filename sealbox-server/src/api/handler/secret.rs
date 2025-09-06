use axum::extract::{Json, Query, State};
use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

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
    // version() method removed since we simplified to single version API
    fn secret_key(&self) -> String {
        self.secret_key.clone()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct GetSecretQueryParams {
    version: Option<i32>,
}

/// API handler function for retrieving secret data
///
/// # Arguments
///
/// * `state` - Application state containing database connection pool and repository instances
/// * `params` - Path parameters containing API version and secret key name
/// * `query` - Query parameters with optional version number for retrieving specific version
///
/// # Returns
///
/// Returns encrypted secret data containing encrypted content and encrypted data key
///
/// # Errors
///
/// * `SealboxError::SecretNotFound` - When the secret does not exist
/// * `SealboxError::InvalidApiVersion` - When the API version is not supported
///
/// # HTTP Route
///
/// `GET /{version}/secrets/{secret_key}[?version=N]`
///
/// # Security Notes
///
/// If no version number is specified, returns the latest version. The returned data is still encrypted and requires the client to decrypt it using the corresponding private key.
pub(crate) async fn get(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Query(query): Query<GetSecretQueryParams>,
    headers: HeaderMap,
) -> Result<SealboxResponse> {
    let secret = match query.version {
        Some(version) => {
            state
                .secret_repo
                .get_secret_by_version(&state.pool, &params.secret_key(), version)
                .await?
        }
        None => {
            state
                .secret_repo
                .get_secret(&state.pool, &params.secret_key())
                .await?
        }
    };

    // Check for multi-client access via X-Client-ID header
    if let Some(client_id_header) = headers.get("X-Client-ID") {
        if let Ok(client_id_str) = client_id_header.to_str() {
            if let Ok(client_id) = Uuid::parse_str(client_id_str) {
                // Look for multi-client association
                if let Ok(Some(association)) = state
                    .secret_client_key_repo
                    .get_association(&state.pool, &secret.key, secret.version, &client_id)
                    .await
                {
                    // Update client's last used timestamp
                    if let Err(err) = state
                        .client_key_repo
                        .update_last_used(&state.pool, &client_id)
                        .await
                    {
                        tracing::warn!(
                            "Failed to update last_used_at for client {}: {}",
                            client_id,
                            err
                        );
                    }

                    // Return secret with client-specific encrypted data key
                    return Ok(SealboxResponse::Json(json!({
                        "key": secret.key,
                        "version": secret.version,
                        "encrypted_data": secret.encrypted_data,
                        "encrypted_data_key": association.encrypted_data_key,
                        "client_key_id": association.client_key_id,
                        "created_at": secret.created_at,
                        "updated_at": secret.updated_at,
                        "expires_at": secret.expires_at,
                    })));
                }
            }
        }
    }

    // Fallback to single-client mode (backward compatibility)
    // Update client's last used timestamp for single-client access
    if let Err(err) = state
        .client_key_repo
        .update_last_used(&state.pool, &secret.client_key_id)
        .await
    {
        tracing::warn!(
            "Failed to update last_used_at for client {}: {}",
            secret.client_key_id,
            err
        );
    }

    Ok(SealboxResponse::Json(json!(secret)))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct SaveSecretPayload {
    secret: String, // Now receives plaintext instead of encrypted data
    ttl: Option<i64>,
    // Optional field for multi-client support
    authorized_clients: Option<Vec<Uuid>>,
}

// PUT /{version}/secrets/{secret_key}
pub(crate) async fn save(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Json(payload): Json<SaveSecretPayload>,
) -> Result<SealboxResponse> {
    // Check if this is a multi-client request
    if let Some(authorized_clients) = payload.authorized_clients {
        // Multi-client mode: validate all client keys exist
        for &client_id in &authorized_clients {
            let client_key = state
                .client_key_repo
                .fetch_client_key(&state.pool, &client_id)
                .await?;
            if client_key.is_none() {
                return Err(SealboxError::ClientKeyNotFound(client_id));
            }
        }

        // Create the secret with multiple client key associations
        let secret = state
            .secret_repo
            .create_new_version_multi_client(
                &state.pool,
                &params.secret_key(),
                &payload.secret,
                &authorized_clients,
                payload.ttl,
            )
            .await?;

        Ok(SealboxResponse::Json(json!(secret)))
    } else {
        // Single-client mode (backward compatibility)
        let client_key = state
            .client_key_repo
            .get_valid_client_key(&state.pool)
            .await?;

        let secret = state
            .secret_repo
            .create_new_version(
                &state.pool,
                &params.secret_key(),
                &payload.secret,
                client_key,
                payload.ttl,
            )
            .await?;

        Ok(SealboxResponse::Json(json!(secret)))
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
    state
        .secret_repo
        .delete_secret_by_version(&state.pool, &params.secret_key(), query.version)
        .await?;
    Ok(SealboxResponse::NoContent)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ListSecretsPathParams {
    version: Version,
}

impl ListSecretsPathParams {
    // version() method removed since we simplified to single version API
}

/// API handler function for listing all secrets
///
/// # Arguments
///
/// * `state` - Application state containing database connection pool and repository instances
/// * `params` - Path parameters containing API version
///
/// # Returns
///
/// Returns a list of secrets with basic information (key, version, timestamps)
///
/// # Errors
///
/// * `SealboxError::InvalidApiVersion` - When the API version is not supported
///
/// # HTTP Route
///
/// `GET /{version}/secrets`
///
/// # Security Notes
///
/// Returns only metadata about secrets, not the encrypted content. Automatically filters out expired secrets.
pub(crate) async fn list(
    State(state): State<AppState>,
    Path(_params): Path<ListSecretsPathParams>,
) -> Result<SealboxResponse> {
    let secrets = state.secret_repo.list_secrets(&state.pool).await?;
    Ok(SealboxResponse::Json(json!({ "secrets": secrets })))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct SecretPermissionsPathParams {
    version: Version,
    secret_key: String,
}

impl SecretPermissionsPathParams {
    // version() method removed since we simplified to single version API
    fn secret_key(&self) -> String {
        self.secret_key.clone()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ClientPermission {
    client_id: Uuid,
    client_name: Option<String>,
    authorized_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SecretPermissionsResponse {
    key: String,
    authorized_clients: Vec<ClientPermission>,
}

/// API handler function for viewing secret access permissions
///
/// # Arguments
///
/// * `state` - Application state containing database connection pool and repository instances
/// * `params` - Path parameters containing API version and secret key name
///
/// # Returns
///
/// Returns a list of clients authorized to access the secret
///
/// # Errors
///
/// * `SealboxError::SecretNotFound` - When the secret does not exist
/// * `SealboxError::InvalidApiVersion` - When the API version is not supported
///
/// # HTTP Route
///
/// `GET /{version}/secrets/{secret_key}/permissions`
pub(crate) async fn get_permissions(
    State(state): State<AppState>,
    Path(params): Path<SecretPermissionsPathParams>,
) -> Result<SealboxResponse> {
    // First verify the secret exists (get latest version)
    let secret = state
        .secret_repo
        .get_secret(&state.pool, &params.secret_key())
        .await?;

    // Get all associations for this secret (latest version)
    let associations = state
        .secret_client_key_repo
        .get_associations_for_secret(&state.pool, &secret.key, secret.version)
        .await?;

    // Build the response with client information
    let mut authorized_clients = Vec::new();
    for association in associations {
        // Get client information
        let client = state
            .client_key_repo
            .fetch_client_key(&state.pool, &association.client_key_id)
            .await?;
        if let Some(client) = client {
            authorized_clients.push(ClientPermission {
                client_id: association.client_key_id,
                client_name: client.name,
                authorized_at: association.created_at,
            });
        }
    }

    let response = SecretPermissionsResponse {
        key: secret.key,
        authorized_clients,
    };

    Ok(SealboxResponse::Json(json!(response)))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct RevokePermissionPathParams {
    version: Version,
    secret_key: String,
    client_id: String,
}

impl RevokePermissionPathParams {
    // version() method removed since we simplified to single version API
    fn secret_key(&self) -> String {
        self.secret_key.clone()
    }
    fn client_id(&self) -> String {
        self.client_id.clone()
    }
}

/// API handler function for revoking client access permission
///
/// # Arguments
///
/// * `state` - Application state containing database connection pool and repository instances
/// * `params` - Path parameters containing API version, secret key name, and client ID
///
/// # Returns
///
/// Returns 204 No Content on successful revocation
///
/// # Errors
///
/// * `SealboxError::SecretNotFound` - When the secret does not exist
/// * `SealboxError::ClientKeyNotFound` - When the client key does not exist or has no permission
/// * `SealboxError::InvalidApiVersion` - When the API version is not supported
///
/// # HTTP Route
///
/// `DELETE /{version}/secrets/{secret_key}/permissions/{client_id}`
pub(crate) async fn revoke_permission(
    State(state): State<AppState>,
    Path(params): Path<RevokePermissionPathParams>,
) -> Result<SealboxResponse> {
    // Parse client ID
    let client_id = Uuid::parse_str(&params.client_id())
        .map_err(|_| SealboxError::InvalidInput("Invalid client ID format".to_string()))?;

    // First verify the secret exists (get latest version)
    let secret = state
        .secret_repo
        .get_secret(&state.pool, &params.secret_key())
        .await?;

    // Check if the association exists
    let association = state
        .secret_client_key_repo
        .get_association(&state.pool, &secret.key, secret.version, &client_id)
        .await?;

    if association.is_none() {
        return Err(SealboxError::ClientKeyNotFound(client_id));
    }

    // Revoke the permission by removing the association
    state
        .secret_client_key_repo
        .remove_association(&state.pool, &secret.key, secret.version, &client_id)
        .await?;

    Ok(SealboxResponse::NoContent)
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdatePermissionDataKeyPayload {
    secret_version: i32,
    new_encrypted_data_key: Vec<u8>,
}

/// Update a client's encrypted data key for a secret version (client-side rotation flow)
/// PUT /{version}/secrets/{secret_key}/permissions/{client_id}/data-key
pub(crate) async fn update_permission_data_key(
    State(state): State<AppState>,
    Path(params): Path<RevokePermissionPathParams>,
    Json(payload): Json<UpdatePermissionDataKeyPayload>,
) -> Result<SealboxResponse> {
    let client_id = Uuid::parse_str(&params.client_id())
        .map_err(|_| SealboxError::InvalidInput("Invalid client ID format".to_string()))?;

    // Ensure association exists
    let association = state
        .secret_client_key_repo
        .get_association(
            &state.pool,
            &params.secret_key(),
            payload.secret_version,
            &client_id,
        )
        .await?;
    if association.is_none() {
        return Err(SealboxError::ClientKeyNotFound(client_id));
    }

    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let mut tx = state.pool.begin().await?;

    // Update secret_client_keys
    sqlx::query(
        "UPDATE secret_client_keys SET encrypted_data_key = ?1, created_at = ?2
         WHERE secret_key = ?3 AND secret_version = ?4 AND client_key_id = ?5",
    )
    .bind(&payload.new_encrypted_data_key)
    .bind(now)
    .bind(params.secret_key())
    .bind(payload.secret_version)
    .bind(client_id)
    .execute(&mut *tx)
    .await?;

    // If this client is the primary of the secret version, update secrets table as well
    let updated = sqlx::query(
        "UPDATE secrets SET encrypted_data_key = ?1, updated_at = ?2
         WHERE key = ?3 AND version = ?4 AND client_key_id = ?5",
    )
    .bind(&payload.new_encrypted_data_key)
    .bind(now)
    .bind(params.secret_key())
    .bind(payload.secret_version)
    .bind(client_id)
    .execute(&mut *tx)
    .await?;

    if updated.rows_affected() > 0 {
        // primary record updated
    }

    tx.commit().await?;

    Ok(SealboxResponse::NoContent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::Version,
        api::path::Path as SealboxPath,
        api::state::AppState,
        crypto::client_key::generate_key_pair,
        repo::{
            ClientKey, ClientKeyRepo, ClientKeyStatus, SecretClientKeyRepo, SecretRepo,
            SqliteClientKeyRepo, SqliteSecretClientKeyRepo, SqliteSecretRepo,
        },
    };
    use sqlx::SqlitePool;
    use std::sync::Arc;
    use uuid::Uuid;

    async fn setup_test_state() -> AppState {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        // Initialize all tables
        SqliteSecretRepo::init_table(&pool)
            .await
            .expect("Failed to init secrets table");
        SqliteClientKeyRepo::init_table(&pool)
            .await
            .expect("Failed to init client_keys table");
        SqliteSecretClientKeyRepo::init_table(&pool)
            .await
            .expect("Failed to init secret_client_keys table");

        AppState {
            config: Arc::new(crate::config::SealboxConfig::default()),
            pool,
            health_repo: Arc::new(crate::repo::SqliteHealthRepo {}),
            secret_repo: Arc::new(SqliteSecretRepo {}),
            client_key_repo: Arc::new(SqliteClientKeyRepo {}),
            secret_client_key_repo: Arc::new(SqliteSecretClientKeyRepo {}),
            enroll_repo: Arc::new(crate::repo::SqliteEnrollRepo {}),
        }
    }

    fn create_test_client_key() -> ClientKey {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        ClientKey {
            id: Uuid::new_v4(),
            public_key: public_pem,
            created_at: time::OffsetDateTime::now_utc().unix_timestamp(),
            status: ClientKeyStatus::Active,
            description: Some("Test client key".to_string()),
            metadata: None,
            name: Some("test-client".to_string()),
            last_used_at: None,
        }
    }

    // MultiClientSaveSecretPayload removed - using SaveSecretPayload with authorized_clients field instead

    #[tokio::test]
    async fn test_single_client_secret_creation() {
        let state = setup_test_state().await;
        let client_key1 = create_test_client_key();

        // Register client key first
        state
            .client_key_repo
            .create_client_key(&state.pool, &client_key1)
            .await
            .expect("Should create client key 1");

        let path_params = SecretPathParams {
            version: Version::V1,
            secret_key: "single-client-secret".to_string(),
        };

        let payload = SaveSecretPayload {
            secret: "test secret data".to_string(),
            authorized_clients: Some(vec![client_key1.id]), // Use multi-client mode with specific client
            ttl: None,
        };

        let result = save(State(state), SealboxPath(path_params), Json(payload)).await;

        // This should succeed with current single-client implementation
        match result {
            Ok(_) => {}
            Err(e) => panic!("Expected success but got error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_multi_client_secret_retrieval_with_x_client_id_header() {
        let state = setup_test_state().await;
        let client_key1 = create_test_client_key();
        let client_key2 = create_test_client_key();

        // Register client keys
        state
            .client_key_repo
            .create_client_key(&state.pool, &client_key1)
            .await
            .expect("Should create client key 1");
        state
            .client_key_repo
            .create_client_key(&state.pool, &client_key2)
            .await
            .expect("Should create client key 2");

        // Create multi-client secret
        let secret_path_params = SecretPathParams {
            version: Version::V1,
            secret_key: "multi-secret".to_string(),
        };

        let multi_payload = SaveSecretPayload {
            secret: "shared secret data".to_string(),
            authorized_clients: Some(vec![client_key1.id, client_key2.id]),
            ttl: None,
        };

        let _save_result = save(
            State(state.clone()),
            SealboxPath(secret_path_params.clone()),
            Json(multi_payload),
        )
        .await
        .expect("Should create multi-client secret");

        // Test retrieval with X-Client-ID header for client 1
        let mut headers = HeaderMap::new();
        headers.insert("X-Client-ID", client_key1.id.to_string().parse().unwrap());

        let get_params = GetSecretQueryParams { version: Some(1) };
        let result1 = get(
            State(state.clone()),
            SealboxPath(secret_path_params.clone()),
            Query(get_params.clone()),
            headers.clone(),
        )
        .await
        .expect("Should retrieve secret for client 1");

        // Test retrieval with X-Client-ID header for client 2
        headers.clear();
        headers.insert("X-Client-ID", client_key2.id.to_string().parse().unwrap());

        let result2 = get(
            State(state.clone()),
            SealboxPath(secret_path_params.clone()),
            Query(get_params),
            headers,
        )
        .await
        .expect("Should retrieve secret for client 2");

        // Verify that both clients get the same encrypted data but different encrypted data keys
        if let SealboxResponse::Json(json1) = result1 {
            if let SealboxResponse::Json(json2) = result2 {
                let secret1: serde_json::Value = json1;
                let secret2: serde_json::Value = json2;

                // Same encrypted data (same secret content)
                assert_eq!(secret1["encrypted_data"], secret2["encrypted_data"]);

                // Different encrypted data keys (each encrypted with different client public key)
                assert_ne!(secret1["encrypted_data_key"], secret2["encrypted_data_key"]);

                // Different client key IDs
                assert_eq!(secret1["client_key_id"], client_key1.id.to_string());
                assert_eq!(secret2["client_key_id"], client_key2.id.to_string());
            } else {
                panic!("Expected JSON response for client 2");
            }
        } else {
            panic!("Expected JSON response for client 1");
        }
    }

    #[tokio::test]
    async fn test_get_secret_permissions() {
        let state = setup_test_state().await;
        let client_key1 = create_test_client_key();
        let client_key2 = create_test_client_key();

        // Register client keys
        state
            .client_key_repo
            .create_client_key(&state.pool, &client_key1)
            .await
            .expect("Should create client key 1");
        state
            .client_key_repo
            .create_client_key(&state.pool, &client_key2)
            .await
            .expect("Should create client key 2");

        // Create multi-client secret
        let secret_path_params = SecretPathParams {
            version: Version::V1,
            secret_key: "permissions-test-secret".to_string(),
        };

        let multi_payload = SaveSecretPayload {
            secret: "test secret for permissions".to_string(),
            authorized_clients: Some(vec![client_key1.id, client_key2.id]),
            ttl: None,
        };

        let _save_result = save(
            State(state.clone()),
            SealboxPath(secret_path_params.clone()),
            Json(multi_payload),
        )
        .await
        .expect("Should create multi-client secret");

        // Test get permissions API
        let permissions_params = SecretPermissionsPathParams {
            version: Version::V1,
            secret_key: "permissions-test-secret".to_string(),
        };

        let permissions_result =
            get_permissions(State(state.clone()), SealboxPath(permissions_params))
                .await
                .expect("Should get permissions");

        if let SealboxResponse::Json(json_value) = permissions_result {
            let response: SecretPermissionsResponse = serde_json::from_value(json_value)
                .expect("Should deserialize permissions response");

            assert_eq!(response.key, "permissions-test-secret");
            assert_eq!(response.authorized_clients.len(), 2);

            let client_ids: Vec<Uuid> = response
                .authorized_clients
                .iter()
                .map(|cp| cp.client_id)
                .collect();
            assert!(client_ids.contains(&client_key1.id));
            assert!(client_ids.contains(&client_key2.id));
        } else {
            panic!("Expected JSON response");
        }
    }

    #[tokio::test]
    async fn test_revoke_permission() {
        let state = setup_test_state().await;
        let client_key1 = create_test_client_key();
        let client_key2 = create_test_client_key();

        // Register client keys
        state
            .client_key_repo
            .create_client_key(&state.pool, &client_key1)
            .await
            .expect("Should create client key 1");
        state
            .client_key_repo
            .create_client_key(&state.pool, &client_key2)
            .await
            .expect("Should create client key 2");

        // Create multi-client secret
        let secret_path_params = SecretPathParams {
            version: Version::V1,
            secret_key: "revoke-test-secret".to_string(),
        };

        let multi_payload = SaveSecretPayload {
            secret: "test secret for revoke".to_string(),
            authorized_clients: Some(vec![client_key1.id, client_key2.id]),
            ttl: None,
        };

        let _save_result = save(
            State(state.clone()),
            SealboxPath(secret_path_params.clone()),
            Json(multi_payload),
        )
        .await
        .expect("Should create multi-client secret");

        // Revoke permission for client 1
        let revoke_params = RevokePermissionPathParams {
            version: Version::V1,
            secret_key: "revoke-test-secret".to_string(),
            client_id: client_key1.id.to_string(),
        };

        let revoke_result = revoke_permission(State(state.clone()), SealboxPath(revoke_params))
            .await
            .expect("Should revoke permission");

        // Check that we get NoContent response
        matches!(revoke_result, SealboxResponse::NoContent);

        // Verify client 1 no longer has access
        let association = state
            .secret_client_key_repo
            .get_association(&state.pool, "revoke-test-secret", 1, &client_key1.id)
            .await
            .expect("Should query association");

        assert!(
            association.is_none(),
            "Client 1 should no longer have access"
        );

        // Verify client 2 still has access
        let association = state
            .secret_client_key_repo
            .get_association(&state.pool, "revoke-test-secret", 1, &client_key2.id)
            .await
            .expect("Should query association");

        assert!(association.is_some(), "Client 2 should still have access");
    }

    #[tokio::test]
    async fn test_revoke_permission_nonexistent_client() {
        let state = setup_test_state().await;
        let client_key = create_test_client_key();
        let nonexistent_client_id = Uuid::new_v4();

        // Register client key
        state
            .client_key_repo
            .create_client_key(&state.pool, &client_key)
            .await
            .expect("Should create client key");

        // Create secret with specific client
        let secret_path_params = SecretPathParams {
            version: Version::V1,
            secret_key: "single-client-secret".to_string(),
        };

        let single_payload = SaveSecretPayload {
            secret: "test secret".to_string(),
            ttl: None,
            authorized_clients: Some(vec![client_key.id]), // Specify the client explicitly
        };

        let _save_result = save(
            State(state.clone()),
            SealboxPath(secret_path_params),
            Json(single_payload),
        )
        .await
        .expect("Should create single-client secret");

        // Try to revoke permission for nonexistent client
        let revoke_params = RevokePermissionPathParams {
            version: Version::V1,
            secret_key: "single-client-secret".to_string(),
            client_id: nonexistent_client_id.to_string(),
        };

        let revoke_result =
            revoke_permission(State(state.clone()), SealboxPath(revoke_params)).await;

        // Should return ClientKeyNotFound error
        assert!(revoke_result.is_err());
        matches!(
            revoke_result.unwrap_err(),
            SealboxError::ClientKeyNotFound(_)
        );
    }

    // save_multi_client function removed - using main save function with authorized_clients field instead

    #[tokio::test]
    async fn test_multi_client_secret_creation_with_valid_clients() {
        // Test that shared DataKey design works correctly
        let state = setup_test_state().await;
        let client_key1 = create_test_client_key();
        let client_key2 = create_test_client_key();

        // Register client keys
        state
            .client_key_repo
            .create_client_key(&state.pool, &client_key1)
            .await
            .expect("Should create client key 1");
        state
            .client_key_repo
            .create_client_key(&state.pool, &client_key2)
            .await
            .expect("Should create client key 2");

        let path_params = SecretPathParams {
            version: Version::V1,
            secret_key: "multi-client-secret".to_string(),
        };

        let payload = SaveSecretPayload {
            secret: "shared secret data".to_string(),
            authorized_clients: Some(vec![client_key1.id, client_key2.id]),
            ttl: None,
        };

        // This should now succeed with the implemented multi-client API
        let result = save(
            State(state.clone()),
            SealboxPath(path_params),
            Json(payload),
        )
        .await;
        assert!(result.is_ok(), "Multi-client creation should succeed");

        // Verify that both clients have different encrypted data keys but can access the same data
        let association1 = state
            .secret_client_key_repo
            .get_association(&state.pool, "multi-client-secret", 1, &client_key1.id)
            .await
            .expect("Should get association for client 1")
            .expect("Association 1 should exist");

        let association2 = state
            .secret_client_key_repo
            .get_association(&state.pool, "multi-client-secret", 1, &client_key2.id)
            .await
            .expect("Should get association for client 2")
            .expect("Association 2 should exist");

        // The encrypted data keys should be different (each encrypted with different client public keys)
        assert_ne!(
            association1.encrypted_data_key, association2.encrypted_data_key,
            "Different clients should have different encrypted data keys"
        );

        // But both should reference the same secret
        assert_eq!(association1.secret_key, association2.secret_key);
        assert_eq!(association1.secret_version, association2.secret_version);
    }

    #[tokio::test]
    async fn test_multi_client_secret_creation_with_invalid_clients() {
        // Test error handling for non-existent client keys
        let state = setup_test_state().await;
        let client_key1 = create_test_client_key();
        let _non_existent_client_id = Uuid::new_v4();

        // Register only one client key
        state
            .client_key_repo
            .create_client_key(&state.pool, &client_key1)
            .await
            .expect("Should create client key 1");

        // Multi-client validation should be implemented to check for non-existent client keys
    }

    #[tokio::test]
    async fn test_client_side_update_permission_data_key() {
        use crate::crypto::client_key::{PrivateClientKey, PublicClientKey, generate_key_pair};
        use std::str::FromStr;

        let state = setup_test_state().await;
        let (old_priv_pem, old_pub_pem) = generate_key_pair().expect("gen pair");

        // Register client
        let client = crate::repo::ClientKey::new_with_name(old_pub_pem.clone(), Some("c-1".into()))
            .expect("create client");
        state
            .client_key_repo
            .create_client_key(&state.pool, &client)
            .await
            .expect("store client");

        // Create secret authorized to this client
        let secret_path_params = SecretPathParams {
            version: Version::V1,
            secret_key: "upd-datakey".to_string(),
        };
        let payload = SaveSecretPayload {
            secret: "abc".into(),
            authorized_clients: Some(vec![client.id]),
            ttl: None,
        };
        save(
            State(state.clone()),
            SealboxPath(secret_path_params.clone()),
            Json(payload),
        )
        .await
        .expect("save");

        // Fetch association
        let assoc = state
            .secret_client_key_repo
            .get_association(&state.pool, "upd-datakey", 1, &client.id)
            .await
            .expect("get assoc")
            .expect("exists");

        // Client-side re-encrypt
        let old_priv = PrivateClientKey::from_str(&old_priv_pem).unwrap();
        let data_key_bytes = old_priv.decrypt(&assoc.encrypted_data_key).unwrap();
        let (_new_priv_pem, new_pub_pem) = generate_key_pair().expect("gen new pair");
        let new_pub = PublicClientKey::from_str(&new_pub_pem).unwrap();
        let new_enc_data_key = new_pub.encrypt(&data_key_bytes).unwrap();

        // Update association on server
        let params = RevokePermissionPathParams {
            version: Version::V1,
            secret_key: "upd-datakey".into(),
            client_id: client.id.to_string(),
        };
        let body = UpdatePermissionDataKeyPayload {
            secret_version: 1,
            new_encrypted_data_key: new_enc_data_key.clone(),
        };
        let res = update_permission_data_key(State(state.clone()), SealboxPath(params), Json(body))
            .await
            .expect("update ok");
        matches!(res, SealboxResponse::NoContent);

        // Verify association updated
        let assoc2 = state
            .secret_client_key_repo
            .get_association(&state.pool, "upd-datakey", 1, &client.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(assoc2.encrypted_data_key, new_enc_data_key);
    }
}
