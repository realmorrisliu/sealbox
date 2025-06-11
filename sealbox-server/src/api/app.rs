use axum::{
    Router,
    extract::State,
    http::{HeaderName, Request},
    response::{IntoResponse, Response},
    routing::get,
};
use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{error, info_span};

use crate::{
    api::{path::Path, state::AppState},
    config::SealboxConfig,
    error::{Result, SealboxError},
    repo::Secret,
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
            get(handler)
                .put(handler)
                .delete(handler)
                .post(handler)
                .head(handler)
                .options(handler),
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

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Params {
    version: Version,
    secret_key: String,
}

impl Params {
    fn version(&self) -> Version {
        self.version.clone()
    }
    fn secret_key(&self) -> String {
        self.secret_key.clone()
    }
}

pub enum SealboxResponse {
    Ok,
    Data(serde_json::Value),
}
impl IntoResponse for SealboxResponse {
    fn into_response(self) -> Response {
        match self {
            SealboxResponse::Ok => axum::Json(json!({"result": "Ok"})).into_response(),
            SealboxResponse::Data(data) => axum::Json(data).into_response(),
        }
    }
}

async fn handler(
    method: Method,
    Path(params): Path<Params>,
    State(state): State<AppState>,
) -> Result<SealboxResponse> {
    match (method, params.version()) {
        (Method::GET, Version::V1) => {
            let _secret = state.secret_repo.get_secret(&params.secret_key());
            Ok(SealboxResponse::Data(json!({"secret": ""})))
        }
        (Method::PUT, Version::V1) => {
            let secret = Secret::create(&params.secret_key()).await?;
            state.secret_repo.save_secret(&secret);
            Ok(SealboxResponse::Ok)
        }
        (Method::DELETE, Version::V1) => {
            state.secret_repo.delete_secret(&params.secret_key());
            Ok(SealboxResponse::Ok)
        }
        _ => Err(SealboxError::InvalidMethod),
    }
}
