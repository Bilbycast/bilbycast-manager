use axum::response::Html;
use axum::routing::get;
use axum::Router;

use crate::app_state::AppState;

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

async fn index_page() -> Html<&'static str> {
    Html(SHELL_REDIRECT_DASHBOARD)
}

async fn login_page() -> Html<&'static str> {
    Html(LOGIN_HTML)
}

async fn dashboard_page() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

async fn topology_page() -> Html<&'static str> {
    Html(TOPOLOGY_HTML)
}

async fn events_page() -> Html<&'static str> {
    Html(EVENTS_HTML)
}

async fn users_page() -> Html<&'static str> {
    Html(USERS_HTML)
}

async fn settings_page() -> Html<&'static str> {
    Html(SETTINGS_HTML)
}

async fn ai_assistant_page() -> Html<&'static str> {
    Html(AI_ASSISTANT_HTML)
}

async fn ai_settings_page() -> Html<&'static str> {
    Html(AI_SETTINGS_HTML)
}

async fn node_detail_page() -> Html<&'static str> {
    Html(NODE_DETAIL_HTML)
}

async fn node_config_page() -> Html<&'static str> {
    Html(NODE_CONFIG_HTML)
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
