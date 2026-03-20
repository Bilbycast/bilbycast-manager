use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

use crate::app_state::AppState;
use manager_core::db;
use manager_core::models::tunnel::*;

pub async fn list_tunnels(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    match db::tunnels::list_tunnels(&state.db).await {
        Ok(tunnels) => Json(json!({ "tunnels": tunnels })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn get_tunnel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match db::tunnels::get_tunnel(&state.db, &id).await {
        Ok(Some(tunnel)) => Json(serde_json::to_value(tunnel).unwrap_or(json!(null))),
        Ok(None) => Json(json!({ "error": "tunnel not found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn create_tunnel(
    State(state): State<AppState>,
    Json(req): Json<CreateTunnelRequest>,
) -> Json<serde_json::Value> {
    // Validate relay_addr is provided for relay mode
    if matches!(req.mode, TunnelMode::Relay) && req.relay_addr.is_none() {
        return Json(json!({ "error": "relay_addr is required for relay mode" }));
    }

    match db::tunnels::create_tunnel(&state.db, &req).await {
        Ok(tunnel) => Json(serde_json::to_value(tunnel).unwrap_or(json!(null))),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn update_tunnel(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTunnelRequest>,
) -> Json<serde_json::Value> {
    match db::tunnels::update_tunnel(&state.db, &id, &req).await {
        Ok(Some(tunnel)) => Json(serde_json::to_value(tunnel).unwrap_or(json!(null))),
        Ok(None) => Json(json!({ "error": "tunnel not found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn delete_tunnel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match db::tunnels::delete_tunnel(&state.db, &id).await {
        Ok(true) => Json(json!({ "deleted": true })),
        Ok(false) => Json(json!({ "error": "tunnel not found" })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

pub async fn list_node_tunnels(
    State(state): State<AppState>,
    Path(node_id): Path<String>,
) -> Json<serde_json::Value> {
    match db::tunnels::list_tunnels_for_node(&state.db, &node_id).await {
        Ok(tunnels) => Json(json!({ "tunnels": tunnels })),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}
