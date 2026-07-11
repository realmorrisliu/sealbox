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

#[derive(Clone, Debug)]
pub(crate) struct TenantPrincipal {
    pub(crate) tenant_id: String,
    pub(crate) token_id: uuid::Uuid,
}

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
            if constant_time_eq(token.as_bytes(), state.config.auth_token.as_bytes()) {
                Ok(next.run(request).await)
            } else {
                Err(SealboxError::Unauthorized)
            }
        }
        _ => Err(SealboxError::Unauthorized),
    }
}

pub(crate) async fn tenant_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    let Some(token) = bearer_token(&headers) else {
        return Err(SealboxError::Unauthorized);
    };
    let authenticated = {
        let conn = state.conn_pool.lock()?;
        state.tenant_repo.authenticate_token(&conn, token)?
    };
    let Some(authenticated) = authenticated else {
        return Err(SealboxError::Unauthorized);
    };
    request.extensions_mut().insert(TenantPrincipal {
        tenant_id: authenticated.tenant_id,
        token_id: authenticated.token_id,
    });
    Ok(next.run(request).await)
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();

    for index in 0..max_len {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        diff |= usize::from(left_byte ^ right_byte);
    }

    diff == 0
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"same-token", b"same-token"));
        assert!(!constant_time_eq(b"same-token", b"other-token"));
        assert!(!constant_time_eq(b"short", b"shorter"));
    }
}
