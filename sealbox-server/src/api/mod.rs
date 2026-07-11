use axum::{
    Router,
    extract::State,
    http::{HeaderName, Request},
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::{get, post},
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
        auth::{static_auth, tenant_auth},
        handler::{admin, master_key, secret, tenant},
        state::AppState,
    },
    config::SealboxConfig,
    error::{Result, SealboxError},
};

mod auth;
mod handler;
mod path;
mod state;

const REQUEST_ID_HEADER: &str = "x-request-id";

pub fn create_app(config: &SealboxConfig) -> Result<Router> {
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

    let state = AppState::new(config)?;

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

    let legacy_routes = Router::new()
        .route("/{version}/secrets", get(secret::list))
        .route(
            "/{version}/secrets/{secret_key}/history",
            get(secret::history),
        )
        .route(
            "/{version}/secrets/{secret_key}",
            get(secret::get).put(secret::save).delete(secret::delete),
        )
        .route(
            "/{version}/master-key",
            get(master_key::list)
                .put(master_key::rotate)
                .post(master_key::create),
        )
        .route("/{version}/master-key/active", get(master_key::active))
        .route(
            "/{version}/master-key/by-id/{master_key_id}",
            get(master_key::get),
        )
        .route(
            "/{version}/master-key/by-id/{master_key_id}/secrets",
            get(master_key::secrets),
        )
        .route(
            "/{version}/admin/cleanup-expired",
            axum::routing::delete(admin::cleanup_expired),
        )
        .route_layer(from_fn_with_state(state.clone(), static_auth));

    let tenant_routes = Router::new()
        .route("/v2/secrets", get(secret::list_v2))
        .route("/v2/secrets/{secret_key}/history", get(secret::history_v2))
        .route(
            "/v2/secrets/{secret_key}",
            get(secret::get_v2)
                .put(secret::save_v2)
                .delete(secret::delete_v2),
        )
        .route(
            "/v2/master-key",
            get(master_key::list_v2)
                .put(master_key::rotate_v2)
                .post(master_key::create_v2),
        )
        .route("/v2/master-key/active", get(master_key::active_v2))
        .route(
            "/v2/master-key/by-id/{master_key_id}",
            get(master_key::get_v2),
        )
        .route(
            "/v2/master-key/by-id/{master_key_id}/secrets",
            get(master_key::secrets_v2),
        )
        .route_layer(from_fn_with_state(state.clone(), tenant_auth));

    let tenant_admin_routes = Router::new()
        .route("/v2/admin/tenants", get(tenant::list).post(tenant::create))
        .route("/v2/admin/tenants/{tenant_id}", get(tenant::get))
        .route(
            "/v2/admin/tenants/{tenant_id}/suspend",
            post(tenant::suspend),
        )
        .route("/v2/admin/tenants/{tenant_id}/resume", post(tenant::resume))
        .route(
            "/v2/admin/tenants/{tenant_id}/tokens",
            get(tenant::list_tokens).post(tenant::create_token),
        )
        .route(
            "/v2/admin/tenants/{tenant_id}/tokens/{token_id}",
            axum::routing::delete(tenant::revoke_token),
        )
        .route_layer(from_fn_with_state(state.clone(), static_auth));

    let app = Router::new()
        // Health check endpoints without authentication (Kubernetes standard)
        .route("/", get(root))
        .route("/healthz/live", get(liveness_probe))
        .route("/healthz/ready", get(readiness_probe))
        .merge(tenant_routes)
        .merge(tenant_admin_routes);
    let app = if config.legacy_v1_enabled {
        app.merge(legacy_routes)
    } else {
        app
    };
    Ok(app
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
    SealboxResponse::Ok
}

/// Readiness probe - check if service is ready to receive traffic
/// Checks database connection and other critical dependencies for Kubernetes readiness probe
async fn readiness_probe(State(state): State<AppState>) -> Result<SealboxResponse> {
    let conn = state.conn_pool.lock().map_err(|e| {
        error!("{}", e);
        SealboxError::DatabaseError("Database connection unavailable".to_string())
    })?;

    state.health_repo.check_health(&conn).map_err(|e| {
        error!("{}", e);
        SealboxError::DatabaseError("Database health check failed".to_string())
    })?;

    Ok(SealboxResponse::Ok)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
enum Version {
    V1,
    V2,
    V3,
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
