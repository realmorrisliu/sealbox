use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::crypto::{data_key::DataKeyCryptoError, master_key::MasterKeyCryptoError};

pub type Result<T, E = SealboxError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum SealboxError {
    #[error("Secret not found: {0}")]
    SecretNotFound(String),

    #[error("Missing valid master key")]
    MissingValidMasterKey,

    #[error("Master key not found: {0}")]
    MasterKeyNotFound(Uuid),

    #[error("Master key mismatch for {0}: expected {1}, got {2}")]
    MasterKeyMismatch(String, String, String),

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

    #[error("Unknown error")]
    Unknown,
}

fn errorfmt(error: &SealboxError) -> String {
    format!("[SealboxError] {error}")
}

impl IntoResponse for SealboxError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            SealboxError::SecretNotFound(_) => (StatusCode::NOT_FOUND, errorfmt(&self)),
            SealboxError::MissingValidMasterKey => {
                (StatusCode::PRECONDITION_REQUIRED, errorfmt(&self))
            }
            SealboxError::MasterKeyNotFound(_) => (StatusCode::NOT_FOUND, errorfmt(&self)),
            SealboxError::MasterKeyMismatch(_, _, _) => {
                (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self))
            }
            SealboxError::CryptoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self)),
            SealboxError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self)),
            SealboxError::ResponseBuildFailed(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self))
            }
            SealboxError::Unauthorized => (StatusCode::UNAUTHORIZED, errorfmt(&self)),
            SealboxError::InvalidApiVersion => (StatusCode::NOT_FOUND, errorfmt(&self)),
            SealboxError::Unknown => (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self)),
        };

        let body = axum::Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}

impl From<MasterKeyCryptoError> for SealboxError {
    fn from(err: MasterKeyCryptoError) -> Self {
        SealboxError::CryptoError(err.to_string())
    }
}

impl From<DataKeyCryptoError> for SealboxError {
    fn from(err: DataKeyCryptoError) -> Self {
        SealboxError::CryptoError(err.to_string())
    }
}

impl From<std::sync::PoisonError<std::sync::MutexGuard<'_, rusqlite::Connection>>>
    for SealboxError
{
    fn from(err: std::sync::PoisonError<std::sync::MutexGuard<'_, rusqlite::Connection>>) -> Self {
        SealboxError::DatabaseError(err.to_string())
    }
}

impl From<rusqlite::Error> for SealboxError {
    fn from(err: rusqlite::Error) -> Self {
        SealboxError::DatabaseError(err.to_string())
    }
}
