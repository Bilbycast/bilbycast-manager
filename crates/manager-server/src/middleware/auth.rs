use axum::extract::{FromRequestParts, State};
use axum::http::{Method, Request, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;

use crate::app_state::AppState;
use manager_core::models::UserRole;

/// Authenticated user information extracted from the JWT.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub role: UserRole,
    pub allowed_node_ids: Option<Vec<String>>,
}

impl AuthUser {
    pub fn can_access_node(&self, node_id: &str) -> bool {
        match &self.allowed_node_ids {
            None => true,
            Some(ids) => ids.iter().any(|id| id == node_id),
        }
    }
}

/// Auth middleware that validates the JWT from cookie (primary) or Authorization header (fallback).
/// Also enforces CSRF token validation on state-changing methods.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_token(&request);
    let token = token.ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = manager_core::auth::validate_session_token(&token, &state.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Check if session has been revoked (logout)
    let revoked = manager_core::db::sessions::is_session_revoked(&state.db, &claims.jti)
        .await
        .unwrap_or(false);
    if revoked {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let role = UserRole::from_str(&claims.role).ok_or(StatusCode::UNAUTHORIZED)?;

    // Fetch user to check active status and get allowed nodes
    let user = manager_core::db::users::get_user_by_id(&state.db, &claims.sub)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if !user.is_active || user.is_expired() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // CSRF validation on state-changing methods
    let method = request.method().clone();
    if method == Method::POST
        || method == Method::PUT
        || method == Method::PATCH
        || method == Method::DELETE
    {
        validate_csrf(&request)?;
    }

    let auth_user = AuthUser {
        user_id: claims.sub,
        role,
        allowed_node_ids: user.allowed_node_ids,
    };

    request.extensions_mut().insert(auth_user);

    Ok(next.run(request).await)
}

/// Validate CSRF token.
///
/// Primary: double-submit cookie — X-CSRF-Token header must match csrf_token cookie.
/// Fallback: if the browser doesn't send the csrf_token cookie (can happen with
/// self-signed TLS certs or Chrome cookie partitioning), accept a non-empty
/// X-CSRF-Token header alone. The header can only be set by same-origin JS
/// (blocked cross-origin by CORS), so its presence is sufficient CSRF proof.
fn validate_csrf(request: &Request<axum::body::Body>) -> Result<(), StatusCode> {
    // Extract CSRF token from header (required in all cases)
    let header_csrf = request
        .headers()
        .get("x-csrf-token")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let header_csrf = match header_csrf {
        Some(h) if !h.is_empty() => h,
        _ => {
            return Err(StatusCode::FORBIDDEN);
        }
    };

    // Extract CSRF token from cookie
    let cookie_csrf = request
        .headers()
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str.split(';').find_map(|cookie| {
                let cookie = cookie.trim();
                cookie.strip_prefix("csrf_token=").map(|v| v.to_string())
            })
        });

    match cookie_csrf {
        Some(c) if !c.is_empty() => {
            // Double-submit cookie validation: compare cookie and header
            if !manager_core::auth::verify_csrf_token(&c, &header_csrf) {
                return Err(StatusCode::FORBIDDEN);
            }
        }
        _ => {
            // Cookie missing (self-signed cert, browser partitioning, etc.)
            // The X-CSRF-Token header alone is sufficient — it can only be set
            // by same-origin JS due to CORS restrictions.
        }
    }

    Ok(())
}

fn extract_token(request: &Request<axum::body::Body>) -> Option<String> {
    // Try cookie first (primary method)
    if let Some(cookie_header) = request.headers().get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some(token) = cookie.strip_prefix("session=") {
                    if !token.is_empty() {
                        return Some(token.to_string());
                    }
                }
            }
        }
    }

    // Fallback: Authorization header (for API clients)
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(header_str) = auth_header.to_str() {
            if let Some(token) = header_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    None
}

/// Axum extractor for authenticated users.
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            parts
                .extensions
                .get::<AuthUser>()
                .cloned()
                .ok_or(StatusCode::UNAUTHORIZED)
        }
    }
}
