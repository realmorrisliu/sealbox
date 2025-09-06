use axum::extract::State;
use serde_json::json;

use crate::{
    api::{SealboxResponse, state::AppState},
    error::Result,
};

/// API handler for cleaning up expired secrets
///
/// # Arguments
///
/// * `state` - Application state containing database connection pool and repository instances
///
/// # Returns
///
/// Returns JSON response with cleanup statistics
///
/// # HTTP Route
///
/// `DELETE /v1/admin/cleanup-expired`
///
/// # Response Format
///
/// ```json
/// {
///   "deleted_count": 42,
///   "cleaned_at": 1703876543
/// }
/// ```
pub(crate) async fn cleanup_expired(State(state): State<AppState>) -> Result<SealboxResponse> {
    let deleted_count = state
        .secret_repo
        .cleanup_expired_secrets(&state.pool)
        .await?;
    let cleaned_at = time::OffsetDateTime::now_utc().unix_timestamp();

    Ok(SealboxResponse::Json(json!({
        "deleted_count": deleted_count,
        "cleaned_at": cleaned_at
    })))
}
