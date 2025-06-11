use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;
use tracing::debug;

pub type Result<T, E = SealboxError> = anyhow::Result<T, E>;

#[derive(Error, Debug)]
pub enum SealboxError {
    #[error("Secret not found: {0}")]
    NotFound(String),

    #[error("Storage failure: {0}")]
    StorageError(String),

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Invalid method")]
    InvalidMethod,

    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Unknown error")]
    Unknown,
}

impl IntoResponse for SealboxError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            SealboxError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            SealboxError::StorageError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            SealboxError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            SealboxError::InvalidMethod => (StatusCode::METHOD_NOT_ALLOWED, self.to_string()),
            SealboxError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            SealboxError::Unknown => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = axum::Json(json!({
            "error": message,
        }));

        debug!("Responding with error: {}", message);

        (status, body).into_response()
    }
}
