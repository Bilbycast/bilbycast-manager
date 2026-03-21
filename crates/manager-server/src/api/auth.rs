use axum::extract::{ConnectInfo, Form, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
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
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<LoginRequest>,
) -> Response {
    // Rate limiting by IP
    if !state.login_limiter.check(addr.ip()) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(LoginResponse {
                success: false,
                token: None,
                csrf_token: None,
                user: None,
                error: Some("Too many login attempts. Try again later.".into()),
            }),
        )
            .into_response();
    }

    // Always compute a dummy hash to prevent username enumeration via timing
    let dummy_hash = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    let user = manager_core::db::users::get_user_by_username(&state.db, &req.username)
        .await
        .ok()
        .flatten();

    let (password_valid, user) = match user {
        Some(u) => {
            let valid =
                manager_core::auth::verify_password(&req.password, &u.password_hash)
                    .unwrap_or(false);
            (valid, Some(u))
        }
        None => {
            // Run Argon2 against dummy hash to equalize timing
            let _ = manager_core::auth::verify_password(&req.password, dummy_hash);
            (false, None)
        }
    };

    let user = match (password_valid, user) {
        (true, Some(u)) => u,
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
            )
                .into_response();
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
        )
            .into_response();
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
        )
            .into_response();
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
            )
                .into_response();
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

    // Cookie flags: HttpOnly and Secure are omitted for self-signed certs
    // because some browsers don't reliably store httpOnly cookies from
    // fetch/redirect responses when the TLS cert is not trusted.
    let (http_only, secure_flag) = if state.is_self_signed_cert {
        ("", "")
    } else {
        (" HttpOnly;", " Secure;")
    };
    let session_cookie = format!(
        "session={token};{http_only}{secure_flag} SameSite=Lax; Path=/; Max-Age=86400"
    );
    let csrf_cookie = format!(
        "csrf_token={csrf_token};{secure_flag} SameSite=Lax; Path=/; Max-Age=86400"
    );

    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&session_cookie).unwrap(),
    );
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&csrf_cookie).unwrap(),
    );

    (
        StatusCode::OK,
        headers,
        Json(LoginResponse {
            success: true,
            token: Some(token.clone()),
            csrf_token: Some(csrf_token),
            user: Some(user_info),
            error: None,
        }),
    )
        .into_response()
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    // Extract JWT from cookie or Authorization header and revoke the session
    let token = extract_token_from_headers(&headers);

    if let Some(ref token_str) = token {
        if let Ok(claims) =
            manager_core::auth::validate_session_token(token_str, &state.jwt_secret)
        {
            // Revoke this session
            let expires_at =
                chrono::DateTime::from_timestamp(claims.exp, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

            let _ = manager_core::db::sessions::revoke_session(
                &state.db,
                &claims.jti,
                &expires_at,
            )
            .await;

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

    // Clear cookies
    let secure_flag = if state.is_self_signed_cert { "" } else { " Secure;" };
    let clear_session = format!(
        "session=; HttpOnly;{secure_flag} SameSite=Lax; Path=/; Max-Age=0"
    );
    let clear_csrf = format!(
        "csrf_token=;{secure_flag} SameSite=Lax; Path=/; Max-Age=0"
    );

    let mut resp_headers = HeaderMap::new();
    resp_headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&clear_session).unwrap(),
    );
    resp_headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&clear_csrf).unwrap(),
    );

    (
        StatusCode::OK,
        resp_headers,
        Json(serde_json::json!({"success": true})),
    )
        .into_response()
}

/// Form-based login that responds with a 302 redirect + Set-Cookie headers.
/// Browsers reliably process Set-Cookie on redirect responses (unlike fetch).
#[derive(Deserialize)]
pub struct LoginFormRequest {
    pub username: String,
    pub password: String,
}

pub async fn login_form(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Form(req): Form<LoginFormRequest>,
) -> Response {
    if !state.login_limiter.check(addr.ip()) {
        return Redirect::to("/login?error=rate_limited").into_response();
    }

    let dummy_hash = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    let user = manager_core::db::users::get_user_by_username(&state.db, &req.username)
        .await
        .ok()
        .flatten();

    let (password_valid, user) = match user {
        Some(u) => {
            let valid = manager_core::auth::verify_password(&req.password, &u.password_hash)
                .unwrap_or(false);
            (valid, Some(u))
        }
        None => {
            let _ = manager_core::auth::verify_password(&req.password, dummy_hash);
            (false, None)
        }
    };

    let user = match (password_valid, user) {
        (true, Some(u)) if u.is_active && !u.is_expired() => u,
        _ => return Redirect::to("/login?error=invalid").into_response(),
    };

    let session_id = Uuid::new_v4().to_string();
    let token = match manager_core::auth::create_session_token(
        &user.id, user.role, &session_id, &state.jwt_secret, 24,
    ) {
        Ok(t) => t,
        Err(_) => return Redirect::to("/login?error=internal").into_response(),
    };

    let csrf_token = manager_core::auth::generate_csrf_token();

    let _ = manager_core::db::users::update_last_login(&state.db, &user.id).await;
    let _ = manager_core::db::audit::log_audit(
        &state.db, Some(&user.id), "auth.login", Some("user"), Some(&user.id), None, None,
    ).await;

    let (http_only, secure_flag) = if state.is_self_signed_cert {
        ("", "")
    } else {
        (" HttpOnly;", " Secure;")
    };
    let session_cookie = format!(
        "session={token};{http_only}{secure_flag} SameSite=Lax; Path=/; Max-Age=86400"
    );
    let csrf_cookie = format!(
        "csrf_token={csrf_token};{secure_flag} SameSite=Lax; Path=/; Max-Age=86400"
    );

    let mut headers = HeaderMap::new();
    headers.insert(header::LOCATION, HeaderValue::from_static("/dashboard"));
    headers.append(header::SET_COOKIE, HeaderValue::from_str(&session_cookie).unwrap());
    headers.append(header::SET_COOKIE, HeaderValue::from_str(&csrf_cookie).unwrap());

    (StatusCode::SEE_OTHER, headers).into_response()
}

fn extract_token_from_headers(headers: &HeaderMap) -> Option<String> {
    // Try cookie first
    if let Some(cookie_header) = headers.get(header::COOKIE) {
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
    // Try Authorization header
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        if let Ok(header_str) = auth_header.to_str() {
            if let Some(token) = header_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }
    None
}
