use axum::{
    Router,
    http::{HeaderName, Request},
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::get,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{error, info_span};

use crate::{
    api::{
        auth::static_auth,
        handler::{master_key, secret},
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

    let state = AppState::new(config)?;

    Ok(Router::new()
        .route("/", get(root))
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
        .route_layer(from_fn_with_state(state.clone(), static_auth))
        .with_state(state)
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
        .layer(PropagateRequestIdLayer::new(x_request_id)))
}

async fn root() -> &'static str {
    "Hello, Sealbox!"
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
enum Version {
    V1,
    V2,
    V3,
}

pub enum SealboxResponse {
    Ok,
    Json(serde_json::Value),
    Text(String),
}
impl IntoResponse for SealboxResponse {
    fn into_response(self) -> Response {
        match self {
            SealboxResponse::Ok => axum::Json(json!({"result": "Ok"})).into_response(),
            SealboxResponse::Json(data) => axum::Json(data).into_response(),
            SealboxResponse::Text(data) => axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(data)
                .map(|response| response.into_response())
                .unwrap_or_else(|err| {
                    SealboxError::ResponseCreationError(err.to_string()).into_response()
                }),
        }
    }
}
