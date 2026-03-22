use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::db;
use manager_core::models::tunnel::*;
use manager_core::models::UserRole;
use manager_core::validation;

pub async fn list_tunnels(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::Operator) {
        return Err(StatusCode::FORBIDDEN);
    }

    let tunnels = db::tunnels::list_tunnels(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "tunnels": tunnels })))
}

pub async fn get_tunnel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::Operator) {
        return Err(StatusCode::FORBIDDEN);
    }

    match db::tunnels::get_tunnel(&state.db, &id).await {
        Ok(Some(tunnel)) => Ok(Json(serde_json::to_value(tunnel).unwrap_or(json!(null)))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn create_tunnel(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateTunnelRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Forbidden"}))));
    }

    // Input validation
    validation::validate_name(&req.name, "tunnel name", 128)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;
    validation::validate_addr(&req.egress_forward_addr, "egress_forward_addr")
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;
    if let Some(ref addr) = req.relay_addr {
        validation::validate_addr(addr, "relay_addr")
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;
    }

    if matches!(req.mode, TunnelMode::Relay) && req.relay_addr.is_none() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "relay_addr is required for relay mode"}))));
    }

    let tunnel = db::tunnels::create_tunnel(&state.db, &req)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to create tunnel"}))))?;

    Ok(Json(serde_json::to_value(tunnel).unwrap_or(json!(null))))
}

pub async fn update_tunnel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateTunnelRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Forbidden"}))));
    }

    // Input validation
    if let Some(ref name) = req.name {
        validation::validate_name(name, "tunnel name", 128)
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;
    }
    if let Some(ref addr) = req.egress_forward_addr {
        validation::validate_addr(addr, "egress_forward_addr")
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;
    }
    if let Some(ref addr) = req.relay_addr {
        validation::validate_addr(addr, "relay_addr")
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;
    }

    match db::tunnels::update_tunnel(&state.db, &id, &req).await {
        Ok(Some(tunnel)) => Ok(Json(serde_json::to_value(tunnel).unwrap_or(json!(null)))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(json!({"error": "Tunnel not found"})))),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to update tunnel"})))),
    }
}

pub async fn delete_tunnel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    match db::tunnels::delete_tunnel(&state.db, &id).await {
        Ok(true) => Ok(Json(json!({ "deleted": true }))),
        Ok(false) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn list_node_tunnels(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(node_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::Operator) {
        return Err(StatusCode::FORBIDDEN);
    }

    if !auth.can_access_node(&node_id) {
        return Err(StatusCode::FORBIDDEN);
    }

    let tunnels = db::tunnels::list_tunnels_for_node(&state.db, &node_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "tunnels": tunnels })))
}
