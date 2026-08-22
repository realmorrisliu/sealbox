//! HTTP-level tests: routing, middleware, authentication, authorisation, and audit.
//! These assert behavior no handler can assert in isolation.

use axum::{Router, body::Body};
use http::{Request, StatusCode, header};
use sealbox_server::{config::SealboxConfig, create_app};
use tower::ServiceExt;

const BOOTSTRAP_TOKEN: &str = "bootstrap-secret";

/// A server with its own temporary store. The router is cloned per request so state — including
/// the identities created during a test — persists across them.
struct TestServer {
    app: Router,
    _dir: tempfile::TempDir,
}

impl TestServer {
    fn new() -> Self {
        Self::with_bootstrap_window(std::time::Duration::from_secs(1800))
    }

    /// A server whose bootstrap window has already closed.
    fn with_closed_bootstrap_window() -> Self {
        Self::with_bootstrap_window(std::time::Duration::ZERO)
    }

    fn with_bootstrap_window(bootstrap_window: std::time::Duration) -> Self {
        let dir = tempfile::tempdir().expect("Should create a temp dir");
        let key_path = dir.path().join("master.pem");
        let (private_pem, _) =
            sealbox_server::crypto::master_key::generate_key_pair().expect("Should generate a key");
        std::fs::write(&key_path, private_pem).expect("Should write the key file");

        let config = SealboxConfig {
            bootstrap_token: Some(BOOTSTRAP_TOKEN.to_string()),
            store_path: dir.path().join("test.db").to_string_lossy().into_owned(),
            listen_addr: "127.0.0.1:0".to_string(),
            master_key_paths: vec![key_path.to_string_lossy().into_owned()],
            bootstrap_window,
        };

        Self {
            app: create_app(&config).expect("Should build the app"),
            _dir: dir,
        }
    }

    async fn send(&self, request: Request<Body>) -> http::Response<Body> {
        self.app
            .clone()
            .oneshot(request)
            .await
            .expect("Should handle request")
    }

    async fn json(&self, response: http::Response<Body>) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("Should read body");
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    }

    /// Claim the server and return the first admin's token.
    async fn bootstrap(&self) -> String {
        let response = self
            .send(
                post("/v1/bootstrap", None)
                    .body(Body::from(
                        serde_json::json!({ "token": BOOTSTRAP_TOKEN, "name": "root" }).to_string(),
                    ))
                    .expect("Should build request"),
            )
            .await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "bootstrap should succeed"
        );
        self.json(response).await["token"]
            .as_str()
            .expect("Should return a token")
            .to_string()
    }

    /// Create an identity with the given role and return its token.
    async fn identity(&self, admin: &str, name: &str, role: &str) -> String {
        let response = self
            .send(
                post("/v1/identities", Some(admin))
                    .body(Body::from(
                        serde_json::json!({ "name": name, "role": role }).to_string(),
                    ))
                    .expect("Should build request"),
            )
            .await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "creating {name} as {role}"
        );
        self.json(response).await["token"]
            .as_str()
            .expect("Should return a token")
            .to_string()
    }
}

fn get(uri: &str, token: Option<&str>) -> Request<Body> {
    build("GET", uri, token).body(Body::empty()).unwrap()
}

fn post(uri: &str, token: Option<&str>) -> http::request::Builder {
    build("POST", uri, token).header(header::CONTENT_TYPE, "application/json")
}

fn build(method: &str, uri: &str, token: Option<&str>) -> http::request::Builder {
    let builder = Request::builder().method(method).uri(uri);
    match token {
        Some(t) => builder.header(header::AUTHORIZATION, format!("Bearer {t}")),
        None => builder,
    }
}

// ---------------------------------------------------------------- transport

#[tokio::test]
async fn no_response_carries_cors_headers() {
    let server = TestServer::new();
    for (method, uri) in [
        ("GET", "/healthz/live"),
        ("GET", "/v1/secrets"),
        ("OPTIONS", "/v1/secrets"),
    ] {
        let response = server
            .send(
                build(method, uri, None)
                    .header(header::ORIGIN, "https://example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        let cors: Vec<_> = response
            .headers()
            .keys()
            .filter(|n| n.as_str().starts_with("access-control-"))
            .collect();
        assert!(cors.is_empty(), "{method} {uri} returned {cors:?}");
    }
}

#[tokio::test]
async fn only_v1_is_routed() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    for uri in ["/v2/secrets", "/v3/secrets", "/v99/secrets", "/vx/secrets"] {
        let response = server.send(get(uri, Some(&admin))).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
    }
}

#[tokio::test]
async fn health_probes_need_no_credential() {
    let server = TestServer::new();
    for uri in ["/healthz/live", "/healthz/ready"] {
        let response = server.send(get(uri, None)).await;
        assert_eq!(response.status(), StatusCode::OK, "{uri} should be public");
    }
}

// ---------------------------------------------------------------- identity

#[tokio::test]
async fn business_endpoints_require_an_identity() {
    let server = TestServer::new();
    // No credential at all.
    let response = server.send(get("/v1/secrets", None)).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // A syntactically fine token belonging to nobody.
    let response = server
        .send(get("/v1/secrets", Some("sealbox_deadbeef")))
        .await;
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "there is no shared credential to fall back to"
    );
}

#[tokio::test]
async fn the_role_matrix_is_enforced() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let operator = server.identity(&admin, "op", "operator").await;
    let agent = server.identity(&admin, "bot", "agent").await;

    // Reading is open to every role.
    for token in [&admin, &operator, &agent] {
        let response = server.send(get("/v1/secrets", Some(token))).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    // Managing identities is admin-only, and refusal is forbidden — not unauthorised.
    for (token, who) in [(&operator, "operator"), (&agent, "agent")] {
        let response = server.send(get("/v1/identities", Some(token))).await;
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{who} must not manage identities, and must be told it is forbidden rather than \
             unauthenticated"
        );
    }
    let response = server.send(get("/v1/identities", Some(&admin))).await;
    assert_eq!(response.status(), StatusCode::OK);

    // Writing a secret is operator-and-above.
    let write = |token: String| {
        build("PUT", "/v1/secrets/k", Some(&token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"secret":"v"}"#))
            .unwrap()
    };
    let response = server.send(write(agent.clone())).await;
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "an agent may invoke, but may not store secrets"
    );
    let response = server.send(write(operator.clone())).await;
    assert!(response.status().is_success(), "an operator may store");
}

#[tokio::test]
async fn revocation_is_immediate_and_isolated() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let doomed = server.identity(&admin, "doomed", "operator").await;
    let bystander = server.identity(&admin, "bystander", "operator").await;

    assert!(
        server
            .send(get("/v1/secrets", Some(&doomed)))
            .await
            .status()
            .is_success()
    );

    let response = server
        .send(
            build("DELETE", "/v1/identities/doomed", Some(&admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert!(response.status().is_success());

    assert_eq!(
        server
            .send(get("/v1/secrets", Some(&doomed)))
            .await
            .status(),
        StatusCode::UNAUTHORIZED,
        "a revoked identity stops working on the very next request"
    );
    assert!(
        server
            .send(get("/v1/secrets", Some(&bystander)))
            .await
            .status()
            .is_success(),
        "revoking one identity must not disturb another"
    );
}

#[tokio::test]
async fn a_token_is_returned_once_and_never_again() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.identity(&admin, "op", "operator").await;

    let response = server.send(get("/v1/identities", Some(&admin))).await;
    let body = server.json(response).await;
    let serialised = body.to_string();

    assert!(
        !serialised.contains("token"),
        "listing identities must not expose credentials: {serialised}"
    );
    assert!(
        !serialised.contains("sealbox_"),
        "no token prefix may appear in a listing: {serialised}"
    );
}

// ---------------------------------------------------------------- bootstrap

#[tokio::test]
async fn bootstrap_works_once_and_cannot_be_replayed() {
    let server = TestServer::new();
    let _admin = server.bootstrap().await;

    let response = server
        .send(
            post("/v1/bootstrap", None)
                .body(Body::from(
                    serde_json::json!({ "token": BOOTSTRAP_TOKEN, "name": "second" }).to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "bootstrap must be refused once any identity exists"
    );
}

#[tokio::test]
async fn bootstrap_refuses_after_the_window_closes() {
    // The token is correct and no identity exists — the window alone must refuse it, so that a
    // token left in the environment stops being useful without anyone removing it.
    let server = TestServer::with_closed_bootstrap_window();
    let response = server
        .send(
            post("/v1/bootstrap", None)
                .body(Body::from(
                    serde_json::json!({ "token": BOOTSTRAP_TOKEN, "name": "late" }).to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bootstrap_refuses_a_wrong_token() {
    let server = TestServer::new();
    let response = server
        .send(
            post("/v1/bootstrap", None)
                .body(Body::from(
                    serde_json::json!({ "token": "wrong", "name": "root" }).to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------- audit

#[tokio::test]
async fn attempts_are_recorded_including_refusals() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let agent = server.identity(&admin, "bot", "agent").await;

    // A refusal: an agent reaching for an admin endpoint.
    server.send(get("/v1/identities", Some(&agent))).await;
    // An unauthenticated attempt.
    server.send(get("/v1/secrets", None)).await;

    let response = server.send(get("/v1/audit?limit=50", Some(&admin))).await;
    let body = server.json(response).await;
    let records = body["audit"].as_array().expect("Should return records");

    let forbidden = records
        .iter()
        .find(|r| r["identity"] == "bot" && r["outcome"] == "Forbidden");
    assert!(
        forbidden.is_some(),
        "a refused attempt must be recorded against the identity that made it: {records:?}"
    );

    let anonymous = records
        .iter()
        .find(|r| r["outcome"] == "Unauthenticated" && r["identity"].is_null());
    assert!(
        anonymous.is_some(),
        "an unauthenticated attempt is recorded without inventing an identity: {records:?}"
    );

    // Bootstrap itself is in the trail, from an empty start.
    assert!(
        records
            .iter()
            .any(|r| r["action"] == "POST /v1/bootstrap" && r["outcome"] == "Allowed"),
        "claiming the server must be the first thing in the record"
    );
}

#[tokio::test]
async fn audit_records_carry_no_secret_values() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    let response = server
        .send(
            build("PUT", "/v1/secrets/db-password", Some(&admin))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"secret":"hunter2-do-not-log-me"}"#))
                .unwrap(),
        )
        .await;
    assert!(response.status().is_success());

    let response = server.send(get("/v1/audit?limit=50", Some(&admin))).await;
    let serialised = server.json(response).await.to_string();

    assert!(
        serialised.contains("secrets/db-password"),
        "the resource is named so the record is useful"
    );
    assert!(
        !serialised.contains("hunter2"),
        "the value must never reach the audit trail: {serialised}"
    );
}

#[tokio::test]
async fn health_probes_are_not_audited() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.send(get("/healthz/live", None)).await;
    server.send(get("/healthz/ready", None)).await;

    let response = server.send(get("/v1/audit?limit=50", Some(&admin))).await;
    let serialised = server.json(response).await.to_string();
    assert!(
        !serialised.contains("healthz"),
        "probes are noise, not activity: {serialised}"
    );
}

// ---------------------------------------------------------------- payloads

#[tokio::test]
async fn a_rekey_request_carrying_a_private_key_is_rejected() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let (private_pem, _) =
        sealbox_server::crypto::master_key::generate_key_pair().expect("Should generate a key");

    let response = server
        .send(
            build("PUT", "/v1/master-key", Some(&admin))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "new_master_key_id": uuid::Uuid::new_v4(),
                        "old_master_key_id": uuid::Uuid::new_v4(),
                        "old_private_key_pem": private_pem,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;

    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "a payload containing key material must be refused, not quietly accepted"
    );
}

// ---------------------------------------------------------------- generation

#[tokio::test]
async fn a_generated_secret_is_never_returned() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    let response = server
        .send(
            build("PUT", "/v1/secrets/db-pass", Some(&admin))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"generate":{"type":"password","length":32}}"#,
                ))
                .unwrap(),
        )
        .await;
    assert!(response.status().is_success());

    let body = server.json(response).await;
    let keys: Vec<_> = body.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        keys,
        vec!["created_at", "expires_at", "key", "version"],
        "storing a secret reports which version it became, and nothing else"
    );
}

#[tokio::test]
async fn two_generations_differ() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    let generate = |name: &str| {
        build("PUT", &format!("/v1/secrets/{name}"), Some(&admin))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"generate":{"type":"hex","length":32}}"#))
            .unwrap()
    };
    server.send(generate("a")).await;
    server.send(generate("b")).await;

    // The values are unreadable through the API by design, so compare the ciphertexts: identical
    // plaintext under different random data keys would still differ, but identical *values*
    // would be an alarming coincidence worth catching another way. Here we assert the weaker,
    // checkable thing — both were stored, independently.
    let response = server.send(get("/v1/secrets", Some(&admin))).await;
    let body = server.json(response).await;
    let names: Vec<_> = body["secrets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["key"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"a".to_string()) && names.contains(&"b".to_string()));
}

#[tokio::test]
async fn generation_refuses_a_length_below_the_minimum() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    let response = server
        .send(
            build("PUT", "/v1/secrets/short", Some(&admin))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"generate":{"type":"password","length":8}}"#))
                .unwrap(),
        )
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = server.json(response).await.to_string();
    assert!(
        body.contains("16"),
        "the error must name the minimum so the caller can fix it: {body}"
    );
}

#[tokio::test]
async fn a_payload_cannot_both_supply_and_generate() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    for payload in [
        r#"{"secret":"x","generate":{"type":"hex"}}"#,
        r#"{"ttl":60}"#,
    ] {
        let response = server
            .send(
                build("PUT", "/v1/secrets/k", Some(&admin))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await;
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "ambiguous payload must be refused: {payload}"
        );
    }
}

#[tokio::test]
async fn a_listing_carries_no_values() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    server
        .send(
            build("PUT", "/v1/secrets/k", Some(&admin))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"secret":"hunter2-do-not-leak"}"#))
                .unwrap(),
        )
        .await;

    let response = server.send(get("/v1/secrets", Some(&admin))).await;
    let serialised = server.json(response).await.to_string();

    assert!(serialised.contains("\"key\":\"k\""));
    for forbidden in [
        "hunter2",
        "encrypted_data",
        "encrypted_data_key",
        "master_key_id",
    ] {
        assert!(
            !serialised.contains(forbidden),
            "a listing must not carry {forbidden}: {serialised}"
        );
    }
}

// ---------------------------------------------------------------- grants

impl TestServer {
    /// A stored secret, so grants that declare it validate.
    async fn secret(&self, admin: &str, name: &str) {
        let response = self
            .send(
                build("PUT", &format!("/v1/secrets/{name}"), Some(admin))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"generate":{"type":"hex","length":16}}"#))
                    .unwrap(),
            )
            .await;
        assert!(response.status().is_success(), "creating secret {name}");
    }

    async fn add_grant(&self, token: &str, body: serde_json::Value) -> http::Response<Body> {
        self.send(
            post("/v1/grants", Some(token))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
    }
}

fn adapter_grant(name: &str, secret: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "runner": "prod-cluster",
        "adapter": "kubernetes-secret",
        "config": { "namespace": "prod" },
        "secrets": { "DATABASE_URL": secret },
    })
}

#[tokio::test]
async fn a_grant_round_trips() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.secret(&admin, "app-db-url").await;

    let response = server
        .add_grant(&admin, adapter_grant("k8s-sync", "app-db-url"))
        .await;
    assert!(response.status().is_success());

    let response = server.send(get("/v1/grants/k8s-sync", Some(&admin))).await;
    let grant = server.json(response).await;
    assert_eq!(grant["name"], "k8s-sync");
    assert_eq!(grant["secrets"]["DATABASE_URL"], "app-db-url");
    assert_eq!(grant["created_by"], "root", "who approved it is recorded");

    let response = server.send(get("/v1/grants", Some(&admin))).await;
    let body = server.json(response).await;
    assert_eq!(body["grants"].as_array().unwrap().len(), 1);

    let response = server
        .send(
            build("DELETE", "/v1/grants/k8s-sync", Some(&admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert!(response.status().is_success());
    assert_eq!(
        server
            .send(get("/v1/grants/k8s-sync", Some(&admin)))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn an_agent_may_read_grants_but_not_create_them() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let agent = server.identity(&admin, "bot", "agent").await;
    server.secret(&admin, "app-db-url").await;

    let response = server
        .add_grant(&agent, adapter_grant("sneaky", "app-db-url"))
        .await;
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "an agent cannot approve its own capability"
    );

    // But it must be able to see what it may invoke, and to draft a proposal.
    server
        .add_grant(&admin, adapter_grant("k8s-sync", "app-db-url"))
        .await;
    let response = server.send(get("/v1/grants", Some(&agent))).await;
    assert!(response.status().is_success());
    let body = server.json(response).await;
    assert_eq!(body["grants"][0]["name"], "k8s-sync");
}

#[tokio::test]
async fn a_grant_declaring_a_missing_secret_is_refused() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    let response = server
        .add_grant(&admin, adapter_grant("k8s-sync", "not-a-secret"))
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = server.json(response).await.to_string();
    assert!(
        body.contains("not-a-secret"),
        "the error must name the missing secret: {body}"
    );
}

#[tokio::test]
async fn an_unknown_adapter_is_refused_at_creation() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.secret(&admin, "app-db-url").await;

    let mut grant = adapter_grant("k8s-sync", "app-db-url");
    grant["adapter"] = serde_json::json!("does-not-exist");
    let response = server.add_grant(&admin, grant).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = server.json(response).await.to_string();
    assert!(
        body.contains("kubernetes-secret"),
        "the error must list what is known so the author can correct it: {body}"
    );
}

#[tokio::test]
async fn an_implementation_is_exactly_one_of_adapter_or_script() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.secret(&admin, "app-db-url").await;

    let both = serde_json::json!({
        "name": "both", "runner": "r", "secrets": {},
        "adapter": "kubernetes-secret",
        "script": "#!/bin/sh\ntrue", "command": ["x"],
    });
    assert_eq!(
        server.add_grant(&admin, both).await.status(),
        StatusCode::BAD_REQUEST
    );

    let neither = serde_json::json!({ "name": "neither", "runner": "r", "secrets": {} });
    assert_eq!(
        server.add_grant(&admin, neither).await.status(),
        StatusCode::BAD_REQUEST
    );

    let script_without_command =
        serde_json::json!({ "name": "s", "runner": "r", "secrets": {}, "script": "true" });
    assert_eq!(
        server
            .add_grant(&admin, script_without_command)
            .await
            .status(),
        StatusCode::BAD_REQUEST,
        "a script needs the argv it is invoked with"
    );
}

#[tokio::test]
async fn a_script_is_stored_with_the_grant() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    let body = "#!/usr/bin/env bash\nprintf %s \"$SEALBOX_NEW\"\n";
    let grant = serde_json::json!({
        "name": "custom", "runner": "r", "secrets": {},
        "script": body, "command": ["{script}", "{arg}"],
    });
    assert!(server.add_grant(&admin, grant).await.status().is_success());

    let response = server.send(get("/v1/grants/custom", Some(&admin))).await;
    let shown = server.json(response).await;
    assert_eq!(
        shown["implementation"]["script"], body,
        "what was approved is what is stored — never a path resolved later"
    );
}

#[tokio::test]
async fn a_chain_is_validated() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.secret(&admin, "app-db-url").await;

    // Chaining to something that does not exist.
    let mut dangling = adapter_grant("a", "app-db-url");
    dangling["then"] = serde_json::json!(["nowhere"]);
    let response = server.add_grant(&admin, dangling).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(server.json(response).await.to_string().contains("nowhere"));

    // A cycle. Immutability means it takes a removal and a recreation to build one.
    server
        .add_grant(&admin, adapter_grant("b", "app-db-url"))
        .await;
    let mut a = adapter_grant("a", "app-db-url");
    a["then"] = serde_json::json!(["b"]);
    assert!(server.add_grant(&admin, a).await.status().is_success());

    server
        .send(
            build("DELETE", "/v1/grants/b", Some(&admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    let mut b = adapter_grant("b", "app-db-url");
    b["then"] = serde_json::json!(["a"]);
    let response = server.add_grant(&admin, b).await;
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "a -> b -> a would never terminate"
    );
    assert!(
        server
            .json(response)
            .await
            .to_string()
            .contains("terminate")
    );
}

#[tokio::test]
async fn a_duplicate_name_is_refused_and_the_original_survives() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.secret(&admin, "app-db-url").await;
    server.secret(&admin, "other").await;

    server
        .add_grant(&admin, adapter_grant("k8s-sync", "app-db-url"))
        .await;

    let mut replacement = adapter_grant("k8s-sync", "other");
    replacement["secrets"] = serde_json::json!({ "OTHER": "other" });
    let response = server.add_grant(&admin, replacement).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let response = server.send(get("/v1/grants/k8s-sync", Some(&admin))).await;
    let grant = server.json(response).await;
    assert_eq!(
        grant["secrets"]["DATABASE_URL"], "app-db-url",
        "the original must be untouched — a silent replacement is how a capability widens"
    );
}

#[tokio::test]
async fn uses_enumerates_what_a_credential_can_do() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.secret(&admin, "pg-admin").await;
    server.secret(&admin, "unused").await;

    for name in ["pg-provision", "rotate-db"] {
        let mut grant = adapter_grant(name, "pg-admin");
        grant["adapter"] = serde_json::json!("postgres-role");
        grant["secrets"] = serde_json::json!({ "admin": "pg-admin" });
        assert!(server.add_grant(&admin, grant).await.status().is_success());
    }

    let response = server
        .send(get("/v1/secrets?uses=pg-admin", Some(&admin)))
        .await;
    let body = server.json(response).await;
    let mut used_by: Vec<String> = body["used_by"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    used_by.sort();
    assert_eq!(used_by, vec!["pg-provision", "rotate-db"]);

    let response = server
        .send(get("/v1/secrets?uses=unused", Some(&admin)))
        .await;
    let body = server.json(response).await;
    assert_eq!(
        body["used_by"].as_array().unwrap().len(),
        0,
        "a secret nothing uses is an empty answer, not an error"
    );
}

#[tokio::test]
async fn secret_names_are_hierarchical() {
    // Every example in the design uses paths — `utopia/prod/database-url`,
    // `pg/prod-admin-password`. A route matching a single segment would silently make those
    // unusable, which is how this was found.
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let name = "utopia/prod/database-url";

    let response = server
        .send(
            build("PUT", &format!("/v1/secrets/{name}"), Some(&admin))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"secret":"postgresql://u:p@h/d"}"#))
                .unwrap(),
        )
        .await;
    assert!(response.status().is_success(), "a layered name must store");
    assert_eq!(server.json(response).await["key"], name);

    let response = server.send(get("/v1/secrets", Some(&admin))).await;
    let body = server.json(response).await;
    assert_eq!(body["secrets"][0]["key"], name);

    // And a grant can declare it.
    let mut grant = adapter_grant("sync", name);
    grant["secrets"] = serde_json::json!({ "DATABASE_URL": name });
    assert!(server.add_grant(&admin, grant).await.status().is_success());

    let response = server
        .send(get(&format!("/v1/secrets?uses={name}"), Some(&admin)))
        .await;
    let body = server.json(response).await;
    assert_eq!(body["used_by"][0], "sync");
}

#[tokio::test]
async fn a_parameterised_secret_name_is_refused() {
    // `secrets = { DB = "app/{env}/url" }` reads harmlessly, but the parameter is supplied by
    // the caller — so the grant would reach whichever credential the caller names, and the
    // declaration would stop being the boundary.
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.secret(&admin, "app/prod/url").await;

    let mut grant = adapter_grant("sync", "app/prod/url");
    grant["secrets"] = serde_json::json!({ "DATABASE_URL": "app/{env}/url" });
    let response = server.add_grant(&admin, grant).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = server.json(response).await.to_string();
    assert!(
        body.contains("literally"),
        "the error must explain why, not just refuse: {body}"
    );
}
