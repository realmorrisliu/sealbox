use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

use crate::{
    api::state::AppState,
    error::{Result, SealboxError},
};

pub(crate) async fn static_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            if token == state.config.auth_token {
                Ok(next.run(request).await)
            } else {
                Err(SealboxError::Unauthorized)
            }
        }
        _ => Err(SealboxError::Unauthorized),
    }
}
