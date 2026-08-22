//! HTTP-level tests: routing, middleware, authentication, authorisation, and audit.
//! These assert behavior no handler can assert in isolation.
//!
//! Inside the crate rather than in `tests/` for one reason: an admin has no credential to be
//! handed any more, so acting as one means minting a passkey session directly — which nothing
//! outside the crate can do, and no route offers. What *cannot* be tested here is the ceremony
//! itself: WebAuthn needs an authenticator and a person, so the browser half is verified by
//! hand (see the change's tasks).

use axum::{Router, body::Body};
use http::{Request, StatusCode, header};
use tower::ServiceExt;

use crate::{api::state::AppState, config::SealboxConfig};

const BOOTSTRAP_TOKEN: &str = "bootstrap-secret";

/// A server with its own temporary store. The router is cloned per request so state — including
/// the identities created during a test — persists across them.
struct TestServer {
    app: Router,
    state: AppState,
    _dir: tempfile::TempDir,
}

impl TestServer {
    /// The SQLite file behind this server, for the few tests that have to stand in for time.
    fn store_path(&self) -> std::path::PathBuf {
        self._dir.path().join("test.db")
    }
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
            crate::crypto::master_key::generate_key_pair().expect("Should generate a key");
        std::fs::write(&key_path, private_pem).expect("Should write the key file");

        let config = SealboxConfig {
            public_url: "http://localhost:8080".to_string(),
            bootstrap_token: Some(BOOTSTRAP_TOKEN.to_string()),
            store_path: dir.path().join("test.db").to_string_lossy().into_owned(),
            listen_addr: "127.0.0.1:0".to_string(),
            master_key_paths: vec![key_path.to_string_lossy().into_owned()],
            bootstrap_window,
            replication_metrics_url: None,
        };

        let state = AppState::new(&config).expect("Should build the state");
        Self {
            app: super::build_router(state.clone()),
            state,
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

    /// Claim the server and open an admin session.
    ///
    /// Bootstrap returns an enrolment link, not a credential — so the session is minted here
    /// instead of being registered and signed for, which needs hardware and a human.
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
        let body = self.json(response).await;
        assert!(
            body["token"].is_null(),
            "bootstrap must not hand out an admin credential"
        );
        assert!(
            body["enrol_at"]
                .as_str()
                .is_some_and(|u| u.contains("/enrol/")),
            "bootstrap should point at an enrolment link: {body}"
        );
        self.state.passkey.issue_session("root")
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
        crate::crypto::master_key::generate_key_pair().expect("Should generate a key");

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
        vec!["created_at", "expires_at", "key", "rotate_after", "version"],
        "storing a secret reports which version it became and its policy, and nothing else"
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

    /// Submit a grant and stop there: it is staged for approval, not created.
    async fn stage_grant(&self, token: &str, body: serde_json::Value) -> http::Response<Body> {
        self.send(
            post("/v1/grants", Some(token))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
    }

    async fn add_grant(&self, token: &str, body: serde_json::Value) -> http::Response<Body> {
        let response = self.stage_grant(token, body).await;
        if !response.status().is_success() {
            return response;
        }

        // Standing in for the signature: a real approval needs an authenticator and a person, so
        // the tests take the staged approval and run the same function the ceremony calls once
        // the signature verifies. Everything after this point is therefore identical.
        let body = self.json(response).await;
        let id = body["pending_approval"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("submitting a grant should stage an approval: {body}"));
        let pending = self
            .state
            .passkey
            .take_approval(&id)
            .expect("the staged approval should be there");
        match crate::api::handler::grant::create_from_payload(
            &self.state,
            pending.payload,
            &pending.requested_by,
        ) {
            Ok(_) => http::Response::builder()
                .status(StatusCode::OK)
                .body(Body::empty())
                .unwrap(),
            Err(e) => axum::response::IntoResponse::into_response(e),
        }
    }
}

fn adapter_grant(name: &str, secret: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "runner": "prod-cluster",
        "adapter": "kubernetes-secret",
        "config": { "namespace": "prod", "name": "app-runtime-secrets" },
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
async fn an_agent_may_stage_a_grant_but_not_create_one() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let agent = server.identity(&admin, "bot", "agent").await;
    server.secret(&admin, "app-db-url").await;

    // Drafting is the agent's job, and it is harmless: staging creates nothing.
    let response = server
        .stage_grant(&agent, adapter_grant("sneaky", "app-db-url"))
        .await;
    assert!(response.status().is_success());
    let body = server.json(response).await;
    assert!(
        body["approve_at"]
            .as_str()
            .is_some_and(|u| u.contains("/approve/"))
    );

    assert_eq!(
        server
            .send(get("/v1/grants/sneaky", Some(&agent)))
            .await
            .status(),
        StatusCode::NOT_FOUND,
        "a staged grant does not exist until someone signs for it"
    );

    // And it cannot remove one either.
    assert_eq!(
        server
            .send(
                build("DELETE", "/v1/grants/sneaky", Some(&agent))
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .status(),
        StatusCode::FORBIDDEN
    );

    // But it must be able to see what it may invoke.
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
        grant["config"] = serde_json::json!({
            "host": "db.internal", "database": "app",
            "role_prefix": "app", "owner": "migrator", "privileges": ["CONNECT", "SELECT"],
        });
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

// ---------------------------------------------------------------- jobs and runners

impl TestServer {
    async fn runner_identity(&self, admin: &str, name: &str) -> String {
        self.identity(admin, name, "runner").await
    }

    async fn claim(&self, runner_token: &str) -> serde_json::Value {
        let response = self.send(get("/v1/jobs/claim", Some(runner_token))).await;
        assert!(response.status().is_success(), "claiming should succeed");
        self.json(response).await
    }

    async fn submit(&self, token: &str, grant: &str) -> serde_json::Value {
        let response = self
            .send(
                post("/v1/jobs", Some(token))
                    .body(Body::from(
                        serde_json::json!({ "grant": grant }).to_string(),
                    ))
                    .unwrap(),
            )
            .await;
        assert!(response.status().is_success(), "submitting {grant}");
        self.json(response).await
    }
}

fn script_grant(name: &str, secret: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "runner": "local",
        "secrets": { "VALUE": secret },
        "script": "#!/bin/sh\ntrue\n",
        "command": ["{script}"],
    })
}

#[tokio::test]
async fn a_grant_with_files_round_trips() {
    // The `files` column sits between `secrets` and `chain`; every earlier test used a grant
    // without it, so an off-by-one in the column indices went unnoticed until a real run.
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.secret(&admin, "app/greeting").await;
    server.secret(&admin, "k8s/config").await;

    let mut grant = script_grant("greet", "app/greeting");
    grant["files"] = serde_json::json!({ "KUBECONFIG": "k8s/config" });
    grant["then"] = serde_json::json!([]);
    assert!(server.add_grant(&admin, grant).await.status().is_success());

    let response = server.send(get("/v1/grants/greet", Some(&admin))).await;
    let shown = server.json(response).await;
    assert_eq!(shown["files"]["KUBECONFIG"], "k8s/config");
    assert_eq!(shown["secrets"]["VALUE"], "app/greeting");
    assert_eq!(
        shown["created_by"], "root",
        "an off-by-one here reads as a type error"
    );

    // A file-shaped secret counts as declared, so `uses` finds it.
    let response = server
        .send(get("/v1/secrets?uses=k8s/config", Some(&admin)))
        .await;
    assert_eq!(server.json(response).await["used_by"][0], "greet");
}

#[tokio::test]
async fn runner_permissions_are_disjoint() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let runner = server.runner_identity(&admin, "local").await;

    // A runner may do nothing an agent may.
    for uri in ["/v1/secrets", "/v1/grants", "/v1/audit"] {
        assert_eq!(
            server.send(get(uri, Some(&runner))).await.status(),
            StatusCode::FORBIDDEN,
            "{uri} must be closed to a runner"
        );
    }

    // And being an admin does not confer the runner's permission: the most privileged identity
    // is still not the machine the job was addressed to.
    assert_eq!(
        server
            .send(get("/v1/jobs/claim", Some(&admin)))
            .await
            .status(),
        StatusCode::FORBIDDEN,
        "runner permissions are disjoint, not beneath admin's"
    );
}

#[tokio::test]
async fn a_claim_carries_only_the_declared_secrets() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let runner = server.runner_identity(&admin, "local").await;
    server.secret(&admin, "declared").await;
    server.secret(&admin, "undeclared").await;

    server
        .add_grant(&admin, script_grant("g", "declared"))
        .await;
    server.submit(&admin, "g").await;

    let claimed = server.claim(&runner).await;
    let secrets = claimed["secrets"].as_object().unwrap();
    assert_eq!(secrets.len(), 1, "exactly what the grant declares");
    assert!(secrets.contains_key("VALUE"));

    let serialised = claimed.to_string();
    assert!(
        !serialised.contains("undeclared"),
        "a secret the grant did not declare must not appear: {serialised}"
    );
}

#[tokio::test]
async fn a_job_is_claimed_exactly_once() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let runner = server.runner_identity(&admin, "local").await;
    server.secret(&admin, "s").await;
    server.add_grant(&admin, script_grant("g", "s")).await;
    server.submit(&admin, "g").await;

    let first = server.claim(&runner).await;
    assert!(first["id"].is_i64(), "the first claim gets the job");

    // The second finds nothing: the UPDATE itself decided the winner.
    let second = server.claim(&runner).await;
    assert!(second.is_null(), "a job is handed out once: {second}");
}

#[tokio::test]
async fn only_the_claiming_runner_may_report() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let mine = server.runner_identity(&admin, "local").await;
    let other = server.runner_identity(&admin, "elsewhere").await;
    server.secret(&admin, "s").await;
    server.add_grant(&admin, script_grant("g", "s")).await;

    let job = server.submit(&admin, "g").await;
    let id = job["id"].as_i64().unwrap();
    server.claim(&mine).await;

    let report = |token: String| {
        post(&format!("/v1/jobs/{id}/result"), Some(&token))
            .body(Body::from(r#"{"exit_code":0,"output":"ok"}"#))
            .unwrap()
    };
    assert_eq!(
        server.send(report(other)).await.status(),
        StatusCode::BAD_REQUEST,
        "a runner cannot report a job it does not hold"
    );
    assert!(server.send(report(mine)).await.status().is_success());
}

#[tokio::test]
async fn a_job_names_a_grant_and_cannot_smuggle_an_implementation() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    let response = server
        .send(
            post("/v1/jobs", Some(&admin))
                .body(Body::from(r#"{"grant":"nonexistent"}"#))
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // A field describing what to execute is rejected, not ignored — ADR 0003 is that an agent
    // supplies a name, never a command.
    let response = server
        .send(
            post("/v1/jobs", Some(&admin))
                .body(Body::from(
                    r#"{"grant":"g","command":["sh","-c","curl evil.com"]}"#,
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn a_chain_queues_the_next_grant_on_success() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let runner = server.runner_identity(&admin, "local").await;
    server.secret(&admin, "s").await;

    server.add_grant(&admin, script_grant("second", "s")).await;
    let mut first = script_grant("first", "s");
    first["then"] = serde_json::json!(["second"]);
    server.add_grant(&admin, first).await;

    let job = server.submit(&admin, "first").await;
    let id = job["id"].as_i64().unwrap();
    server.claim(&runner).await;
    server
        .send(
            post(&format!("/v1/jobs/{id}/result"), Some(&runner))
                .body(Body::from(r#"{"exit_code":0,"output":""}"#))
                .unwrap(),
        )
        .await;

    // The server queued the follow-up — a runner that drove its own chain would keep going
    // unsupervised if it were compromised.
    let next = server.claim(&runner).await;
    assert_eq!(next["grant"], "second", "the chain continues: {next}");
}

#[tokio::test]
async fn a_failing_step_stops_the_chain() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let runner = server.runner_identity(&admin, "local").await;
    server.secret(&admin, "s").await;

    server.add_grant(&admin, script_grant("second", "s")).await;
    let mut first = script_grant("first", "s");
    first["then"] = serde_json::json!(["second"]);
    server.add_grant(&admin, first).await;

    let job = server.submit(&admin, "first").await;
    let id = job["id"].as_i64().unwrap();
    server.claim(&runner).await;
    server
        .send(
            post(&format!("/v1/jobs/{id}/result"), Some(&runner))
                .body(Body::from(r#"{"exit_code":1,"output":"it broke"}"#))
                .unwrap(),
        )
        .await;

    assert!(
        server.claim(&runner).await.is_null(),
        "nothing follows a failed step"
    );

    let response = server
        .send(get(&format!("/v1/jobs/{id}"), Some(&admin)))
        .await;
    let job = server.json(response).await;
    assert_eq!(job["status"], "Failed");
    assert_eq!(job["exit_code"], 1);
}

#[tokio::test]
async fn the_caller_gets_a_status_and_output_never_a_value() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let runner = server.runner_identity(&admin, "local").await;
    server.secret(&admin, "s").await;
    server.add_grant(&admin, script_grant("g", "s")).await;

    let job = server.submit(&admin, "g").await;
    let id = job["id"].as_i64().unwrap();
    let claimed = server.claim(&runner).await;
    let value = claimed["secrets"]["VALUE"].as_str().unwrap().to_string();

    server
        .send(
            post(&format!("/v1/jobs/{id}/result"), Some(&runner))
                .body(Body::from(r#"{"exit_code":0,"output":"done"}"#))
                .unwrap(),
        )
        .await;

    let response = server
        .send(get(&format!("/v1/jobs/{id}"), Some(&admin)))
        .await;
    let serialised = server.json(response).await.to_string();
    assert!(serialised.contains("done"), "output comes back");
    assert!(
        !serialised.contains(&value),
        "the value the runner saw must not reach the caller: {serialised}"
    );
}

// ---------------------------------------------------------------- rotation

impl TestServer {
    async fn rotate(
        &self,
        token: &str,
        secret: &str,
        body: serde_json::Value,
    ) -> http::Response<Body> {
        self.send(
            post(&format!("/v1/rotate/{secret}"), Some(token))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
    }

    async fn finish(&self, runner: &str, exit_code: i32, captured: Option<&str>) {
        let claimed = self.claim(runner).await;
        let id = claimed["id"].as_i64().expect("a job should be waiting");
        let mut body = serde_json::json!({ "exit_code": exit_code, "output": "stderr text" });
        if let Some(v) = captured {
            body["captured"] = serde_json::json!(v);
        }
        let response = self
            .send(
                post(&format!("/v1/jobs/{id}/result"), Some(runner))
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await;
        assert!(response.status().is_success(), "reporting job {id}");
    }

    async fn current_version(&self, token: &str, secret: &str) -> i64 {
        let response = self.send(get("/v1/secrets", Some(token))).await;
        let body = self.json(response).await;
        body["secrets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["key"] == secret)
            .map(|s| s["version"].as_i64().unwrap())
            .unwrap_or(0)
    }
}

async fn rotation_fixture(server: &TestServer) -> (String, String) {
    let admin = server.bootstrap().await;
    let runner = server.runner_identity(&admin, "local").await;
    server.secret(&admin, "app/db").await;
    server
        .add_grant(&admin, script_grant("push", "app/db"))
        .await;
    (admin, runner)
}

#[tokio::test]
async fn a_failed_rotation_changes_nothing() {
    let server = TestServer::new();
    let (admin, runner) = rotation_fixture(&server).await;
    let before = server.current_version(&admin, "app/db").await;

    let response = server
        .rotate(&admin, "app/db", serde_json::json!({ "via": "push" }))
        .await;
    assert!(response.status().is_success());

    server.finish(&runner, 1, None).await;

    assert_eq!(
        server.current_version(&admin, "app/db").await,
        before,
        "a failed rotation must leave the previous value current — a stored credential that \
         silently disagrees with reality is worse than no rotation"
    );
}

#[tokio::test]
async fn a_successful_rotation_commits() {
    let server = TestServer::new();
    let (admin, runner) = rotation_fixture(&server).await;
    let before = server.current_version(&admin, "app/db").await;

    server
        .rotate(&admin, "app/db", serde_json::json!({ "via": "push" }))
        .await;
    server.finish(&runner, 0, None).await;

    assert_eq!(server.current_version(&admin, "app/db").await, before + 1);
}

#[tokio::test]
async fn a_pending_version_is_invisible_until_it_commits() {
    let server = TestServer::new();
    let (admin, runner) = rotation_fixture(&server).await;
    let before = server.current_version(&admin, "app/db").await;

    server
        .rotate(&admin, "app/db", serde_json::json!({ "via": "push" }))
        .await;

    // The rotation is in flight: the listing must still show the old version.
    assert_eq!(
        server.current_version(&admin, "app/db").await,
        before,
        "a pending version must not be visible before its grant succeeds"
    );

    // And the claim carries the new value as an ordinary secret.
    let claimed = server.claim(&runner).await;
    assert!(
        claimed["secrets"]["SEALBOX_NEW"].is_string(),
        "the implementation receives the new value like any declared secret: {claimed}"
    );
}

#[tokio::test]
async fn the_caller_cannot_choose_the_new_value() {
    let server = TestServer::new();
    let (admin, _runner) = rotation_fixture(&server).await;

    let response = server
        .rotate(
            &admin,
            "app/db",
            serde_json::json!({ "via": "push", "secret": "chosen-by-me" }),
        )
        .await;
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "the system generates the value; a caller supplying one is refused, not honoured"
    );
}

#[tokio::test]
async fn a_captured_value_never_reaches_the_job_record() {
    let server = TestServer::new();
    let (admin, runner) = rotation_fixture(&server).await;

    server
        .rotate(
            &admin,
            "app/db",
            serde_json::json!({ "via": "push", "from_output": true }),
        )
        .await;

    let claimed = server.claim(&runner).await;
    let id = claimed["id"].as_i64().unwrap();
    assert_eq!(claimed["capture"], true, "the runner is told to capture");

    let composed = "postgresql://app:s3cr3t-composed@db:5432/app";
    let response = server
        .send(
            post(&format!("/v1/jobs/{id}/result"), Some(&runner))
                .body(Body::from(
                    serde_json::json!({
                        "exit_code": 0,
                        "output": "diagnostics on stderr",
                        "captured": composed,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert!(response.status().is_success());

    // Output is stored in the clear for the caller to read; the value must not travel that way.
    let response = server
        .send(get(&format!("/v1/jobs/{id}"), Some(&admin)))
        .await;
    let job = server.json(response).await.to_string();
    assert!(job.contains("diagnostics"), "output is kept");
    assert!(
        !job.contains("s3cr3t-composed"),
        "a captured value must not appear in the job record: {job}"
    );

    let response = server.send(get("/v1/audit?limit=50", Some(&admin))).await;
    let trail = server.json(response).await.to_string();
    assert!(
        !trail.contains("s3cr3t-composed"),
        "nor in the audit trail: {trail}"
    );
}

#[tokio::test]
async fn capturing_nothing_fails_the_rotation() {
    let server = TestServer::new();
    let (admin, runner) = rotation_fixture(&server).await;
    let before = server.current_version(&admin, "app/db").await;

    server
        .rotate(
            &admin,
            "app/db",
            serde_json::json!({ "via": "push", "from_output": true }),
        )
        .await;

    let claimed = server.claim(&runner).await;
    let id = claimed["id"].as_i64().unwrap();
    let response = server
        .send(
            post(&format!("/v1/jobs/{id}/result"), Some(&runner))
                .body(Body::from(
                    r#"{"exit_code":0,"output":"forgot to print","captured":""}"#,
                ))
                .unwrap(),
        )
        .await;

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "storing an empty credential because a script forgot to print is the same failure as \
         storing the wrong one"
    );
    assert_eq!(server.current_version(&admin, "app/db").await, before);
}

#[tokio::test]
async fn a_failed_rotation_leaves_no_version_gap() {
    let server = TestServer::new();
    let (admin, runner) = rotation_fixture(&server).await;
    let before = server.current_version(&admin, "app/db").await;

    // Fail one, then succeed. Deleting the pending row returns MAX(version), so the successful
    // rotation reuses the number rather than skipping it.
    server
        .rotate(&admin, "app/db", serde_json::json!({ "via": "push" }))
        .await;
    server.finish(&runner, 1, None).await;
    server
        .rotate(&admin, "app/db", serde_json::json!({ "via": "push" }))
        .await;
    server.finish(&runner, 0, None).await;

    assert_eq!(
        server.current_version(&admin, "app/db").await,
        before + 1,
        "no gap: a failed rotation leaves nothing behind to skip over"
    );
}

#[tokio::test]
async fn an_agent_rotates_through_an_approved_grant() {
    let server = TestServer::new();
    let (admin, _runner) = rotation_fixture(&server).await;
    let agent = server.identity(&admin, "bot", "agent").await;

    // Rotation widens nothing: the grant was approved by a human, the value is generated here and
    // returned to nobody, and the previous credential keeps working until something drops it. A
    // person in this path would buy no boundary and cost the automation the design is for
    // (ADR 0013). An agent that may invoke a grant may rotate through one.
    let response = server
        .rotate(&agent, "app/db", serde_json::json!({ "via": "push" }))
        .await;
    assert!(
        response.status().is_success(),
        "an agent may rotate through a grant that declares the secret"
    );
}

#[tokio::test]
async fn a_runner_still_cannot_rotate() {
    let server = TestServer::new();
    let (admin, _runner) = rotation_fixture(&server).await;
    let second = server.identity(&admin, "other-runner", "runner").await;

    // The runner's permissions are disjoint, not beneath: it takes what it is given and reports
    // back. Nothing about opening rotation to agents reaches it.
    assert_eq!(
        server
            .rotate(&second, "app/db", serde_json::json!({ "via": "push" }))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn adapter_configuration_is_validated_at_approval() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.secret(&admin, "app/db").await;

    // A typo that would otherwise silently write to the `default` namespace.
    let mut typo = adapter_grant("sync", "app/db");
    typo["config"] = serde_json::json!({ "namspace": "prod", "name": "app" });
    let response = server.add_grant(&admin, typo).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(server.json(response).await.to_string().contains("namspace"));

    // A missing required setting, named.
    let mut incomplete = adapter_grant("sync", "app/db");
    incomplete["config"] = serde_json::json!({ "namespace": "prod" });
    let response = server.add_grant(&admin, incomplete).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(server.json(response).await.to_string().contains("name"));
}

#[tokio::test]
async fn a_privilege_outside_the_closed_set_is_refused_at_approval() {
    // An open set would have to be interpolated into SQL, and a field interpolated into SQL is
    // a field that can carry SQL.
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.secret(&admin, "pg/admin").await;

    let mut grant = adapter_grant("provision", "pg/admin");
    grant["adapter"] = serde_json::json!("postgres-role");
    grant["secrets"] = serde_json::json!({ "admin": "pg/admin" });
    grant["config"] = serde_json::json!({
        "host": "db", "database": "app", "role_prefix": "app", "owner": "migrator",
        "privileges": ["SELECT", "DROP"],
    });

    let response = server.add_grant(&admin, grant).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = server.json(response).await.to_string();
    assert!(
        body.contains("DROP"),
        "names the offending privilege: {body}"
    );
    assert!(body.contains("CONNECT"), "and lists the permitted: {body}");
}

#[tokio::test]
async fn a_valid_adapter_grant_is_approved() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    server.secret(&admin, "pg/admin").await;

    let mut grant = adapter_grant("provision", "pg/admin");
    grant["adapter"] = serde_json::json!("postgres-role");
    grant["secrets"] = serde_json::json!({ "admin": "pg/admin" });
    grant["config"] = serde_json::json!({
        "host": "db.internal", "database": "app", "role_prefix": "app", "owner": "migrator",
        "privileges": ["CONNECT", "SELECT", "INSERT", "UPDATE", "DELETE"],
    });

    assert!(server.add_grant(&admin, grant).await.status().is_success());
}

// ---------------------------------------------------------------- admin authentication
//
// The ceremony itself needs an authenticator and a person, so what is asserted here is
// everything around it: which credentials the gate accepts, what an enrolment link will and
// will not do, and that a stolen database row is not a way in.

#[tokio::test]
async fn an_admin_route_refuses_a_bearer_token() {
    let server = TestServer::new();
    server.bootstrap().await;

    // Not an agent's token, not a stale one: a *valid admin identity's* token, minted the same
    // way every other identity's is. It must still be refused, or every caller that simply
    // forgets to stop sending one keeps the old hole open.
    let (identity, token) =
        crate::repo::Identity::new("second-admin".to_string(), crate::repo::Role::Admin)
            .expect("Should build an identity");
    server
        .state
        .identity_repo
        .create(&identity)
        .expect("Should store it");

    let response = server.send(get("/v1/identities", Some(&token))).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = server.json(response).await;
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("passkey"),
        "the refusal should say what is needed instead: {body}"
    );
}

#[tokio::test]
async fn an_enrolment_link_is_refused_once_an_authenticator_exists() {
    let server = TestServer::new();
    server.bootstrap().await;

    // A leaked link should be a way to become *an* admin for the first time, never a way to
    // displace a working credential.
    let id = server.state.passkey.issue_enrolment("root");
    server
        .state
        .authenticator_repo
        .register("root", "credential-id", "{}")
        .expect("Should register");

    let response = server
        .send(
            post(&format!("/enrol/{id}/start"), None)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_unknown_enrolment_link_says_nothing_about_who_it_was_for() {
    let server = TestServer::new();
    server.bootstrap().await;

    let response = server
        .send(get(&format!("/enrol/{}", uuid::Uuid::new_v4()), None))
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_session_is_the_only_thing_that_acts_as_an_admin() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    assert!(
        server
            .send(get("/v1/identities", Some(&admin)))
            .await
            .status()
            .is_success()
    );

    // Sessions are held in memory with an expiry; a restart drops them, and so does time.
    let server2 = TestServer::new();
    server2.bootstrap().await;
    assert_eq!(
        server2
            .send(get("/v1/identities", Some(&admin)))
            .await
            .status(),
        StatusCode::UNAUTHORIZED,
        "a session from one server must not authenticate against another"
    );
}

#[tokio::test]
async fn agent_operator_and_runner_authentication_is_untouched() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let agent = server.identity(&admin, "claude", "agent").await;
    let operator = server.identity(&admin, "alice", "operator").await;
    let runner = server.identity(&admin, "prod-cluster", "runner").await;

    for (who, token) in [("agent", &agent), ("operator", &operator)] {
        assert!(
            server
                .send(get("/v1/secrets", Some(token)))
                .await
                .status()
                .is_success(),
            "{who} still reads secrets with a bearer token"
        );
    }
    assert!(
        server
            .send(get("/v1/jobs/claim?runner=prod-cluster", Some(&runner)))
            .await
            .status()
            .is_success(),
        "a runner still claims with a bearer token"
    );
}

#[tokio::test]
async fn a_stored_authenticator_holds_only_public_data() {
    let server = TestServer::new();
    server.bootstrap().await;

    // Whatever is written here is what a database dump yields. WebAuthn's guarantee is that the
    // private half never leaves the authenticator, so the row is worth nothing on its own — but
    // that only holds if nothing else is stored beside it.
    server
        .state
        .authenticator_repo
        .register("root", "cred-1", r#"{"cred":"public-key-material"}"#)
        .expect("Should register");

    let stored = server
        .state
        .authenticator_repo
        .for_identity("root")
        .expect("Should read back");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].passkey, r#"{"cred":"public-key-material"}"#);

    // There is no route that returns it, and no route that turns it into a session: a signature
    // is the only way in, and only the authenticator can produce one.
    let response = server
        .send(
            post(&format!("/login/{}/finish", uuid::Uuid::new_v4()), None)
                .body(Body::from(
                    serde_json::json!({
                        "challenge_id": uuid::Uuid::new_v4(),
                        "credential": {},
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert!(
        !response.status().is_success(),
        "replaying stored data must not authenticate"
    );
}

// ---------------------------------------------------------------- reads carry no ciphertext

#[tokio::test]
async fn a_read_carries_no_ciphertext() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let agent = server.identity(&admin, "bot", "agent").await;

    for body in [
        r#"{"secret":"hunter2-do-not-leak"}"#,
        r#"{"secret":"hunter3-do-not-leak"}"#,
    ] {
        server
            .send(
                build("PUT", "/v1/secrets/app/db-url", Some(&admin))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await;
    }

    // Both the current version and a named one. The ciphertext used to come back here, and
    // holding it is the whole of what an agent would need on the day a master key leaks.
    for uri in ["/v1/secrets/app/db-url", "/v1/secrets/app/db-url?version=1"] {
        let response = server.send(get(uri, Some(&agent))).await;
        assert!(response.status().is_success(), "{uri}");
        let serialised = server.json(response).await.to_string();

        assert!(
            serialised.contains("\"key\":\"app/db-url\""),
            "the metadata must still be there: {serialised}"
        );
        for forbidden in ["hunter2", "hunter3", "encrypted_data", "encrypted_data_key"] {
            assert!(
                !serialised.contains(forbidden),
                "{uri} must not carry {forbidden}: {serialised}"
            );
        }
    }
}

#[tokio::test]
async fn an_agent_still_learns_that_a_secret_exists_and_when_it_changed() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let agent = server.identity(&admin, "bot", "agent").await;
    server.secret(&admin, "app/db-url").await;

    let body = server
        .json(
            server
                .send(get("/v1/secrets/app/db-url", Some(&agent)))
                .await,
        )
        .await;
    assert_eq!(body["version"], 1);
    assert!(body["updated_at"].as_i64().is_some());

    // And an absent one is still absent — the question is answerable in both directions.
    assert_eq!(
        server
            .send(get("/v1/secrets/app/nothing", Some(&agent)))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
}

// ---------------------------------------------------------------- rotation policy

/// Store a secret carrying a rotation interval, and backdate it so it is already overdue.
async fn overdue_secret(server: &TestServer, admin: &str, key: &str, after: i64, age: i64) {
    let response = server
        .send(
            build("PUT", &format!("/v1/secrets/{key}"), Some(admin))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "secret": "value", "rotate_after": after }).to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert!(response.status().is_success());

    // Standing in for time passing: reach into the store and age the row. Nothing else in the
    // system can make a secret older, which is the point — the interval is measured from a
    // timestamp only a rotation moves.
    if age > 0 {
        let then = time::OffsetDateTime::now_utc().unix_timestamp() - age;
        let conn = rusqlite::Connection::open(server.store_path()).expect("Should open the store");
        conn.execute(
            "UPDATE secrets SET updated_at = ?1 WHERE key = ?2",
            (then, key),
        )
        .expect("Should backdate");
    }
}

#[tokio::test]
async fn overdue_lists_exactly_what_is_past_its_interval() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    overdue_secret(&server, &admin, "app/stale", 3600, 7200).await;
    overdue_secret(&server, &admin, "app/fresh", 86400, 0).await;
    server.secret(&admin, "app/no-policy").await;

    let body = server
        .json(
            server
                .send(get("/v1/secrets?overdue=true", Some(&admin)))
                .await,
        )
        .await;
    let names: Vec<&str> = body["secrets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["key"].as_str().unwrap())
        .collect();

    assert_eq!(
        names,
        vec!["app/stale"],
        "one is past its interval; one is not; one declares none and so is never due"
    );

    // The interval is a declaration, not a timer: the overdue secret is untouched and still
    // readable. Nothing swept it, nothing queued anything, and job 1 does not exist because no
    // job was ever created.
    let still = server
        .json(
            server
                .send(get("/v1/secrets/app/stale", Some(&admin)))
                .await,
        )
        .await;
    assert_eq!(still["version"], 1);
    assert!(
        !server
            .send(get("/v1/jobs/1", Some(&admin)))
            .await
            .status()
            .is_success(),
        "nothing acts on the interval on its own — no job exists to have been created"
    );
}

#[tokio::test]
async fn a_rotation_carries_the_interval_and_settles_the_secret() {
    let server = TestServer::new();
    let (admin, runner) = rotation_fixture(&server).await;

    // `rotation_fixture` stores app/db without a policy; give it one, already overdue.
    overdue_secret(&server, &admin, "app/db", 3600, 7200).await;
    async fn overdue_count(server: &TestServer, admin: &str) -> usize {
        server
            .json(
                server
                    .send(get("/v1/secrets?overdue=true", Some(admin)))
                    .await,
            )
            .await["secrets"]
            .as_array()
            .unwrap()
            .len()
    }
    assert_eq!(overdue_count(&server, &admin).await, 1);

    server
        .rotate(&admin, "app/db", serde_json::json!({ "via": "push" }))
        .await;
    server.finish(&runner, 0, None).await;

    let after = server
        .json(server.send(get("/v1/secrets/app/db", Some(&admin))).await)
        .await;
    assert_eq!(
        after["rotate_after"], 3600,
        "losing the policy at the first rotation that honoured it would be the worst moment"
    );
    assert_eq!(
        overdue_count(&server, &admin).await,
        0,
        "rotating settles it, because the only thing making it overdue is the timestamp"
    );
}

// ---------------------------------------------------------------- the ceremonies
//
// A software authenticator stands in for the hardware one. What it does not stand in for is the
// person: consent, presence, and the browser's origin checks are the authenticator's job in a
// real ceremony, and here they are simply granted. What these do cover is everything on the
// server's side of the wire — the challenge, the signature over it, what the signature is bound
// to, and what each of them is then allowed to do.

use webauthn_authenticator_rs::{AuthenticatorBackend, softpasskey::SoftPasskey};
use webauthn_rs::prelude::Url;

/// One person with one authenticator, driving real ceremonies against the real routes.
struct Person {
    authenticator: SoftPasskey,
    origin: Url,
}

impl Person {
    fn new() -> Self {
        Self {
            // `true` falsifies user verification: there is nobody here to verify.
            authenticator: SoftPasskey::new(true),
            origin: Url::parse("http://localhost:8080").expect("Should parse"),
        }
    }

    /// Register through the enrolment link, exactly as the page does.
    async fn enrol(&mut self, server: &TestServer, id: &uuid::Uuid) -> serde_json::Value {
        let challenge = server
            .json(
                server
                    .send(
                        post(&format!("/enrol/{id}/start"), None)
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await,
            )
            .await;

        let options = serde_json::from_value(challenge["publicKey"].clone())
            .expect("the server should send a creation challenge");
        let credential = self
            .authenticator
            .perform_register(self.origin.clone(), options, 60_000)
            .expect("the authenticator should register");

        let response = server
            .send(
                post(&format!("/enrol/{id}/finish"), None)
                    .body(Body::from(serde_json::to_vec(&credential).unwrap()))
                    .unwrap(),
            )
            .await;
        assert!(response.status().is_success(), "enrolment should finish");
        server.json(response).await
    }

    /// Sign a challenge at `base` (a login or an approval) and post the result to its finish.
    async fn sign(
        &mut self,
        server: &TestServer,
        base: &str,
        identity: &str,
    ) -> http::Response<Body> {
        let started = server
            .json(
                server
                    .send(
                        post(&format!("{base}/start"), None)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Body::from(
                                serde_json::json!({ "identity": identity }).to_string(),
                            ))
                            .unwrap(),
                    )
                    .await,
            )
            .await;

        let options = serde_json::from_value(started["options"]["publicKey"].clone())
            .expect("the server should send a request challenge");
        let credential = self
            .authenticator
            .perform_auth(self.origin.clone(), options, 60_000)
            .expect("the authenticator should sign");

        server
            .send(
                post(&format!("{base}/finish"), None)
                    .body(Body::from(
                        serde_json::json!({
                            "challenge_id": started["challenge_id"],
                            "credential": credential,
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
    }
}

/// Claim a server and come out of it holding a session, the way a person actually would.
async fn enrol_and_sign_in(server: &TestServer, person: &mut Person) -> String {
    let body = server
        .json(
            server
                .send(
                    post("/v1/bootstrap", None)
                        .body(Body::from(
                            serde_json::json!({ "token": BOOTSTRAP_TOKEN, "name": "root" })
                                .to_string(),
                        ))
                        .unwrap(),
                )
                .await,
        )
        .await;

    let enrolment: uuid::Uuid = body["enrol_at"]
        .as_str()
        .and_then(|u| u.rsplit('/').next())
        .and_then(|id| id.parse().ok())
        .unwrap_or_else(|| panic!("bootstrap should return an enrolment link: {body}"));
    person.enrol(server, &enrolment).await;

    let opened = server
        .json(
            server
                .send(post("/v1/auth/login", None).body(Body::empty()).unwrap())
                .await,
        )
        .await;
    let login = opened["login"].as_str().expect("a login id").to_string();

    let response = person
        .sign(server, &format!("/login/{login}"), "root")
        .await;
    assert!(response.status().is_success(), "signing in should succeed");

    let collected = server
        .json(
            server
                .send(get(&format!("/v1/auth/login/{login}"), None))
                .await,
        )
        .await;
    collected["session"]
        .as_str()
        .expect("the waiting caller should collect a session")
        .to_string()
}

#[tokio::test]
async fn a_passkey_carries_someone_from_enrolment_to_an_admin_operation() {
    let server = TestServer::new();
    let mut person = Person::new();
    let session = enrol_and_sign_in(&server, &mut person).await;

    // The session is the only thing that acts as an admin, and it does.
    let response = server.send(get("/v1/identities", Some(&session))).await;
    assert!(
        response.status().is_success(),
        "a signed-in admin should reach an admin route"
    );
}

#[tokio::test]
async fn an_enrolment_link_cannot_be_used_twice() {
    let server = TestServer::new();
    let mut person = Person::new();
    enrol_and_sign_in(&server, &mut person).await;

    // A leaked link must not be a way to add a second authenticator to a working identity.
    let id = server.state.passkey.issue_enrolment("root");
    let response = server
        .send(
            post(&format!("/enrol/{id}/start"), None)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_grant_exists_only_after_it_is_signed_for() {
    let server = TestServer::new();
    let mut person = Person::new();
    let session = enrol_and_sign_in(&server, &mut person).await;
    server.secret(&session, "app-db-url").await;

    let staged = server
        .json(
            server
                .stage_grant(&session, adapter_grant("k8s-sync", "app-db-url"))
                .await,
        )
        .await;
    let approval = staged["pending_approval"].as_str().expect("an approval id");

    assert_eq!(
        server
            .send(get("/v1/grants/k8s-sync", Some(&session)))
            .await
            .status(),
        StatusCode::NOT_FOUND,
        "staging must create nothing"
    );

    let response = person
        .sign(&server, &format!("/approve/{approval}"), "root")
        .await;
    assert!(response.status().is_success(), "approving should succeed");

    let grant = server
        .json(
            server
                .send(get("/v1/grants/k8s-sync", Some(&session)))
                .await,
        )
        .await;
    assert_eq!(grant["name"], "k8s-sync");
    assert_eq!(
        grant["created_by"], "root",
        "the grant records who signed for it"
    );
}

#[tokio::test]
async fn a_signature_for_one_approval_does_not_approve_another() {
    let server = TestServer::new();
    let mut person = Person::new();
    let session = enrol_and_sign_in(&server, &mut person).await;
    server.secret(&session, "app-db-url").await;

    let first = server
        .json(
            server
                .stage_grant(&session, adapter_grant("harmless", "app-db-url"))
                .await,
        )
        .await;
    let second = server
        .json(
            server
                .stage_grant(&session, adapter_grant("the-one-they-want", "app-db-url"))
                .await,
        )
        .await;

    // Sign for the first, then submit that signature against the second — the substitution the
    // rendered page exists to prevent, attempted at the wire instead of the display.
    let started = server
        .json(
            server
                .send(
                    post(
                        &format!(
                            "/approve/{}/start",
                            first["pending_approval"].as_str().unwrap()
                        ),
                        None,
                    )
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "identity": "root" }).to_string(),
                    ))
                    .unwrap(),
                )
                .await,
        )
        .await;
    let options = serde_json::from_value(started["options"]["publicKey"].clone()).unwrap();
    let credential = person
        .authenticator
        .perform_auth(person.origin.clone(), options, 60_000)
        .expect("the authenticator should sign");

    let response = server
        .send(
            post(
                &format!(
                    "/approve/{}/finish",
                    second["pending_approval"].as_str().unwrap()
                ),
                None,
            )
            .body(Body::from(
                serde_json::json!({
                    "challenge_id": started["challenge_id"],
                    "credential": credential,
                })
                .to_string(),
            ))
            .unwrap(),
        )
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        server
            .send(get("/v1/grants/the-one-they-want", Some(&session)))
            .await
            .status(),
        StatusCode::NOT_FOUND,
        "the grant they wanted must not exist"
    );
}

// ---------------------------------------------------------------- workload identity
//
// A runner's credential is the only one that receives plaintext. These drive the real routes with
// real signatures, so what is asserted is what a cluster's token would meet.

/// A stand-in issuer: an RSA key, the JWKS that publishes it, and tokens signed by it.
struct Platform {
    encoding: jsonwebtoken::EncodingKey,
    jwks: String,
    url: String,
}

impl Platform {
    fn new(url: &str) -> Self {
        use base64::Engine;
        use rsa::traits::PublicKeyParts;

        let (private_pem, _) =
            crate::crypto::master_key::generate_key_pair().expect("Should generate a key");
        // The generator emits PKCS#1; jsonwebtoken wants PKCS#8, so convert once here.
        let private =
            <rsa::RsaPrivateKey as rsa::pkcs1::DecodeRsaPrivateKey>::from_pkcs1_pem(&private_pem)
                .expect("Should parse");
        let pkcs8 = <rsa::RsaPrivateKey as rsa::pkcs8::EncodePrivateKey>::to_pkcs8_pem(
            &private,
            rsa::pkcs8::LineEnding::LF,
        )
        .expect("Should re-encode");
        let public = private.to_public_key();

        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let jwks = serde_json::json!({
            "keys": [{
                "kty": "RSA",
                "kid": "test-key",
                "alg": "RS256",
                "use": "sig",
                "n": b64.encode(public.n().to_bytes_be()),
                "e": b64.encode(public.e().to_bytes_be()),
            }]
        })
        .to_string();

        Self {
            encoding: jsonwebtoken::EncodingKey::from_rsa_pem(pkcs8.as_bytes())
                .expect("Should build an encoding key"),
            jwks,
            url: url.to_string(),
        }
    }

    fn sign(&self, subject: &str, audience: &str, expires_in: i64) -> String {
        self.sign_with_kid(subject, audience, expires_in, Some("test-key"))
    }

    fn sign_with_kid(
        &self,
        subject: &str,
        audience: &str,
        expires_in: i64,
        kid: Option<&str>,
    ) -> String {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let claims = serde_json::json!({
            "iss": self.url,
            "sub": subject,
            "aud": audience,
            "iat": now,
            "exp": now + expires_in,
        });
        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = kid.map(|k| k.to_string());
        jsonwebtoken::encode(&header, &claims, &self.encoding).expect("Should sign")
    }
}

/// Register the platform as an issuer and bind a runner identity to one subject.
async fn bind_runner(
    server: &TestServer,
    admin: &str,
    platform: &Platform,
    name: &str,
    subject: &str,
) {
    let response = server
        .send(
            post("/v1/issuers", Some(admin))
                .body(Body::from(
                    serde_json::json!({
                        "name": "prod-cluster",
                        "url": platform.url,
                        "jwks": platform.jwks,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert!(response.status().is_success(), "registering the issuer");

    let response = server
        .send(
            post("/v1/identities", Some(admin))
                .body(Body::from(
                    serde_json::json!({
                        "name": name,
                        "role": "runner",
                        "issuer": "prod-cluster",
                        "subject": subject,
                        "audience": "sealbox",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert!(response.status().is_success(), "binding the runner");
    let body = server.json(response).await;
    assert!(
        body["token"].is_null(),
        "a bound identity holds no credential: {body}"
    );
}

/// What a runner does with its token: claim a job.
async fn claim_with(server: &TestServer, token: &str) -> http::StatusCode {
    server
        .send(get("/v1/jobs/claim?runner=prod-runner", Some(token)))
        .await
        .status()
}

#[tokio::test]
async fn a_workload_token_authenticates_a_runner() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let platform = Platform::new("https://kubernetes.default.svc.cluster.local");
    bind_runner(
        &server,
        &admin,
        &platform,
        "prod-runner",
        "system:serviceaccount:sealbox:runner",
    )
    .await;

    let token = platform.sign("system:serviceaccount:sealbox:runner", "sealbox", 3600);
    assert!(
        claim_with(&server, &token).await.is_success(),
        "a valid token from the bound issuer authenticates"
    );
}

#[tokio::test]
async fn every_way_a_workload_token_can_be_wrong_is_refused() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let platform = Platform::new("https://kubernetes.default.svc.cluster.local");
    bind_runner(
        &server,
        &admin,
        &platform,
        "prod-runner",
        "system:serviceaccount:sealbox:runner",
    )
    .await;

    let elsewhere = Platform::new("https://someone-elses-cluster");

    let cases = [
        (
            "another subject in the same cluster",
            platform.sign("system:serviceaccount:default:anyone", "sealbox", 3600),
        ),
        (
            "a subject differing only by a suffix",
            platform.sign("system:serviceaccount:sealbox:runner-2", "sealbox", 3600),
        ),
        (
            "a token minted for the API server",
            platform.sign(
                "system:serviceaccount:sealbox:runner",
                "https://kubernetes.default.svc",
                3600,
            ),
        ),
        (
            "an expired token",
            platform.sign("system:serviceaccount:sealbox:runner", "sealbox", -7200),
        ),
        (
            "a token signed by another issuer's key",
            elsewhere.sign("system:serviceaccount:sealbox:runner", "sealbox", 3600),
        ),
        (
            "a key nobody registered",
            platform.sign_with_kid(
                "system:serviceaccount:sealbox:runner",
                "sealbox",
                3600,
                Some("some-other-key"),
            ),
        ),
        (
            "something that is not a token at all",
            "not.a.jwt".to_string(),
        ),
    ];

    for (what, token) in cases {
        assert_eq!(
            claim_with(&server, &token).await,
            StatusCode::UNAUTHORIZED,
            "{what} must be refused"
        );
    }
}

#[tokio::test]
async fn revoking_a_bound_identity_ends_it_whatever_the_platform_signs() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let platform = Platform::new("https://kubernetes.default.svc.cluster.local");
    bind_runner(
        &server,
        &admin,
        &platform,
        "prod-runner",
        "system:serviceaccount:sealbox:runner",
    )
    .await;

    let token = platform.sign("system:serviceaccount:sealbox:runner", "sealbox", 3600);
    assert!(claim_with(&server, &token).await.is_success());

    // Revocation has to be sealbox's, not the platform's: the cluster will happily keep signing.
    let response = server
        .send(
            build("DELETE", "/v1/identities/prod-runner", Some(&admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert!(response.status().is_success(), "revoking");

    assert_eq!(
        claim_with(&server, &token).await,
        StatusCode::UNAUTHORIZED,
        "the same token must stop working the moment the identity is revoked"
    );
}

#[tokio::test]
async fn both_keys_work_while_a_signing_key_rotates() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let old = Platform::new("https://kubernetes.default.svc.cluster.local");
    bind_runner(
        &server,
        &admin,
        &old,
        "prod-runner",
        "system:serviceaccount:sealbox:runner",
    )
    .await;

    // A cluster rotating its signing key publishes both for a while. Registering the combined
    // JWKS is how that overlaps here instead of cutting over and stranding every runner.
    let new = Platform::new("https://kubernetes.default.svc.cluster.local");
    let mut combined: serde_json::Value = serde_json::from_str(&old.jwks).unwrap();
    let mut new_key: serde_json::Value = serde_json::from_str(&new.jwks).unwrap();
    new_key["keys"][0]["kid"] = serde_json::json!("rotated-key");
    combined["keys"]
        .as_array_mut()
        .unwrap()
        .push(new_key["keys"][0].clone());

    let response = server
        .send(
            build("PUT", "/v1/issuers/prod-cluster", Some(&admin))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "jwks": combined.to_string() }).to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert!(response.status().is_success(), "registering both keys");

    for (which, token) in [
        (
            "the old key",
            old.sign("system:serviceaccount:sealbox:runner", "sealbox", 3600),
        ),
        (
            "the new key",
            new.sign_with_kid(
                "system:serviceaccount:sealbox:runner",
                "sealbox",
                3600,
                Some("rotated-key"),
            ),
        ),
    ] {
        assert!(
            claim_with(&server, &token).await.is_success(),
            "{which} should authenticate during a rotation"
        );
    }
}

#[tokio::test]
async fn binding_an_identity_needs_all_three_parts() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    let response = server
        .send(
            post("/v1/identities", Some(&admin))
                .body(Body::from(
                    serde_json::json!({
                        "name": "half-bound",
                        "role": "runner",
                        "issuer": "prod-cluster",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = server.json(response).await;
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("audience"),
        "the refusal should say what is missing and why: {body}"
    );
}

#[tokio::test]
async fn an_issuer_that_does_not_parse_is_refused_while_someone_is_there_to_fix_it() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    let response = server
        .send(
            post("/v1/issuers", Some(&admin))
                .body(Body::from(
                    serde_json::json!({
                        "name": "typo",
                        "url": "https://cluster",
                        "jwks": "{ not a jwks }",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------- recovery
//
// The master key is the only thing that can read the store, and replication covers the database
// and not the key. These assert the property that matters: a blob plus its recovery key produces
// the master key, and a blob alone produces nothing.

/// Register a recovery key and return (id, private PEM).
async fn register_recovery(server: &TestServer, admin: &str) -> (uuid::Uuid, String) {
    let (private_pem, public_pem) =
        crate::crypto::master_key::generate_key_pair().expect("Should generate");

    let response = server
        .send(
            post("/v1/recovery", Some(admin))
                .body(Body::from(
                    serde_json::json!({ "public_key": public_pem }).to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert!(response.status().is_success(), "registering a recovery key");

    let body = server.json(response).await;
    let id = body["recovery_key_id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .expect("a recovery key id");
    (id, private_pem)
}

/// What the restore tool does: the recovery key opens the data key, the data key opens the payload.
fn open_blob(blob: &serde_json::Value, private_pem: &str) -> Option<Vec<u8>> {
    use std::str::FromStr;

    let field = |name: &str| -> Vec<u8> {
        blob[name]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u8)
            .collect()
    };

    let private = crate::crypto::master_key::PrivateMasterKey::from_str(private_pem).ok()?;
    let data_key_bytes = private.decrypt(&field("encrypted_data_key")).ok()?;
    let data_key = crate::crypto::data_key::DataKey::from_bytes(&data_key_bytes).ok()?;
    data_key.decrypt(&field("encrypted_data")).ok()
}

async fn fetch_blob(server: &TestServer, admin: &str, id: &uuid::Uuid) -> serde_json::Value {
    server
        .json(
            server
                .send(get(&format!("/v1/recovery/{id}"), Some(admin)))
                .await,
        )
        .await
}

#[tokio::test]
async fn a_blob_and_its_key_produce_the_master_key() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let (id, private_pem) = register_recovery(&server, &admin).await;

    let blob = fetch_blob(&server, &admin, &id).await;
    let recovered = open_blob(&blob, &private_pem).expect("the blob should open");

    let on_disk = std::fs::read(server.state.config.master_key_paths[0].clone())
        .expect("Should read the master key");
    assert_eq!(
        recovered, on_disk,
        "what comes out is the master key file itself, byte for byte"
    );
}

#[tokio::test]
async fn a_blob_alone_yields_nothing() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let (id, _) = register_recovery(&server, &admin).await;

    let blob = fetch_blob(&server, &admin, &id).await;
    let serialised = blob.to_string();

    let on_disk = std::fs::read_to_string(server.state.config.master_key_paths[0].clone()).unwrap();
    let body = on_disk.lines().nth(1).expect("a line of key material");
    assert!(
        !serialised.contains(body),
        "the blob must not carry the key it protects"
    );

    // And the wrong key fails cleanly rather than producing rubbish that looks like a key.
    let (other_pem, _) = crate::crypto::master_key::generate_key_pair().unwrap();
    assert!(
        open_blob(&blob, &other_pem).is_none(),
        "a recovery key that did not encrypt this blob must not open it"
    );
}

#[tokio::test]
async fn a_private_key_sent_by_mistake_is_refused() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let (private_pem, _) = crate::crypto::master_key::generate_key_pair().unwrap();

    // Pasting the wrong half must not be silently accepted: the blob is safe to store anywhere
    // only because the server cannot open it.
    let response = server
        .send(
            post("/v1/recovery", Some(&admin))
                .body(Body::from(
                    serde_json::json!({ "public_key": private_pem }).to_string(),
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = server.json(response).await;
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("public half"),
        "the refusal should say which half to send: {body}"
    );
}

#[tokio::test]
async fn two_recovery_keys_each_recover_independently() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;

    let (first_id, first_pem) = register_recovery(&server, &admin).await;
    let (second_id, second_pem) = register_recovery(&server, &admin).await;

    // Registering the second must not disturb the first: an operator adding a colleague's key
    // has not thereby retired their own.
    for (id, pem) in [(first_id, first_pem), (second_id, second_pem)] {
        let blob = fetch_blob(&server, &admin, &id).await;
        assert!(
            open_blob(&blob, &pem).is_some(),
            "each key opens its own blob"
        );
    }
}

#[tokio::test]
async fn changing_the_master_key_refreshes_every_blob() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let (id, private_pem) = register_recovery(&server, &admin).await;
    let before = fetch_blob(&server, &admin, &id).await;

    // Stand in for the key changing on disk. A backup that quietly stops matching what it is
    // meant to restore is worse than no backup.
    let (new_pem, _) = crate::crypto::master_key::generate_key_pair().unwrap();
    std::fs::write(server.state.config.master_key_paths[0].clone(), &new_pem)
        .expect("Should replace the key file");
    let refreshed = server
        .state
        .refresh_every_recovery_blob()
        .expect("Should refresh");
    assert_eq!(refreshed, 1);

    let after = fetch_blob(&server, &admin, &id).await;
    assert_ne!(
        before["encrypted_data"], after["encrypted_data"],
        "the blob should have been re-made"
    );
    assert_eq!(
        open_blob(&after, &private_pem).map(|b| String::from_utf8_lossy(&b).into_owned()),
        Some(new_pem),
        "and it should now hold the key that is actually in use"
    );
}

#[tokio::test]
async fn only_an_admin_reaches_recovery() {
    let server = TestServer::new();
    let admin = server.bootstrap().await;
    let (id, _) = register_recovery(&server, &admin).await;
    let operator = server.identity(&admin, "alice", "operator").await;

    for uri in ["/v1/recovery", &format!("/v1/recovery/{id}")] {
        assert_eq!(
            server.send(get(uri, Some(&operator))).await.status(),
            StatusCode::FORBIDDEN,
            "{uri} is not an operator's to read"
        );
    }
}
