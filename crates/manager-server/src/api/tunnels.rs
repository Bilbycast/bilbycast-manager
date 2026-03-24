// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;

use rand::RngExt;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::db;
use manager_core::models::tunnel::*;
use manager_core::models::UserRole;
use manager_core::validation;

/// Generate a cryptographically random 32-byte key (hex-encoded, 64 chars).
/// Used for tunnel encryption keys (ChaCha20-Poly1305) and direct-mode PSKs.
fn generate_random_key() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

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

    if matches!(req.mode, TunnelMode::Relay) {
        if req.relay_addr.is_none() {
            return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "relay_addr is required for relay mode"}))));
        }
        if req.relay_node_id.is_none() {
            return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "relay_node_id is required for relay mode"}))));
        }
    }

    // ── Generate tunnel encryption key (used for end-to-end encryption) ──
    // Both relay and direct modes use a shared symmetric key (ChaCha20-Poly1305).
    // The manager generates the key and distributes it to both edges.

    let tunnel_encryption_key = generate_random_key();

    // Encrypt the tunnel key at rest using the master key
    let tunnel_key_enc = manager_core::crypto::encrypt(&tunnel_encryption_key, &state.master_key)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to encrypt tunnel key"}))))?;

    // For direct mode, also generate a PSK for QUIC transport authentication
    let tunnel_psk = if matches!(req.mode, TunnelMode::Direct) {
        Some(generate_random_key())
    } else {
        None
    };

    let tunnel = db::tunnels::create_tunnel(&state.db, &req, Some(&tunnel_key_enc))
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to create tunnel"}))))?;

    // Push tunnel config to both edge nodes via WebSocket.
    //
    // Manager naming vs edge direction:
    // - "ingress_node" (entry point, where traffic enters) → edge direction = "egress"
    //   (edge captures local traffic on listen port and sends INTO the tunnel)
    // - "egress_node" (exit point, where traffic exits) → edge direction = "ingress"
    //   (edge receives FROM the tunnel and forwards to the local address)
    let protocol_str = req.protocol.as_str();
    let mode_str = req.mode.as_str();

    // Config for the ingress node (entry point): listens locally, sends into tunnel
    let mut entry_config = json!({
        "id": tunnel.id,
        "name": tunnel.name,
        "enabled": true,
        "protocol": protocol_str,
        "mode": mode_str,
        "direction": "egress",
        "local_addr": format!("0.0.0.0:{}", req.ingress_listen_port),
    });

    // Config for the egress node (exit point): receives from tunnel, forwards locally
    let mut exit_config = json!({
        "id": tunnel.id,
        "name": tunnel.name,
        "enabled": true,
        "protocol": protocol_str,
        "mode": mode_str,
        "direction": "ingress",
        "local_addr": req.egress_forward_addr,
    });

    // Both modes get the tunnel encryption key for end-to-end encryption
    entry_config["tunnel_encryption_key"] = json!(tunnel_encryption_key);
    exit_config["tunnel_encryption_key"] = json!(tunnel_encryption_key);

    if matches!(req.mode, TunnelMode::Relay) {
        // Relay is stateless — no auth needed, edges just connect
        entry_config["relay_addr"] = json!(req.relay_addr);
        exit_config["relay_addr"] = json!(req.relay_addr);
    } else {
        // Direct mode: exit node listens for QUIC, entry node connects
        let psk = tunnel_psk.as_ref().unwrap();
        exit_config["direct_listen_addr"] = json!(format!("0.0.0.0:{}", req.ingress_listen_port + 1000));
        exit_config["tunnel_psk"] = json!(psk);
        entry_config["peer_addr"] = json!(format!("{}:{}", req.egress_forward_addr.split(':').next().unwrap_or("127.0.0.1"), req.ingress_listen_port + 1000));
        entry_config["tunnel_psk"] = json!(psk);
    }

    let entry_cmd = json!({"type": "create_tunnel", "tunnel": entry_config});
    let exit_cmd = json!({"type": "create_tunnel", "tunnel": exit_config});

    if let Err(e) = state.node_hub.send_command(&req.ingress_node_id, entry_cmd).await {
        tracing::warn!("Failed to push tunnel to entry node {}: {}", req.ingress_node_id, e);
    }
    if let Err(e) = state.node_hub.send_command(&req.egress_node_id, exit_cmd).await {
        tracing::warn!("Failed to push tunnel to exit node {}: {}", req.egress_node_id, e);
    }

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

    // Fetch tunnel before deleting to get node IDs
    let tunnel = db::tunnels::get_tunnel(&state.db, &id)
        .await
        .ok()
        .flatten();

    match db::tunnels::delete_tunnel(&state.db, &id).await {
        Ok(true) => {
            // Push delete to both edge nodes
            if let Some(t) = tunnel {
                let cmd = json!({"type": "delete_tunnel", "tunnel_id": id});
                let _ = state.node_hub.send_command(&t.ingress_node_id, cmd.clone()).await;
                let _ = state.node_hub.send_command(&t.egress_node_id, cmd).await;
            }
            Ok(Json(json!({ "deleted": true })))
        }
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
