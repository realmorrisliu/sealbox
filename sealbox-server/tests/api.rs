//! HTTP-level tests for behavior that cannot be asserted from a handler in isolation:
//! routing, middleware, and payload rejection.

use axum::body::Body;
use http::{Request, StatusCode, header};
use sealbox_server::{config::SealboxConfig, create_app};
use tower::ServiceExt;

/// A server with its own temporary store and master key. The key file is required at startup,
/// deliberately: sealbox will not invent one.
fn test_config(dir: &tempfile::TempDir) -> SealboxConfig {
    let key_path = dir.path().join("master.pem");
    let (private_pem, _) =
        sealbox_server::crypto::master_key::generate_key_pair().expect("Should generate a key");
    std::fs::write(&key_path, private_pem).expect("Should write the key file");

    SealboxConfig {
        auth_token: "test-token".to_string(),
        store_path: dir.path().join("test.db").to_string_lossy().into_owned(),
        listen_addr: "127.0.0.1:0".to_string(),
        master_key_paths: vec![key_path.to_string_lossy().into_owned()],
    }
}

async fn send(request: Request<Body>) -> http::Response<Body> {
    let dir = tempfile::tempdir().expect("Should create a temp dir");
    let app = create_app(&test_config(&dir)).expect("Should build the app");
    app.oneshot(request).await.expect("Should handle request")
}

#[tokio::test]
async fn no_response_carries_cors_headers() {
    // The CORS layer used to be enabled whenever debug assertions were on, so debug builds
    // behaved differently from release. This test runs in a debug build.
    for (method, uri) in [
        ("GET", "/healthz/live"),
        ("GET", "/v1/secrets"),
        ("OPTIONS", "/v1/secrets"),
    ] {
        let response = send(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::ORIGIN, "https://example.com")
                .body(Body::empty())
                .expect("Should build request"),
        )
        .await;

        let cors: Vec<_> = response
            .headers()
            .keys()
            .filter(|name| name.as_str().starts_with("access-control-"))
            .collect();
        assert!(
            cors.is_empty(),
            "{method} {uri} returned CORS headers: {cors:?}"
        );
    }
}

#[tokio::test]
async fn only_v1_is_routed() {
    for uri in ["/v2/secrets", "/v3/secrets", "/v99/secrets", "/vx/secrets"] {
        let response = send(
            Request::builder()
                .uri(uri)
                .header(header::AUTHORIZATION, "Bearer test-token")
                .body(Body::empty())
                .expect("Should build request"),
        )
        .await;

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{uri} should not be routed, and should be refused identically to a version that \
             never existed"
        );
    }
}

#[tokio::test]
async fn a_rekey_request_carrying_a_private_key_is_rejected() {
    // An old client sending `old_private_key_pem` must fail loudly. Silently ignoring the field
    // would mean a private key crossed the network for nothing, and the caller would never know.
    let (private_pem, _) =
        sealbox_server::crypto::master_key::generate_key_pair().expect("Should generate a key");
    let body = serde_json::json!({
        "new_master_key_id": uuid::Uuid::new_v4(),
        "old_master_key_id": uuid::Uuid::new_v4(),
        "old_private_key_pem": private_pem,
    });

    let response = send(
        Request::builder()
            .method("PUT")
            .uri("/v1/master-key")
            .header(header::AUTHORIZATION, "Bearer test-token")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .expect("Should build request"),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "a payload containing key material must be refused, not quietly accepted"
    );
}

#[tokio::test]
async fn business_endpoints_require_authentication() {
    let response = send(
        Request::builder()
            .uri("/v1/secrets")
            .body(Body::empty())
            .expect("Should build request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn health_probes_need_no_credential() {
    for uri in ["/healthz/live", "/healthz/ready"] {
        let response = send(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("Should build request"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK, "{uri} should be public");
    }
}
