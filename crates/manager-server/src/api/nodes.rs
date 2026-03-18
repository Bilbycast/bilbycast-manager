use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::models::{CreateNodeRequest, Node, UpdateNodeRequest, UserRole};

pub async fn list_nodes(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Node>>, StatusCode> {
    let nodes = manager_core::db::nodes::list_nodes(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Filter by user's allowed nodes
    let filtered: Vec<Node> = nodes
        .into_iter()
        .filter(|n| auth.can_access_node(&n.id))
        .collect();

    Ok(Json(filtered))
}

pub async fn create_node(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateNodeRequest>,
) -> Result<(StatusCode, Json<Node>), StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    let node = manager_core::db::nodes::create_node(&state.db, &req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = manager_core::db::audit::log_audit(
        &state.db,
        Some(&auth.user_id),
        "node.create",
        Some("node"),
        Some(&node.id),
        None,
        None,
    )
    .await;

    Ok((StatusCode::CREATED, Json(node)))
}

pub async fn get_node(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Node>, StatusCode> {
    if !auth.can_access_node(&id) {
        return Err(StatusCode::FORBIDDEN);
    }

    let node = manager_core::db::nodes::get_node_by_id(&state.db, &id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(node))
}

pub async fn update_node(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateNodeRequest>,
) -> Result<Json<Node>, StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    let node = manager_core::db::nodes::update_node(&state.db, &id, &req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(node))
}

pub async fn delete_node(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Disconnect the node if connected
    state.node_hub.disconnect_node(&id).await;

    manager_core::db::nodes::delete_node(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = manager_core::db::audit::log_audit(
        &state.db,
        Some(&auth.user_id),
        "node.delete",
        Some("node"),
        Some(&id),
        None,
        None,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn regenerate_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    let token = manager_core::db::nodes::regenerate_token(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "registration_token": token
    })))
}

pub async fn get_node_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.can_access_node(&id) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Get cached config from node hub
    match state.node_hub.get_cached_config(&id).await {
        Some(config) => Ok(Json(config)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Deserialize)]
pub struct NodeCommand {
    pub action: serde_json::Value,
}

pub async fn send_command(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(cmd): Json<NodeCommand>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::Operator) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !auth.can_access_node(&id) {
        return Err(StatusCode::FORBIDDEN);
    }

    match state.node_hub.send_command(&id, cmd.action).await {
        Ok(ack) => Ok(Json(ack)),
        Err(e) => {
            tracing::error!("Failed to send command to node {id}: {e}");
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}

/// Proxy flow operations directly to the edge node's HTTP API.
/// More reliable than WebSocket commands for flow CRUD.
pub async fn proxy_flow_create(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(flow): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !auth.role.has_permission(UserRole::Operator) {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Forbidden"}))));
    }
    if !auth.can_access_node(&id) {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "No access to this node"}))));
    }

    let api_addr = get_node_api_addr(&state, &id).await
        .ok_or_else(|| (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": "Node API address not available. Is the node connected?"}))))?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/v1/flows", api_addr);

    let resp = client.post(&url)
        .json(&flow)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": format!("Failed to reach node: {e}")}))))?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await
        .unwrap_or_else(|_| serde_json::json!({"error": "Invalid response from node"}));

    if status.is_success() {
        Ok(Json(body))
    } else {
        Err((StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY), Json(body)))
    }
}

/// Proxy flow delete directly to the edge node's HTTP API.
pub async fn proxy_flow_delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, flow_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !auth.role.has_permission(UserRole::Operator) {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Forbidden"}))));
    }
    if !auth.can_access_node(&id) {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "No access to this node"}))));
    }

    let api_addr = get_node_api_addr(&state, &id).await
        .ok_or_else(|| (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": "Node API address not available"}))))?;

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/v1/flows/{}", api_addr, flow_id);

    let resp = client.delete(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": format!("Failed to reach node: {e}")}))))?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await
        .unwrap_or_else(|_| serde_json::json!({"success": true}));

    if status.is_success() {
        Ok(Json(body))
    } else {
        Err((StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY), Json(body)))
    }
}

/// Get the node's HTTP API address from its cached health data.
async fn get_node_api_addr(state: &AppState, node_id: &str) -> Option<String> {
    let node = manager_core::db::nodes::get_node_by_id(&state.db, node_id).await.ok()?;
    let health = node.last_health?;
    health["api_addr"].as_str().map(|s| s.to_string())
}
