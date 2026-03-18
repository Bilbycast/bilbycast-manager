use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app_state::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token: Option<String>,
    pub csrf_token: Option<String>,
    pub user: Option<manager_core::models::UserInfo>,
    pub error: Option<String>,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> (StatusCode, Json<LoginResponse>) {
    let user = match manager_core::db::users::get_user_by_username(&state.db, &req.username).await {
        Ok(Some(user)) => user,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(LoginResponse {
                    success: false,
                    token: None,
                    csrf_token: None,
                    user: None,
                    error: Some("Invalid credentials".into()),
                }),
            );
        }
    };

    if !user.is_active {
        return (
            StatusCode::UNAUTHORIZED,
            Json(LoginResponse {
                success: false,
                token: None,
                csrf_token: None,
                user: None,
                error: Some("Account is disabled".into()),
            }),
        );
    }

    if user.is_expired() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(LoginResponse {
                success: false,
                token: None,
                csrf_token: None,
                user: None,
                error: Some("Account has expired".into()),
            }),
        );
    }

    let password_valid =
        manager_core::auth::verify_password(&req.password, &user.password_hash).unwrap_or(false);

    if !password_valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(LoginResponse {
                success: false,
                token: None,
                csrf_token: None,
                user: None,
                error: Some("Invalid credentials".into()),
            }),
        );
    }

    let session_id = Uuid::new_v4().to_string();
    let token = match manager_core::auth::create_session_token(
        &user.id,
        user.role,
        &session_id,
        &state.jwt_secret,
        24,
    ) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to create JWT: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LoginResponse {
                    success: false,
                    token: None,
                    csrf_token: None,
                    user: None,
                    error: Some("Internal error".into()),
                }),
            );
        }
    };

    let csrf_token = manager_core::auth::generate_csrf_token();

    // Update last login
    let _ = manager_core::db::users::update_last_login(&state.db, &user.id).await;

    // Log audit
    let _ = manager_core::db::audit::log_audit(
        &state.db,
        Some(&user.id),
        "auth.login",
        Some("user"),
        Some(&user.id),
        None,
        None,
    )
    .await;

    let user_info = manager_core::models::UserInfo::from(user);

    (
        StatusCode::OK,
        Json(LoginResponse {
            success: true,
            token: Some(token),
            csrf_token: Some(csrf_token),
            user: Some(user_info),
            error: None,
        }),
    )
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    // Extract user from auth header for audit
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(header_str) = auth_header.to_str() {
            if let Some(token) = header_str.strip_prefix("Bearer ") {
                if let Ok(claims) =
                    manager_core::auth::validate_session_token(token, &state.jwt_secret)
                {
                    let _ = manager_core::db::audit::log_audit(
                        &state.db,
                        Some(&claims.sub),
                        "auth.logout",
                        Some("user"),
                        Some(&claims.sub),
                        None,
                        None,
                    )
                    .await;
                }
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({"success": true})),
    )
}
