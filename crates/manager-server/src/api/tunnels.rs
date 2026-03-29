// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;

use hmac::{Hmac, Mac};
use rand::RngExt;
use sha2::Sha256;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::db;
use manager_core::models::tunnel::*;
use manager_core::models::UserRole;
use manager_core::validation;

type HmacSha256 = Hmac<Sha256>;

/// Compute an HMAC-SHA256 bind token for relay tunnel authentication.
fn compute_bind_token(tunnel_id: &str, direction: &str, bind_secret: &str) -> String {
    let identity = format!("{tunnel_id}:{direction}");
    let mut mac =
        HmacSha256::new_from_slice(bind_secret.as_bytes()).expect("HMAC key can be any length");
    mac.update(identity.as_bytes());
    let result = mac.finalize();
    result
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

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
    if let Some(ref addr) = req.egress_peer_addr {
        validation::validate_addr(addr, "egress_peer_addr")
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

    if matches!(req.mode, TunnelMode::Direct) {
        if req.egress_peer_addr.is_none() {
            return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "egress_peer_addr is required for direct mode (reachable address of egress node's QUIC listener)"}))));
        }
    }

    // ── Generate tunnel encryption key (used for end-to-end encryption) ──
    // Both relay and direct modes use a shared symmetric key (ChaCha20-Poly1305).
    // The manager generates the key and distributes it to both edges.

    let tunnel_encryption_key = generate_random_key();

    // Encrypt the tunnel key at rest using the master key
    let tunnel_key_enc = manager_core::crypto::encrypt(&tunnel_encryption_key, &state.master_key)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to encrypt tunnel key"}))))?;

    // Generate bind secret for relay tunnel authentication
    let tunnel_bind_secret = generate_random_key();
    let tunnel_bind_secret_enc = manager_core::crypto::encrypt(&tunnel_bind_secret, &state.master_key)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to encrypt bind secret"}))))?;

    // For direct mode, also generate a PSK for QUIC transport authentication
    let tunnel_psk = if matches!(req.mode, TunnelMode::Direct) {
        Some(generate_random_key())
    } else {
        None
    };
    let tunnel_psk_enc = match &tunnel_psk {
        Some(psk) => Some(manager_core::crypto::encrypt(psk, &state.master_key)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to encrypt tunnel PSK"}))))?),
        None => None,
    };

    let secrets = db::tunnels::TunnelSecrets {
        tunnel_key_enc: Some(&tunnel_key_enc),
        tunnel_bind_secret_enc: Some(&tunnel_bind_secret_enc),
        tunnel_psk_enc: tunnel_psk_enc.as_deref(),
    };

    let tunnel = db::tunnels::create_tunnel(&state.db, &req, &secrets)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to create tunnel"}))))?;

    // Build edge configs and push to both nodes
    let (entry_config, exit_config) = build_edge_configs(
        &tunnel, &tunnel_encryption_key, &tunnel_bind_secret, tunnel_psk.as_deref(),
    );

    tracing::info!(
        tunnel_id = %tunnel.id,
        mode = %tunnel.mode,
        ingress_node = %req.ingress_node_id,
        egress_node = %req.egress_node_id,
        relay_addr = ?tunnel.relay_addr,
        "Created tunnel in DB, pushing configs to both edges"
    );
    tracing::debug!(tunnel_id = %tunnel.id, entry_config = %entry_config, "Entry (ingress node) config");
    tracing::debug!(tunnel_id = %tunnel.id, exit_config = %exit_config, "Exit (egress node) config");

    let mut warnings = Vec::new();

    // Authorize on relay if relay mode
    if matches!(req.mode, TunnelMode::Relay) {
        tracing::info!(tunnel_id = %tunnel.id, "Authorizing tunnel on relay");
        if let Some(ref _relay_node_id) = tunnel.relay_node_id {
            match authorize_tunnel_on_relay_tracked(&state, &tunnel, &tunnel_bind_secret).await {
                Ok(()) => {
                    let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "relay", "pushed", None).await;
                }
                Err(e) => {
                    let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "relay", "failed", Some(&e)).await;
                    warnings.push(format!("Relay not authorized: {e}"));
                }
            }
        }
    }

    // Push to both edges
    let entry_cmd = json!({"type": "create_tunnel", "tunnel": entry_config});
    let exit_cmd = json!({"type": "create_tunnel", "tunnel": exit_config});

    tracing::info!(tunnel_id = %tunnel.id, node = %req.ingress_node_id, "Pushing entry config to ingress node");
    match state.node_hub.send_command(&req.ingress_node_id, entry_cmd).await {
        Ok(_) => {
            tracing::info!(tunnel_id = %tunnel.id, node = %req.ingress_node_id, "Ingress node configured OK");
            let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "ingress", "pushed", None).await;
        }
        Err(e) => {
            tracing::warn!("Failed to push tunnel to ingress node {}: {}", req.ingress_node_id, e);
            let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "ingress", "failed", Some(&e)).await;
            warnings.push(format!("Ingress node not configured: {e}"));
        }
    }

    tracing::info!(tunnel_id = %tunnel.id, node = %req.egress_node_id, "Pushing exit config to egress node");
    match state.node_hub.send_command(&req.egress_node_id, exit_cmd).await {
        Ok(_) => {
            tracing::info!(tunnel_id = %tunnel.id, node = %req.egress_node_id, "Egress node configured OK");
            let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "egress", "pushed", None).await;
        }
        Err(e) => {
            tracing::warn!("Failed to push tunnel to egress node {}: {}", req.egress_node_id, e);
            let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "egress", "failed", Some(&e)).await;
            warnings.push(format!("Egress node not configured: {e}"));
        }
    }

    if !warnings.is_empty() {
        let _ = db::tunnels::update_tunnel_status(&state.db, &tunnel.id, "pending").await;
    } else {
        let _ = db::tunnels::update_tunnel_status(&state.db, &tunnel.id, "active").await;
    }

    let mut result = serde_json::to_value(&tunnel).unwrap_or(json!(null));
    if !warnings.is_empty() {
        result["warnings"] = json!(warnings);
        result["status"] = json!("pending");
    } else {
        result["status"] = json!("active");
    }
    Ok(Json(result))
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
    if let Some(ref addr) = req.egress_peer_addr {
        validation::validate_addr(addr, "egress_peer_addr")
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e}))))?;
    }

    let tunnel = match db::tunnels::update_tunnel(&state.db, &id, &req).await {
        Ok(Some(tunnel)) => tunnel,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(json!({"error": "Tunnel not found"})))),
        Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to update tunnel"})))),
    };

    // Push updated config to both edge nodes.
    // First destroy the old tunnel on both edges, then re-push with updated config.
    let delete_cmd = json!({"type": "delete_tunnel", "tunnel_id": id});
    let _ = state.node_hub.send_command(&tunnel.ingress_node_id, delete_cmd.clone()).await;
    let _ = state.node_hub.send_command(&tunnel.egress_node_id, delete_cmd).await;

    // Re-push the tunnel with updated parameters
    let warnings = push_tunnel_to_both_edges(&state, &tunnel).await;

    if warnings.is_empty() {
        let _ = db::tunnels::update_tunnel_status(&state.db, &tunnel.id, "active").await;
    }

    let mut result = serde_json::to_value(&tunnel).unwrap_or(json!(null));
    if !warnings.is_empty() {
        result["warnings"] = json!(warnings);
    }
    Ok(Json(result))
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
            // Push delete to both edge nodes and revoke on relay
            if let Some(t) = tunnel {
                let cmd = json!({"type": "delete_tunnel", "tunnel_id": id});
                let _ = state.node_hub.send_command(&t.ingress_node_id, cmd.clone()).await;
                let _ = state.node_hub.send_command(&t.egress_node_id, cmd).await;
                // Revoke bind authorization on relay
                if let Some(ref relay_node_id) = t.relay_node_id {
                    let revoke_cmd = json!({"type": "revoke_tunnel", "tunnel_id": id});
                    let _ = state.node_hub.send_command(relay_node_id, revoke_cmd).await;
                }
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

// ── Shared helpers for building edge tunnel configs ──

/// Build the entry (ingress-node) and exit (egress-node) tunnel configs from plaintext secrets.
///
/// Manager naming vs edge direction:
/// - "ingress_node" (entry point, where traffic enters) → edge direction = "egress"
///   (edge captures local traffic on listen port and sends INTO the tunnel)
/// - "egress_node" (exit point, where traffic exits) → edge direction = "ingress"
///   (edge receives FROM the tunnel and forwards to the local address)
fn build_edge_configs(
    tunnel: &manager_core::models::tunnel::Tunnel,
    tunnel_encryption_key: &str,
    tunnel_bind_secret: &str,
    tunnel_psk: Option<&str>,
) -> (serde_json::Value, serde_json::Value) {
    let protocol_str = tunnel.protocol.as_str();
    let mode_str = tunnel.mode.as_str();

    // Config for the ingress node (entry point): listens locally, sends into tunnel
    let mut entry_config = json!({
        "id": tunnel.id,
        "name": tunnel.name,
        "enabled": true,
        "protocol": protocol_str,
        "mode": mode_str,
        "direction": "egress",
        "local_addr": format!("0.0.0.0:{}", tunnel.ingress_listen_port),
    });

    // Config for the egress node (exit point): receives from tunnel, forwards locally
    let mut exit_config = json!({
        "id": tunnel.id,
        "name": tunnel.name,
        "enabled": true,
        "protocol": protocol_str,
        "mode": mode_str,
        "direction": "ingress",
        "local_addr": tunnel.egress_forward_addr,
    });

    // Both modes get the tunnel encryption key for end-to-end encryption
    entry_config["tunnel_encryption_key"] = json!(tunnel_encryption_key);
    exit_config["tunnel_encryption_key"] = json!(tunnel_encryption_key);

    // Both modes get the bind secret for relay authentication
    entry_config["tunnel_bind_secret"] = json!(tunnel_bind_secret);
    exit_config["tunnel_bind_secret"] = json!(tunnel_bind_secret);

    if matches!(tunnel.mode, TunnelMode::Relay) {
        entry_config["relay_addr"] = json!(tunnel.relay_addr);
        exit_config["relay_addr"] = json!(tunnel.relay_addr);
    } else if let Some(psk) = tunnel_psk {
        // Direct mode: exit node listens for QUIC, entry node connects
        if let Some(ref egress_peer_addr) = tunnel.egress_peer_addr {
            let listen_port = egress_peer_addr.rsplit(':').next().unwrap_or("0");
            exit_config["direct_listen_addr"] = json!(format!("0.0.0.0:{}", listen_port));
            entry_config["peer_addr"] = json!(egress_peer_addr);
        }
        exit_config["tunnel_psk"] = json!(psk);
        entry_config["tunnel_psk"] = json!(psk);
    }

    (entry_config, exit_config)
}

/// Pre-authorize on relay, returning Result for per-leg tracking.
async fn authorize_tunnel_on_relay_tracked(
    state: &AppState,
    tunnel: &manager_core::models::tunnel::Tunnel,
    tunnel_bind_secret: &str,
) -> Result<(), String> {
    let relay_node_id = tunnel.relay_node_id.as_deref()
        .ok_or_else(|| "No relay_node_id set".to_string())?;
    let ingress_token = compute_bind_token(&tunnel.id, "ingress", tunnel_bind_secret);
    let egress_token = compute_bind_token(&tunnel.id, "egress", tunnel_bind_secret);
    let authorize_cmd = json!({
        "type": "authorize_tunnel",
        "tunnel_id": tunnel.id,
        "ingress_token": ingress_token,
        "egress_token": egress_token,
    });
    state.node_hub.send_command(relay_node_id, authorize_cmd)
        .await
        .map(|_| ())
}

/// Decrypt tunnel secrets from the DB record. Returns (encryption_key, bind_secret, psk).
fn decrypt_tunnel_secrets(
    tunnel: &manager_core::models::tunnel::Tunnel,
    master_key: &[u8; 32],
) -> Result<(String, String, Option<String>), String> {
    let tunnel_encryption_key = tunnel.tunnel_key_enc.as_deref()
        .ok_or("No tunnel encryption key stored")?;
    let tunnel_encryption_key = manager_core::crypto::decrypt(tunnel_encryption_key, master_key)
        .map_err(|e| format!("Failed to decrypt tunnel key: {e}"))?;

    let tunnel_bind_secret = tunnel.tunnel_bind_secret_enc.as_deref()
        .ok_or("No tunnel bind secret stored")?;
    let tunnel_bind_secret = manager_core::crypto::decrypt(tunnel_bind_secret, master_key)
        .map_err(|e| format!("Failed to decrypt bind secret: {e}"))?;

    let tunnel_psk = match tunnel.tunnel_psk_enc.as_deref() {
        Some(enc) => Some(manager_core::crypto::decrypt(enc, master_key)
            .map_err(|e| format!("Failed to decrypt PSK: {e}"))?),
        None => None,
    };

    Ok((tunnel_encryption_key, tunnel_bind_secret, tunnel_psk))
}

/// Push a tunnel config to a specific node. Called on reconnection.
/// `node_id` must be the tunnel's ingress_node_id, egress_node_id, or relay_node_id.
/// For relay nodes, sends `authorize_tunnel` instead of `create_tunnel`.
/// Updates per-leg push status in the DB.
pub async fn push_tunnel_to_node(
    state: &AppState,
    tunnel: &manager_core::models::tunnel::Tunnel,
    node_id: &str,
) -> Result<(), String> {
    // Relay node: re-authorize bind tokens instead of pushing edge config
    if tunnel.relay_node_id.as_deref() == Some(node_id) {
        let tunnel_bind_secret = tunnel.tunnel_bind_secret_enc.as_deref()
            .ok_or("No tunnel bind secret stored")?;
        let tunnel_bind_secret = manager_core::crypto::decrypt(tunnel_bind_secret, &state.master_key)
            .map_err(|e| format!("Failed to decrypt bind secret: {e}"))?;

        let ingress_token = compute_bind_token(&tunnel.id, "ingress", &tunnel_bind_secret);
        let egress_token = compute_bind_token(&tunnel.id, "egress", &tunnel_bind_secret);
        let authorize_cmd = json!({
            "type": "authorize_tunnel",
            "tunnel_id": tunnel.id,
            "ingress_token": ingress_token,
            "egress_token": egress_token,
        });
        match state.node_hub.send_command(node_id, authorize_cmd).await {
            Ok(_) => {
                let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "relay", "pushed", None).await;
                return Ok(());
            }
            Err(e) => {
                let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "relay", "failed", Some(&e)).await;
                return Err(e);
            }
        }
    }

    // Edge node: push create_tunnel with the appropriate config
    let (tunnel_encryption_key, tunnel_bind_secret, tunnel_psk) =
        decrypt_tunnel_secrets(tunnel, &state.master_key)?;

    let (entry_config, exit_config) = build_edge_configs(
        tunnel, &tunnel_encryption_key, &tunnel_bind_secret, tunnel_psk.as_deref(),
    );

    // Determine which config and leg name this node gets
    let (config, leg) = if node_id == tunnel.ingress_node_id {
        (entry_config, "ingress")
    } else if node_id == tunnel.egress_node_id {
        (exit_config, "egress")
    } else {
        return Err(format!("Node {node_id} is not part of tunnel {}", tunnel.id));
    };

    let cmd = json!({"type": "create_tunnel", "tunnel": config});
    match state.node_hub.send_command(node_id, cmd).await {
        Ok(_) => {
            let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, leg, "pushed", None).await;
            Ok(())
        }
        Err(e) => {
            let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, leg, "failed", Some(&e)).await;
            Err(e)
        }
    }
}

/// Push a tunnel to both edges and optionally authorize on relay.
/// Updates per-leg push status in the DB.
pub async fn push_tunnel_to_both_edges(
    state: &AppState,
    tunnel: &manager_core::models::tunnel::Tunnel,
) -> Vec<String> {
    let (tunnel_encryption_key, tunnel_bind_secret, tunnel_psk) =
        match decrypt_tunnel_secrets(tunnel, &state.master_key) {
            Ok(secrets) => secrets,
            Err(e) => return vec![e],
        };

    let (entry_config, exit_config) = build_edge_configs(
        tunnel, &tunnel_encryption_key, &tunnel_bind_secret, tunnel_psk.as_deref(),
    );

    let mut warnings = Vec::new();

    // Authorize on relay if relay mode
    if matches!(tunnel.mode, TunnelMode::Relay) {
        match authorize_tunnel_on_relay_tracked(state, tunnel, &tunnel_bind_secret).await {
            Ok(()) => {
                let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "relay", "pushed", None).await;
            }
            Err(e) => {
                let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "relay", "failed", Some(&e)).await;
                warnings.push(format!("Relay not authorized: {e}"));
            }
        }
    }

    let entry_cmd = json!({"type": "create_tunnel", "tunnel": entry_config});
    let exit_cmd = json!({"type": "create_tunnel", "tunnel": exit_config});

    match state.node_hub.send_command(&tunnel.ingress_node_id, entry_cmd).await {
        Ok(_) => {
            let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "ingress", "pushed", None).await;
        }
        Err(e) => {
            tracing::warn!("Failed to push tunnel to ingress node {}: {e}", tunnel.ingress_node_id);
            let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "ingress", "failed", Some(&e)).await;
            warnings.push(format!("Ingress node not configured: {e}"));
        }
    }
    match state.node_hub.send_command(&tunnel.egress_node_id, exit_cmd).await {
        Ok(_) => {
            let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "egress", "pushed", None).await;
        }
        Err(e) => {
            tracing::warn!("Failed to push tunnel to egress node {}: {e}", tunnel.egress_node_id);
            let _ = db::tunnels::update_tunnel_push_status(&state.db, &tunnel.id, "egress", "failed", Some(&e)).await;
            warnings.push(format!("Egress node not configured: {e}"));
        }
    }

    warnings
}

/// Retry pushing pending/failed tunnel legs to connected nodes.
/// Called periodically by the background retry task.
pub async fn retry_pending_tunnels(state: &AppState) {
    let tunnels = match db::tunnels::list_pending_tunnels(&state.db).await {
        Ok(t) => t,
        Err(e) => {
            tracing::debug!("Failed to list pending tunnels for retry: {e}");
            return;
        }
    };

    if tunnels.is_empty() {
        return;
    }

    tracing::info!("Retrying {} tunnel(s) with pending/failed legs", tunnels.len());

    for tunnel in &tunnels {
        let mut all_pushed = true;

        // Check and retry ingress leg
        if tunnel.ingress_push_status != "pushed" {
            if state.node_hub.connections.contains_key(&tunnel.ingress_node_id) {
                match push_tunnel_to_node(state, tunnel, &tunnel.ingress_node_id).await {
                    Ok(()) => tracing::info!(tunnel_id = %tunnel.id, "Retry: ingress leg pushed OK"),
                    Err(e) => {
                        tracing::warn!(tunnel_id = %tunnel.id, "Retry: ingress leg failed: {e}");
                        all_pushed = false;
                    }
                }
            } else {
                all_pushed = false;
            }
        }

        // Check and retry egress leg
        if tunnel.egress_push_status != "pushed" {
            if state.node_hub.connections.contains_key(&tunnel.egress_node_id) {
                match push_tunnel_to_node(state, tunnel, &tunnel.egress_node_id).await {
                    Ok(()) => tracing::info!(tunnel_id = %tunnel.id, "Retry: egress leg pushed OK"),
                    Err(e) => {
                        tracing::warn!(tunnel_id = %tunnel.id, "Retry: egress leg failed: {e}");
                        all_pushed = false;
                    }
                }
            } else {
                all_pushed = false;
            }
        }

        // Check and retry relay leg
        if let Some(ref relay_status) = tunnel.relay_push_status {
            if relay_status != "pushed" {
                if let Some(ref relay_node_id) = tunnel.relay_node_id {
                    if state.node_hub.connections.contains_key(relay_node_id) {
                        match push_tunnel_to_node(state, tunnel, relay_node_id).await {
                            Ok(()) => tracing::info!(tunnel_id = %tunnel.id, "Retry: relay leg pushed OK"),
                            Err(e) => {
                                tracing::warn!(tunnel_id = %tunnel.id, "Retry: relay leg failed: {e}");
                                all_pushed = false;
                            }
                        }
                    } else {
                        all_pushed = false;
                    }
                }
            }
        }

        // Update overall status if all legs are now pushed
        if all_pushed {
            let _ = db::tunnels::update_tunnel_status(&state.db, &tunnel.id, "active").await;
            tracing::info!(tunnel_id = %tunnel.id, "All tunnel legs pushed — marking active");
        }
    }
}
