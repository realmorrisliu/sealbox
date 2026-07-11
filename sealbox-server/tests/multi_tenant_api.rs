use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

use sealbox_server::{config::SealboxConfig, create_app};

struct TestServer {
    _dir: TempDir,
    app: Router,
    root_token: String,
}

impl TestServer {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root_token = "root-test-token".to_string();
        let config = SealboxConfig {
            auth_token: root_token.clone(),
            store_path: dir.path().join("sealbox.db").display().to_string(),
            listen_addr: "127.0.0.1:0".to_string(),
            legacy_v1_enabled: true,
        };
        let app = create_app(&config).unwrap();
        Self {
            _dir: dir,
            app,
            root_token,
        }
    }

    async fn json(
        &self,
        method: Method,
        path: &str,
        token: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("Authorization", format!("Bearer {token}"));
        if body.is_some() {
            builder = builder.header("Content-Type", "application/json");
        }
        let request = builder
            .body(Body::from(
                body.map(|value| value.to_string()).unwrap_or_default(),
            ))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes).to_string() }));
        (status, value)
    }

    async fn create_tenant(&self, name: &str) -> (String, String) {
        let (status, body) = self
            .json(
                Method::POST,
                "/v2/admin/tenants",
                &self.root_token,
                Some(json!({ "display_name": name, "token_label": "test" })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        (
            body["tenant"]["id"].as_str().unwrap().to_string(),
            body["token"].as_str().unwrap().to_string(),
        )
    }

    async fn register_key(&self, token: &str, public_key: &str) -> String {
        let (status, body) = self
            .json(
                Method::POST,
                "/v2/master-key",
                token,
                Some(json!({ "public_key": public_key })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["id"].as_str().unwrap().to_string()
    }

    async fn save_secret(
        &self,
        token: &str,
        key: &str,
        master_key_id: &str,
        encrypted_data: Vec<u8>,
    ) -> (StatusCode, Value) {
        self.json(
            Method::PUT,
            &format!("/v2/secrets/{key}"),
            token,
            Some(json!({
                "encrypted_data": encrypted_data,
                "encrypted_data_key": [9, 8, 7],
                "master_key_id": master_key_id,
                "ttl": null,
                "metadata": "{\"type\":\"test\"}"
            })),
        )
        .await
    }
}

#[tokio::test]
async fn tenants_isolate_identical_keys_metadata_and_deletion() {
    let server = TestServer::new();
    let (tenant_a, token_a) = server.create_tenant("Tenant A").await;
    let (tenant_b, token_b) = server.create_tenant("Tenant B").await;
    let key_a = server.register_key(&token_a, "public-a").await;
    let key_b = server.register_key(&token_b, "public-b").await;

    let (status_a, _) = server
        .save_secret(&token_a, "same-key", &key_a, vec![1, 1, 1])
        .await;
    let (status_b, _) = server
        .save_secret(&token_b, "same-key", &key_b, vec![2, 2, 2])
        .await;
    assert_eq!(status_a, StatusCode::OK);
    assert_eq!(status_b, StatusCode::OK);

    let (_, value_a) = server
        .json(Method::GET, "/v2/secrets/same-key", &token_a, None)
        .await;
    let (_, value_b) = server
        .json(Method::GET, "/v2/secrets/same-key", &token_b, None)
        .await;
    assert_eq!(value_a["namespace"], tenant_a);
    assert_eq!(value_b["namespace"], tenant_b);
    assert_eq!(value_a["encrypted_data"], json!([1, 1, 1]));
    assert_eq!(value_b["encrypted_data"], json!([2, 2, 2]));

    let (status, _) = server
        .json(Method::DELETE, "/v2/secrets/same-key", &token_a, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = server
        .json(Method::GET, "/v2/secrets/same-key", &token_a, None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = server
        .json(Method::GET, "/v2/secrets/same-key", &token_b, None)
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn tenant_cannot_use_another_tenants_master_key() {
    let server = TestServer::new();
    let (_, token_a) = server.create_tenant("Tenant A").await;
    let (_, token_b) = server.create_tenant("Tenant B").await;
    let key_b = server.register_key(&token_b, "public-b").await;

    let (status, body) = server
        .save_secret(&token_a, "foreign-key", &key_b, vec![1])
        .await;
    assert_eq!(status, StatusCode::PRECONDITION_REQUIRED, "{body}");
    let (status, _) = server
        .json(
            Method::GET,
            &format!("/v2/master-key/by-id/{key_b}"),
            &token_a,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn root_and_legacy_tokens_do_not_cross_authentication_surfaces() {
    let server = TestServer::new();
    let (tenant_id, tenant_token) = server.create_tenant("Tenant").await;

    let (status, _) = server
        .json(Method::GET, "/v2/secrets", &server.root_token, None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = server
        .json(Method::GET, "/v1/secrets", &tenant_token, None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = server
        .json(Method::GET, "/v2/admin/tenants", &tenant_token, None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = server
        .json(
            Method::POST,
            &format!("/v2/admin/tenants/{tenant_id}/suspend"),
            &server.root_token,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = server
        .json(Method::GET, "/v2/secrets", &tenant_token, None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn legacy_v1_routes_can_be_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let config = SealboxConfig {
        auth_token: "root-test-token".to_string(),
        store_path: dir.path().join("sealbox.db").display().to_string(),
        listen_addr: "127.0.0.1:0".to_string(),
        legacy_v1_enabled: false,
    };
    let app = create_app(&config).unwrap();
    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/secrets")
        .header("Authorization", "Bearer root-test-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
