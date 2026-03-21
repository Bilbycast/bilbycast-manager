use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::models::{CreateNodeRequest, Node, UpdateNodeRequest, UserRole};

#[derive(Deserialize, Default)]
pub struct ListNodesQuery {
    pub device_type: Option<String>,
}

pub async fn list_nodes(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListNodesQuery>,
) -> Result<Json<Vec<Node>>, StatusCode> {
    let nodes = manager_core::db::nodes::list_nodes(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Filter by user's allowed nodes and optional device_type
    let filtered: Vec<Node> = nodes
        .into_iter()
        .filter(|n| auth.can_access_node(&n.id))
        .filter(|n| {
            query
                .device_type
                .as_ref()
                .map(|dt| n.device_type == *dt)
                .unwrap_or(true)
        })
        .collect();

    Ok(Json(filtered))
}

/// List all registered device drivers with their capabilities.
pub async fn list_device_types(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let drivers: Vec<serde_json::Value> = state
        .driver_registry
        .all()
        .iter()
        .map(|d| {
            serde_json::json!({
                "device_type": d.device_type(),
                "display_name": d.display_name(),
                "supported_commands": d.supported_commands(),
            })
        })
        .collect();

    Json(serde_json::json!({ "device_types": drivers }))
}

pub async fn create_node(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateNodeRequest>,
) -> Result<(StatusCode, Json<Node>), StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate device_type against registered drivers
    let device_type = req.device_type.as_deref().unwrap_or("edge");
    if !state.driver_registry.is_registered(device_type) {
        tracing::warn!("Rejected node creation with unknown device_type: {device_type}");
        return Err(StatusCode::BAD_REQUEST);
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

    // Request config from node (sends GetConfig command if not cached, waits up to 3s)
    match state.node_hub.request_config(&id).await {
        Ok(Some(config)) => Ok(Json(config)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    }
}

pub async fn update_node_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(config): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::Operator) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !auth.can_access_node(&id) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Send UpdateConfig command to the node via WebSocket
    match state
        .node_hub
        .send_command(
            &id,
            serde_json::json!({"type": "update_config", "config": config}),
        )
        .await
    {
        Ok(ack) => {
            let _ = manager_core::db::audit::log_audit(
                &state.db,
                Some(&auth.user_id),
                "node.config.update",
                Some("node"),
                Some(&id),
                None,
                None,
            )
            .await;
            Ok(Json(ack))
        }
        Err(e) => {
            tracing::error!("Failed to send config update to node {id}: {e}");
            Err(StatusCode::BAD_GATEWAY)
        }
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

/// Create a flow on a node via WebSocket command.
/// Uses the persistent WebSocket connection so it works with nodes behind firewalls/NAT.
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

    match state.node_hub.send_command(&id, serde_json::json!({"type": "create_flow", "flow": flow})).await {
        Ok(ack) => Ok(Json(ack)),
        Err(e) => Err((StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": format!("Failed to send command: {e}")})))),
    }
}

/// Delete a flow on a node via WebSocket command.
/// Uses the persistent WebSocket connection so it works with nodes behind firewalls/NAT.
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

    match state.node_hub.send_command(&id, serde_json::json!({"type": "delete_flow", "flow_id": flow_id})).await {
        Ok(ack) => Ok(Json(ack)),
        Err(e) => Err((StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": format!("Failed to send command: {e}")})))),
    }
}
