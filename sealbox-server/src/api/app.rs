use axum::{
    Router,
    extract::{Json, State},
    http::{HeaderName, Request},
    response::{IntoResponse, Response},
    routing::get,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{error, info_span};
use uuid::Uuid;

use crate::{
    api::{path::Path, state::AppState},
    config::SealboxConfig,
    error::{Result, SealboxError},
    repo::{MasterKey, Secret},
};

const REQUEST_ID_HEADER: &str = "x-request-id";

pub fn create_app(config: &SealboxConfig) -> Result<Router> {
    let x_request_id = HeaderName::from_static(REQUEST_ID_HEADER);
    let middleware = ServiceBuilder::new()
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

    tracing::info!("Initializing API routes");

    Ok(Router::new()
        .route("/", get(root))
        .route(
            "/{version}/secrets/{secret_key}",
            get(get_secret).put(save_secret).delete(delete_secret),
        )
        .route(
            "/{version}/master-key",
            get(list_master_keys)
                .put(rotate_master_key)
                .post(create_master_key),
        )
        .with_state(AppState::new(config)?)
        .layer(middleware))
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

#[derive(Debug, Deserialize, Serialize, Clone)]
struct SecretPathParams {
    version: Version,
    secret_key: String,
}

impl SecretPathParams {
    fn version(&self) -> Version {
        self.version.clone()
    }
    fn secret_key(&self) -> String {
        self.secret_key.clone()
    }
}

// GET /{version}/secrets/{secret_key}
async fn get_secret(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let secret = state.secret_repo.get_secret(&params.secret_key())?;
            Ok(SealboxResponse::Json(json!({"secret": secret})))
        }
        _ => Err(SealboxError::InvalidVersion),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct SaveSecretPayload {
    secret: String,
}

// PUT /{version}/secrets/{secret_key}
async fn save_secret(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
    Json(payload): Json<SaveSecretPayload>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let master_key = state
                .master_key_repo
                .get_valid_master_key()?
                .ok_or_else(|| SealboxError::NotInitialized)?;

            let secret = Secret::new(&params.secret_key(), &payload.secret, master_key)?;
            state.secret_repo.save_secret(&secret)?;

            Ok(SealboxResponse::Ok)
        }
        _ => Err(SealboxError::InvalidVersion),
    }
}

// DELETE /{version}/secrets/{secret_key}
async fn delete_secret(
    State(state): State<AppState>,
    Path(params): Path<SecretPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            state.secret_repo.delete_secret(&params.secret_key())?;
            Ok(SealboxResponse::Ok)
        }
        _ => Err(SealboxError::InvalidVersion),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct MasterKeyPathParams {
    version: Version,
}

impl MasterKeyPathParams {
    fn version(&self) -> Version {
        self.version.clone()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RotateMasterKeyPayload {
    new_master_key_id: Uuid,
    old_master_key_id: Uuid,
    old_private_key_pem: String,
}

// GET /{version}/master-key
async fn list_master_keys(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let master_keys = state.master_key_repo.fetch_all_master_keys()?;
            Ok(SealboxResponse::Json(json!({"master_keys": master_keys})))
        }
        _ => Err(SealboxError::InvalidVersion),
    }
}

// PUT /{version}/master-key
async fn rotate_master_key(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyPathParams>,
    Json(payload): Json<RotateMasterKeyPayload>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let new_master_key_id = payload.new_master_key_id;
            let old_master_key_id = payload.old_master_key_id;
            let old_private_key_pem = payload.old_private_key_pem;

            let new_public_key_pem = state
                .master_key_repo
                .fetch_public_key(&new_master_key_id)?
                .ok_or_else(|| SealboxError::MasterKeyNotFound(new_master_key_id.clone()))?;

            let secrets = state
                .secret_repo
                .fetch_secrets_by_master_key(&old_master_key_id)?;

            let mut failed_secret_keys = Vec::new();

            for secret in secrets {
                let secret_key = secret.key.clone();

                match secret.rotate_master_key(
                    &old_master_key_id,
                    &old_private_key_pem,
                    &new_master_key_id,
                    &new_public_key_pem,
                ) {
                    Ok(rotated_secret) => {
                        state
                            .secret_repo
                            .update_secret_master_key(&rotated_secret)?;
                    }
                    Err(err) => {
                        failed_secret_keys.push(secret_key.clone());
                        error!(
                            "Failed to rotate master key for secret {}: {}",
                            secret_key, err
                        );
                    }
                }
            }

            if !failed_secret_keys.is_empty() {
                return Ok(SealboxResponse::Json(json!({
                  "master_key": new_master_key_id,
                  "failed_secret_keys": failed_secret_keys
                })));
            }

            Ok(SealboxResponse::Json(
                json!({"master_key": new_master_key_id}),
            ))
        }
        _ => Err(SealboxError::InvalidVersion),
    }
}

// POST /{version}/master-key
async fn create_master_key(
    State(state): State<AppState>,
    Path(params): Path<MasterKeyPathParams>,
) -> Result<SealboxResponse> {
    match params.version() {
        Version::V1 => {
            let (master_key, private_key) = MasterKey::create_key_pair()?;
            state.master_key_repo.create_master_key(&master_key)?;
            Ok(SealboxResponse::Text(private_key))
        }
        _ => Err(SealboxError::InvalidVersion),
    }
}
