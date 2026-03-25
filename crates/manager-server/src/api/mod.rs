// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

pub mod auth;
pub mod users;
pub mod nodes;
pub mod events;
pub mod settings;
pub mod ai;
pub mod export;
pub mod tunnels;
pub mod topology;

use axum::Router;
use axum::routing::{get, post, delete};

use crate::app_state::AppState;

pub fn build_api_router(state: AppState) -> Router<AppState> {
    let public = Router::new()
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/login-form", post(auth::login_form))
        .route("/health", get(health_check));

    let authenticated = Router::new()
        // Auth (logout requires valid session + CSRF)
        .route("/api/v1/auth/logout", post(auth::logout))
        // Users
        .route("/api/v1/users", get(users::list_users).post(users::create_user))
        .route(
            "/api/v1/users/{id}",
            get(users::get_user)
                .put(users::update_user)
                .delete(users::delete_user),
        )
        // Device drivers
        .route("/api/v1/device-types", get(nodes::list_device_types))
        // Nodes
        .route("/api/v1/nodes", get(nodes::list_nodes).post(nodes::create_node))
        .route("/api/v1/nodes/endpoints", get(nodes::list_endpoints))
        .route(
            "/api/v1/nodes/{id}",
            get(nodes::get_node)
                .put(nodes::update_node)
                .delete(nodes::delete_node),
        )
        .route("/api/v1/nodes/{id}/token", post(nodes::regenerate_token))
        .route("/api/v1/nodes/{id}/config", get(nodes::get_node_config).put(nodes::update_node_config))
        .route("/api/v1/nodes/{id}/command", post(nodes::send_command))
        .route("/api/v1/nodes/{id}/flows", post(nodes::proxy_flow_create))
        .route("/api/v1/nodes/{id}/flows/{flow_id}", delete(nodes::proxy_flow_delete))
        // Events
        .route("/api/v1/events", get(events::list_events))
        .route("/api/v1/events/{id}/ack", post(events::acknowledge_event))
        .route("/api/v1/events/count", get(events::unacknowledged_count))
        // Settings
        .route(
            "/api/v1/settings",
            get(settings::get_settings).put(settings::update_settings),
        )
        .route("/api/v1/settings/tls", get(settings::get_tls_info))
        .route("/api/v1/settings/tls/upload", post(settings::upload_tls_cert))
        // Export / Import
        .route("/api/v1/export", get(export::export_data))
        .route("/api/v1/import", post(export::import_data))
        // Tunnels
        .route("/api/v1/tunnels", get(tunnels::list_tunnels).post(tunnels::create_tunnel))
        .route(
            "/api/v1/tunnels/{id}",
            get(tunnels::get_tunnel)
                .put(tunnels::update_tunnel)
                .delete(tunnels::delete_tunnel),
        )
        .route("/api/v1/nodes/{id}/tunnels", get(tunnels::list_node_tunnels))
        // Topology
        .route("/api/v1/topology/positions", get(topology::get_positions).put(topology::save_positions).delete(topology::clear_positions))
        // AI
        .route("/api/v1/ai/generate-config", post(ai::generate_config))
        .route("/api/v1/ai/analyze", post(ai::analyze_anomaly))
        .route("/api/v1/ai/query", post(ai::answer_query))
        .route(
            "/api/v1/ai/keys",
            get(ai::list_keys).post(ai::set_key).delete(ai::delete_key),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::auth::auth_middleware,
        ));

    Router::new().merge(public).merge(authenticated)
}

async fn health_check(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "service": "bilbycast-manager",
        "version": env!("CARGO_PKG_VERSION"),
        "self_signed_cert": state.is_self_signed_cert
    }))
}
