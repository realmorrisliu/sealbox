use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::crypto::{client_key::ClientKeyCryptoError, data_key::DataKeyCryptoError};

pub type Result<T, E = SealboxError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum SealboxError {
    #[error("Secret not found: {0}")]
    SecretNotFound(String),

    #[error("Missing valid client key")]
    MissingValidClientKey,

    #[error("No valid client key found")]
    NoValidClientKey,

    #[error("Client key not found: {0}")]
    ClientKeyNotFound(Uuid),

    #[error("Client key mismatch for {0}: expected {1}, got {2}")]
    ClientKeyMismatch(String, String, String),

    #[error("Crypto error: {0}")]
    CryptoError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Response build failed: {0}")]
    ResponseBuildFailed(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Invalid API version")]
    InvalidApiVersion,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Unknown error")]
    Unknown,
}

impl IntoResponse for SealboxError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            SealboxError::SecretNotFound(_) => {
                (StatusCode::NOT_FOUND, format!("[SealboxError] {self}"))
            }
            SealboxError::MissingValidClientKey => (
                StatusCode::PRECONDITION_REQUIRED,
                format!("[SealboxError] {self}"),
            ),
            SealboxError::NoValidClientKey => (
                StatusCode::PRECONDITION_REQUIRED,
                format!("[SealboxError] {self}"),
            ),
            SealboxError::ClientKeyNotFound(_) => {
                (StatusCode::NOT_FOUND, format!("[SealboxError] {self}"))
            }
            SealboxError::ClientKeyMismatch(_, _, _) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("[SealboxError] {self}"),
            ),
            SealboxError::CryptoError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("[SealboxError] {self}"),
            ),
            SealboxError::DatabaseError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("[SealboxError] {self}"),
            ),
            SealboxError::ResponseBuildFailed(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("[SealboxError] {self}"),
            ),
            SealboxError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, format!("[SealboxError] {self}"))
            }
            SealboxError::InvalidApiVersion => {
                (StatusCode::NOT_FOUND, format!("[SealboxError] {self}"))
            }
            SealboxError::InvalidInput(_) => {
                (StatusCode::BAD_REQUEST, format!("[SealboxError] {self}"))
            }
            SealboxError::Unknown => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("[SealboxError] {self}"),
            ),
        };

        let body = axum::Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}

impl From<ClientKeyCryptoError> for SealboxError {
    fn from(err: ClientKeyCryptoError) -> Self {
        SealboxError::CryptoError(err.to_string())
    }
}

impl From<DataKeyCryptoError> for SealboxError {
    fn from(err: DataKeyCryptoError) -> Self {
        SealboxError::CryptoError(err.to_string())
    }
}

impl From<sqlx::Error> for SealboxError {
    fn from(err: sqlx::Error) -> Self {
        SealboxError::DatabaseError(err.to_string())
    }
}
