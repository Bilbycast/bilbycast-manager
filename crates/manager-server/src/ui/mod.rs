// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::Router;

use crate::app_state::AppState;

/// Serve HTML with no-cache headers to prevent stale JS after updates.
fn html_no_cache(content: &'static str) -> impl IntoResponse {
    let versioned = content.replace("{{VERSION}}", env!("CARGO_PKG_VERSION"));
    (
        [(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")],
        Html(versioned),
    )
}

/// Server-side auth guard for UI pages. Redirects to /login?next=<path> if the
/// user has no valid session, so the browser never loads a protected page without auth.
async fn ui_auth_guard(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    // Extract session token from cookie (synchronous — no borrow across await)
    let token = extract_session_cookie(request.headers());
    let path = request
        .uri()
        .path_and_query()
        .map_or("/dashboard".to_owned(), |pq| pq.as_str().to_owned());

    let valid = match token {
        Some(t) => validate_session(&t, &state).await,
        None => false,
    };

    if valid {
        return next.run(request).await;
    }

    // Only include safe relative paths in the redirect to prevent open redirect
    let safe_path = if is_safe_redirect_path(&path) {
        percent_encode_path(&path)
    } else {
        "/dashboard".to_owned()
    };
    let redirect_url = format!("/login?next={}", safe_path);
    Redirect::temporary(&redirect_url).into_response()
}

/// Extract the session token from the Cookie header (synchronous).
pub(crate) fn extract_session_cookie(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str.split(';').find_map(|cookie| {
                let cookie = cookie.trim();
                cookie
                    .strip_prefix("session=")
                    .filter(|t| !t.is_empty())
                    .map(|t| t.to_string())
            })
        })
}

/// Validate a session token: check JWT, revocation, and user status.
pub(crate) async fn validate_session(token: &str, state: &AppState) -> bool {
    let Ok(claims) = manager_core::auth::validate_session_token(token, &state.jwt_secret) else {
        return false;
    };

    // Check session not revoked
    let revoked = manager_core::db::sessions::is_session_revoked(&state.db, &claims.jti)
        .await
        .unwrap_or(false);
    if revoked {
        return false;
    }

    // Check user active and not expired
    let Ok(user) = manager_core::db::users::get_user_by_id(&state.db, &claims.sub).await else {
        return false;
    };

    user.is_active && !user.is_expired()
}

/// Minimal percent-encoding for the `next` query parameter value.
fn percent_encode_path(path: &str) -> String {
    path.replace('%', "%25")
        .replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('#', "%23")
}

/// Only allow simple relative paths to prevent open redirect attacks.
fn is_safe_redirect_path(path: &str) -> bool {
    // Must start with / and only contain safe characters
    // Rejects protocol-relative (//), data:, javascript:, etc.
    path.starts_with('/')
        && !path.starts_with("//")
        && path.bytes().all(|b| b.is_ascii_alphanumeric()
            || b == b'/' || b == b'-' || b == b'_' || b == b'.' || b == b'%')
}

/// Build routes that serve the embedded web UI.
pub fn build_ui_router(state: AppState) -> Router<AppState> {
    // Protected pages — require a valid session, redirect to /login otherwise
    let protected = Router::new()
        .route("/dashboard", get(dashboard_page))
        .route("/topology", get(topology_page))
        .route("/events", get(events_page))
        .route("/admin/nodes", get(managed_nodes_page))
        .route("/admin/users", get(users_page))
        .route("/admin/settings", get(settings_page))
        .route("/ai/assistant", get(ai_assistant_page))
        .route("/ai/settings", get(ai_settings_page))
        .route("/nodes/{node_id}", get(node_detail_page))
        .route("/nodes/{node_id}/config", get(node_config_page))
        .route_layer(axum::middleware::from_fn_with_state(
            state,
            ui_auth_guard,
        ));

    // Public pages — no auth required
    Router::new()
        .route("/", get(index_page))
        .route("/login", get(login_page))
        .merge(protected)
}

async fn index_page() -> impl IntoResponse {
    html_no_cache(SHELL_REDIRECT_DASHBOARD)
}

async fn login_page() -> impl IntoResponse {
    html_no_cache(LOGIN_HTML)
}

async fn dashboard_page() -> impl IntoResponse {
    html_no_cache(DASHBOARD_HTML)
}

async fn topology_page() -> impl IntoResponse {
    html_no_cache(TOPOLOGY_HTML)
}

async fn events_page() -> impl IntoResponse {
    html_no_cache(EVENTS_HTML)
}

async fn managed_nodes_page() -> impl IntoResponse {
    html_no_cache(MANAGED_NODES_HTML)
}

async fn users_page() -> impl IntoResponse {
    html_no_cache(USERS_HTML)
}

async fn settings_page() -> impl IntoResponse {
    html_no_cache(SETTINGS_HTML)
}

async fn ai_assistant_page() -> impl IntoResponse {
    html_no_cache(AI_ASSISTANT_HTML)
}

async fn ai_settings_page() -> impl IntoResponse {
    html_no_cache(AI_SETTINGS_HTML)
}

async fn node_detail_page() -> impl IntoResponse {
    html_no_cache(NODE_DETAIL_HTML)
}

async fn node_config_page() -> impl IntoResponse {
    html_no_cache(NODE_CONFIG_HTML)
}

const SHELL_REDIRECT_DASHBOARD: &str = r#"<!DOCTYPE html><html><head><meta http-equiv="refresh" content="0; url=/dashboard"></head><body></body></html>"#;

const LOGIN_HTML: &str = include_str!("login.html");
const DASHBOARD_HTML: &str = include_str!("dashboard.html");
const TOPOLOGY_HTML: &str = include_str!("topology.html");
const EVENTS_HTML: &str = include_str!("events.html");
const MANAGED_NODES_HTML: &str = include_str!("managed_nodes.html");
const USERS_HTML: &str = include_str!("users.html");
const SETTINGS_HTML: &str = include_str!("settings.html");
const AI_ASSISTANT_HTML: &str = include_str!("ai_assistant.html");
const AI_SETTINGS_HTML: &str = include_str!("ai_settings.html");
const NODE_DETAIL_HTML: &str = include_str!("node_detail.html");
const NODE_CONFIG_HTML: &str = include_str!("node_config.html");
