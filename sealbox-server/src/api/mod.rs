use axum::{
    Router,
    extract::State,
    http::{HeaderName, Request},
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::get,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{error, info_span};

use crate::{
    api::{
        auth::static_auth,
        handler::{admin, client, client_key, secret},
        state::AppState,
    },
    config::SealboxConfig,
    error::{Result, SealboxError},
};

mod auth;
mod enroll;
mod handler;
mod path;
mod state;

const REQUEST_ID_HEADER: &str = "x-request-id";

pub async fn create_app(config: &SealboxConfig) -> Result<Router> {
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

    let state = AppState::new(config).await?;

    // CORS configuration - allow cross-origin requests in development mode
    let cors_layer = if cfg!(debug_assertions) || std::env::var("SEALBOX_ALLOW_CORS").is_ok() {
        tracing::info!("CORS enabled for development");
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        tracing::info!("CORS disabled for production");
        CorsLayer::new().allow_origin([])
    };

    Ok(Router::new()
        // Health check endpoints without authentication (Kubernetes standard)
        .route("/", get(root))
        .route("/healthz/live", get(liveness_probe))
        .route("/healthz/ready", get(readiness_probe))
        // Business endpoints requiring authentication
        .route("/{version}/secrets", get(secret::list))
        .route(
            "/{version}/secrets/{secret_key}",
            get(secret::get).put(secret::save).delete(secret::delete),
        )
        .route(
            "/{version}/secrets/{secret_key}/permissions",
            get(secret::get_permissions),
        )
        .route(
            "/{version}/secrets/{secret_key}/permissions/{client_id}",
            axum::routing::delete(secret::revoke_permission),
        )
        .route(
            "/{version}/secrets/{secret_key}/permissions/{client_id}/data-key",
            axum::routing::put(secret::update_permission_data_key),
        )
        .route(
            "/{version}/client-key",
            get(client_key::list).post(client_key::create),
        )
        .route("/{version}/clients", get(client::list).post(client::create))
        .route(
            "/{version}/clients/{client_id}/status",
            axum::routing::put(client::update_status),
        )
        .route(
            "/{version}/clients/{client_id}/name",
            axum::routing::put(client::rename),
        )
        .route(
            "/{version}/clients/{client_id}/public-key",
            axum::routing::put(client::update_public_key),
        )
        .route(
            "/{version}/clients/{client_id}/secrets",
            get(client::list_client_secrets),
        )
        .route(
            "/{version}/admin/cleanup-expired",
            axum::routing::delete(admin::cleanup_expired),
        )
        // Enrollment code flow
        .route(
            "/{version}/enroll",
            axum::routing::post(enroll::begin_enrollment),
        )
        .route("/{version}/enroll/{code}", get(enroll::check_enrollment))
        .route(
            "/{version}/enroll/{code}/approve",
            axum::routing::put(enroll::approve_enrollment),
        )
        .route_layer(from_fn_with_state(state.clone(), static_auth))
        .with_state(state)
        .layer(cors_layer)
        .layer(request_id_middleware))
}

async fn root() -> &'static str {
    "Hello, Sealbox!"
}

/// Liveness probe - check if service is alive
/// Returns simple status information for Kubernetes liveness probe
async fn liveness_probe() -> SealboxResponse {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    SealboxResponse::Json(json!({"result": "Ok", "timestamp": now}))
}

/// Readiness probe - check if service is ready to receive traffic
/// Checks database connection and other critical dependencies for Kubernetes readiness probe
async fn readiness_probe(State(state): State<AppState>) -> Result<SealboxResponse> {
    state
        .health_repo
        .check_health(&state.pool)
        .await
        .map_err(|e| {
            error!("{}", e);
            SealboxError::DatabaseError("Database health check failed".to_string())
        })?;

    Ok(SealboxResponse::NoContent)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
enum Version {
    V1,
}

#[derive(Debug)]
pub enum SealboxResponse {
    NoContent,
    Json(serde_json::Value),
    Text(String),
}
impl IntoResponse for SealboxResponse {
    fn into_response(self) -> Response {
        match self {
            SealboxResponse::NoContent => (StatusCode::NO_CONTENT, "").into_response(),
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
