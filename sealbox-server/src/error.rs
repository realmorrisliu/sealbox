use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

pub type Result<T, E = SealboxError> = anyhow::Result<T, E>;

#[derive(Error, Debug)]
pub enum SealboxError {
    #[error("System is not initialized")]
    NotInitialized,

    #[error("Secret not found: {0}")]
    SecretNotFound(String),

    #[error("Master key not found: {0}")]
    MasterKeyNotFound(Uuid),

    #[error("Master key not match for {0}: expected {1}, got {2}")]
    MasterKeyNotMatch(String, String, String),

    #[error("Storage failure: {0}")]
    StorageError(String),

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Invalid method")]
    InvalidMethod,

    #[error("Invalid version")]
    InvalidVersion,

    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("PKCS1 crypto error: {0}")]
    PKCS1CryptoError(#[from] rsa::pkcs1::Error),

    #[error("RSA crypto error: {0}")]
    RSACryptoError(#[from] rsa::Error),

    #[error("AES-GCM crypto error: {0}")]
    AESGCMCryptoError(aes_gcm::Error),

    #[error("Error creating response: {0}")]
    ResponseCreationError(String),

    #[error("Database connection error: {0}")]
    DatabaseConnectionError(String),

    #[error("Unknown error")]
    Unknown,
}

fn errorfmt(error: &SealboxError) -> String {
    format!("[SealboxError] {}", error.to_string())
}

impl IntoResponse for SealboxError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            SealboxError::NotInitialized => (StatusCode::PRECONDITION_REQUIRED, errorfmt(&self)),
            SealboxError::SecretNotFound(_) => (StatusCode::NOT_FOUND, errorfmt(&self)),
            SealboxError::MasterKeyNotFound(_) => (StatusCode::NOT_FOUND, errorfmt(&self)),
            SealboxError::MasterKeyNotMatch(_, _, _) => {
                (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self))
            }
            SealboxError::StorageError(_) => (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self)),
            SealboxError::BadRequest(_) => (StatusCode::BAD_REQUEST, errorfmt(&self)),
            SealboxError::InvalidMethod => (StatusCode::METHOD_NOT_ALLOWED, errorfmt(&self)),
            SealboxError::InvalidVersion => (StatusCode::NOT_FOUND, errorfmt(&self)),
            SealboxError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self)),
            SealboxError::PKCS1CryptoError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self))
            }
            SealboxError::RSACryptoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self)),
            SealboxError::AESGCMCryptoError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self))
            }
            SealboxError::ResponseCreationError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self))
            }
            SealboxError::DatabaseConnectionError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self))
            }
            SealboxError::Unknown => (StatusCode::INTERNAL_SERVER_ERROR, errorfmt(&self)),
        };

        let body = axum::Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}

impl From<aes_gcm::Error> for SealboxError {
    fn from(err: aes_gcm::Error) -> Self {
        SealboxError::AESGCMCryptoError(err)
    }
}

impl From<std::sync::PoisonError<std::sync::MutexGuard<'_, rusqlite::Connection>>>
    for SealboxError
{
    fn from(err: std::sync::PoisonError<std::sync::MutexGuard<'_, rusqlite::Connection>>) -> Self {
        SealboxError::DatabaseConnectionError(err.to_string())
    }
}
