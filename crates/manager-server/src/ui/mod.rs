use axum::http::header;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::app_state::AppState;

/// Serve HTML with no-cache headers to prevent stale JS after updates.
fn html_no_cache(content: &'static str) -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")],
        Html(content),
    )
}

/// Build routes that serve the embedded web UI.
pub fn build_ui_router() -> Router<AppState> {
    Router::new()
        .route("/", get(index_page))
        .route("/login", get(login_page))
        .route("/dashboard", get(dashboard_page))
        .route("/topology", get(topology_page))
        .route("/events", get(events_page))
        .route("/admin/users", get(users_page))
        .route("/admin/settings", get(settings_page))
        .route("/ai/assistant", get(ai_assistant_page))
        .route("/ai/settings", get(ai_settings_page))
        .route("/nodes/{node_id}", get(node_detail_page))
        .route("/nodes/{node_id}/config", get(node_config_page))
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
const USERS_HTML: &str = include_str!("users.html");
const SETTINGS_HTML: &str = include_str!("settings.html");
const AI_ASSISTANT_HTML: &str = include_str!("ai_assistant.html");
const AI_SETTINGS_HTML: &str = include_str!("ai_settings.html");
const NODE_DETAIL_HTML: &str = include_str!("node_detail.html");
const NODE_CONFIG_HTML: &str = include_str!("node_config.html");
