use axum::{
    Router,
    extract::State,
    http::{HeaderName, Request},
    middleware::{from_fn, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::{get, put},
};
use http::StatusCode;
use serde_json::json;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{error, info_span};

use crate::{
    api::{
        auth::{
            authenticate_and_audit, require_admin, require_agent, require_operator, require_runner,
        },
        handler::{admin, admin_auth, audit, grant, identity, job, master_key, secret},
        state::AppState,
    },
    config::SealboxConfig,
    error::{Result, SealboxError},
};

mod auth;
mod handler;
pub(crate) mod passkey;
mod path;
pub(crate) mod state;

const REQUEST_ID_HEADER: &str = "x-request-id";

/// A job claimed but unreported for this long is presumed lost.
pub const JOB_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10 * 60);

#[cfg(test)]
mod tests;

pub fn create_app(config: &SealboxConfig) -> Result<Router> {
    Ok(build_router(AppState::new(config)?))
}

/// Split out from `create_app` so the tests below can hold the state as well as the router —
/// minting a passkey session directly, since a WebAuthn ceremony needs an authenticator and a
/// person. No route does this; there is no way in from outside the crate.
fn build_router(state: AppState) -> Router {
    tracing::info!("Initializing API routes");
    let x_request_id = HeaderName::from_static(REQUEST_ID_HEADER);
    let request_id_middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(
            x_request_id.clone(),
            MakeRequestUuid,
        ))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                // Log the request id as generated.
                let request_id = request.headers().get(REQUEST_ID_HEADER);

                match request_id {
                    Some(request_id) => info_span!(
                        "http_request",
                        request_id = ?request_id,
                    ),
                    None => {
                        error!("could not extract request_id");
                        info_span!("http_request")
                    }
                }
            }),
        )
        // send headers from request to response headers
        .layer(PropagateRequestIdLayer::new(x_request_id));

    // Sweep abandoned jobs in the background. Without this, a runner that dies mid-job leaves
    // the caller waiting on something that will never report.
    {
        let state = state.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                ticker.tick().await;
                if let Err(e) = state.sweep_abandoned_jobs(JOB_TIMEOUT) {
                    tracing::error!("Failed to sweep abandoned jobs: {e}");
                }
                state.passkey.sweep();
            }
        });
    }

    // No CORS layer, and no configuration to add one: sealbox serves no browser client
    // (ADR 0004). Behaviour is identical in debug and release builds.
    // Routes are grouped by the role they require, and each group carries its own gate.
    // A route that is not placed in a group is not in the router at all, so a forgotten
    // endpoint 404s rather than serving. A per-handler check has the opposite default.
    //
    // `route_layer` applies only to routes registered before it, which is how the health probes
    // once ended up behind authentication. The public routes are therefore registered last, on
    // the outermost router, after every auth layer.
    let agent_routes = Router::new()
        .route("/v1/secrets", get(secret::list))
        // Wildcard: secret names are hierarchical (`utopia/prod/database-url`), so the
        // path segment has to swallow slashes.
        .route("/v1/secrets/{*secret_key}", get(secret::get))
        .route("/v1/audit", get(audit::list))
        // Staging a grant is an agent's job: nothing is created until a human signs for it, so
        // the draft is harmless. Requiring an admin session to *submit* would mean two
        // ceremonies for one decision, and a ceremony people resent is a ceremony they route
        // around.
        .route("/v1/grants", get(grant::list).post(grant::create))
        .route("/v1/grants/{name}", get(grant::show))
        .route("/v1/jobs", axum::routing::post(job::submit))
        .route("/v1/jobs/{id}", get(job::show))
        .route_layer(from_fn(require_agent));

    let operator_routes = Router::new()
        .route(
            "/v1/secrets/{*secret_key}",
            put(secret::save).delete(secret::delete),
        )
        .route(
            "/v1/rotate/{*secret_key}",
            axum::routing::post(secret::rotate),
        )
        .route_layer(from_fn(require_operator));

    // Disjoint from every other group: only a runner reaches these, and a runner reaches
    // nothing else.
    let runner_routes = Router::new()
        .route("/v1/jobs/claim", get(job::claim))
        .route("/v1/jobs/{id}/result", axum::routing::post(job::report))
        .route_layer(from_fn(require_runner));

    let admin_routes = Router::new()
        .route(
            "/v1/master-key",
            get(master_key::list)
                .put(master_key::rekey)
                .post(master_key::create),
        )
        .route(
            "/v1/admin/cleanup-expired",
            axum::routing::delete(admin::cleanup_expired),
        )
        .route("/v1/identities", get(identity::list).post(identity::create))
        .route(
            "/v1/identities/{name}",
            axum::routing::delete(identity::revoke),
        )
        .route("/v1/grants/{name}", axum::routing::delete(grant::remove))
        .route_layer(from_fn(require_admin));

    Router::new()
        .merge(agent_routes)
        .merge(operator_routes)
        .merge(admin_routes)
        .merge(runner_routes)
        .route_layer(from_fn_with_state(state.clone(), authenticate_and_audit))
        // Public: no credential, and not audited. Registered after every auth layer.
        .route("/", get(root))
        .route("/healthz/live", get(liveness_probe))
        .route("/healthz/ready", get(readiness_probe))
        .route("/v1/bootstrap", axum::routing::post(identity::bootstrap))
        // The ceremony. Public because a browser reaches them, and safe because every id is an
        // unguessable single-use token with a short life. Not an interface (ADR 0004): they must
        // never grow a way to *manage* anything.
        .route("/enrol/{id}", get(admin_auth::enrol_page))
        .route(
            "/enrol/{id}/start",
            axum::routing::post(admin_auth::enrol_start),
        )
        .route(
            "/enrol/{id}/finish",
            axum::routing::post(admin_auth::enrol_finish),
        )
        .route("/approve/{id}", get(admin_auth::approve_page))
        .route(
            "/approve/{id}/start",
            axum::routing::post(admin_auth::approve_start),
        )
        .route(
            "/approve/{id}/finish",
            axum::routing::post(admin_auth::approve_finish),
        )
        // Sign-in: the CLI opens a request, a browser signs it, and the session goes back to
        // the waiting process — never through the terminal, where it would land in scrollback.
        .route(
            "/v1/auth/login",
            axum::routing::post(admin_auth::login_open),
        )
        .route("/v1/auth/login/{id}", get(admin_auth::login_collect))
        .route("/login/{id}", get(admin_auth::login_page))
        .route(
            "/login/{id}/start",
            axum::routing::post(admin_auth::login_start),
        )
        .route(
            "/login/{id}/finish",
            axum::routing::post(admin_auth::login_finish),
        )
        .with_state(state)
        .layer(request_id_middleware)
}

async fn root() -> &'static str {
    "Hello, Sealbox!"
}

/// Liveness probe - check if service is alive
/// Returns simple status information for Kubernetes liveness probe
async fn liveness_probe() -> SealboxResponse {
    SealboxResponse::Ok
}

/// Readiness probe - check if service is ready to receive traffic
/// Checks database connection and other critical dependencies for Kubernetes readiness probe
async fn readiness_probe(State(state): State<AppState>) -> Result<SealboxResponse> {
    state.health_repo.check_health().map_err(|e| {
        error!("{}", e);
        SealboxError::DatabaseError("Database health check failed".to_string())
    })?;

    Ok(SealboxResponse::Ok)
}

#[derive(Debug)]
pub enum SealboxResponse {
    Ok,
    Json(serde_json::Value),
    Text(String),
}
impl IntoResponse for SealboxResponse {
    fn into_response(self) -> Response {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        match self {
            SealboxResponse::Ok => {
                axum::Json(json!({"result": "Ok","timestamp": now})).into_response()
            }
            SealboxResponse::Json(data) => axum::Json(data).into_response(),
            SealboxResponse::Text(data) => axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(data)
                .map(|response| response.into_response())
                .unwrap_or_else(|err| {
                    SealboxError::ResponseBuildFailed(err.to_string()).into_response()
                }),
        }
    }
}
