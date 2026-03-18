use axum::extract::{FromRequestParts, State};
use axum::http::{Request, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;

use crate::app_state::AppState;
use manager_core::models::UserRole;

/// Authenticated user information extracted from the JWT.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub role: UserRole,
    pub session_id: String,
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

/// Auth middleware that validates the JWT from Authorization header or cookie.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_token(&request);

    let token = token.ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = manager_core::auth::validate_session_token(&token, &state.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let role = UserRole::from_str(&claims.role).ok_or(StatusCode::UNAUTHORIZED)?;

    // Fetch user to check active status and get allowed nodes
    let user = manager_core::db::users::get_user_by_id(&state.db, &claims.sub)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if !user.is_active || user.is_expired() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let auth_user = AuthUser {
        user_id: claims.sub,
        role,
        session_id: claims.jti,
        allowed_node_ids: user.allowed_node_ids,
    };

    request.extensions_mut().insert(auth_user);

    Ok(next.run(request).await)
}

fn extract_token(request: &Request<axum::body::Body>) -> Option<String> {
    // Try Authorization header first
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(header_str) = auth_header.to_str() {
            if let Some(token) = header_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    // Try cookie
    if let Some(cookie_header) = request.headers().get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some(token) = cookie.strip_prefix("session=") {
                    return Some(token.to_string());
                }
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
