// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use std::sync::Arc;
use std::time::Instant;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use chrono::Utc;
use dashmap::DashMap;
use tokio::sync::{broadcast, mpsc, oneshot};
use uuid::Uuid;

use crate::app_state::AppState;
use manager_core::models::{CommandAckPayload, EventSeverity, NodeStatus, WsEnvelope};

// ───────────────────────────────────────────────────────
// Node auth rate limiter / lockout
// ───────────────────────────────────────────────────────

/// Tracks failed authentication attempts per identifier (node_id or IP).
/// After `max_failures` within `window_secs`, the identifier is locked out
/// for the remainder of the window.
pub struct NodeAuthLimiter {
    /// Map of identifier -> (failure_count, first_failure_time)
    failures: DashMap<String, (u32, Instant)>,
    max_failures: u32,
    window_secs: u64,
}

impl NodeAuthLimiter {
    pub fn new(max_failures: u32, window_secs: u64) -> Self {
        Self {
            failures: DashMap::new(),
            max_failures,
            window_secs,
        }
    }

    /// Check if an identifier is currently locked out.
    /// Returns true if locked out (should reject), false if allowed.
    pub fn is_locked_out(&self, identifier: &str) -> bool {
        if let Some(entry) = self.failures.get(identifier) {
            let (count, first_failure) = *entry;
            let elapsed = first_failure.elapsed().as_secs();
            if elapsed < self.window_secs && count >= self.max_failures {
                return true;
            }
        }
        false
    }

    /// Record a failed authentication attempt. Returns true if now locked out.
    pub fn record_failure(&self, identifier: &str) -> bool {
        let now = Instant::now();
        let mut entry = self.failures.entry(identifier.to_string()).or_insert((0, now));
        let (count, first_failure) = entry.value_mut();

        // Reset window if expired
        if first_failure.elapsed().as_secs() >= self.window_secs {
            *count = 1;
            *first_failure = now;
            return false;
        }

        *count += 1;
        *count >= self.max_failures
    }

    /// Clear failure tracking for an identifier (on successful auth).
    pub fn clear(&self, identifier: &str) {
        self.failures.remove(identifier);
    }

}

// ───────────────────────────────────────────────────────
// Connected node state
// ───────────────────────────────────────────────────────

/// State for a connected device node.
pub(crate) struct ConnectedNode {
    node_id: String,
    node_name: String,
    device_type: String,
    command_tx: mpsc::Sender<String>,
    cached_config: Option<serde_json::Value>,
    cached_stats: Option<serde_json::Value>,
    cached_health: Option<serde_json::Value>,
    software_version: Option<String>,
    protocol_version: Option<u32>,
    connected_at: chrono::DateTime<chrono::Utc>,
}

// ───────────────────────────────────────────────────────
// NodeHub
// ───────────────────────────────────────────────────────

/// Hub managing all device node WebSocket connections.
pub struct NodeHub {
    pub(crate) connections: DashMap<String, ConnectedNode>,
    browser_tx: broadcast::Sender<String>,
    auth_limiter: Arc<NodeAuthLimiter>,
    pending_commands: DashMap<String, oneshot::Sender<CommandAckPayload>>,
}

impl NodeHub {
    pub fn new(
        browser_tx: broadcast::Sender<String>,
        auth_limiter: Arc<NodeAuthLimiter>,
    ) -> Self {
        Self {
            connections: DashMap::new(),
            browser_tx,
            auth_limiter,
            pending_commands: DashMap::new(),
        }
    }

    /// Queue a command to a connected node without waiting for acknowledgement.
    /// Used internally by `request_config` which has its own polling logic.
    async fn queue_command(
        &self,
        node_id: &str,
        action: serde_json::Value,
    ) -> Result<String, String> {
        let conn = self
            .connections
            .get(node_id)
            .ok_or_else(|| format!("Node {node_id} is not connected"))?;

        let command_id = Uuid::new_v4().to_string();
        let envelope = WsEnvelope::new(
            "command",
            serde_json::json!({
                "command_id": command_id,
                "action": action
            }),
        );

        let msg = serde_json::to_string(&envelope).map_err(|e| e.to_string())?;
        conn.command_tx
            .send(msg)
            .await
            .map_err(|e| e.to_string())?;

        Ok(command_id)
    }

    /// Send a command to a connected node and wait for the acknowledgement.
    /// Returns the actual success/failure from the node, not just "sent".
    pub async fn send_command(
        &self,
        node_id: &str,
        action: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // Create oneshot channel for the ack
        let (tx, rx) = oneshot::channel::<CommandAckPayload>();

        // Queue the command (this also validates the node is connected)
        let command_id = self.queue_command(node_id, action).await?;

        // Store the sender so the ack handler can resolve it
        self.pending_commands.insert(command_id.clone(), tx);

        // Wait for ack with 10s timeout
        match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
            Ok(Ok(ack)) => {
                if ack.success {
                    let mut result = serde_json::json!({
                        "command_id": ack.command_id,
                        "success": true
                    });
                    if let Some(data) = ack.data {
                        result["data"] = data;
                    }
                    Ok(result)
                } else {
                    Err(ack.error.unwrap_or_else(|| "Command failed on node".into()))
                }
            }
            Ok(Err(_)) => {
                // Sender was dropped (entry removed from pending_commands or node disconnected)
                self.pending_commands.remove(&command_id);
                Err("Node disconnected before acknowledging command".into())
            }
            Err(_) => {
                // Timeout
                self.pending_commands.remove(&command_id);
                Err("Command timed out waiting for node response (10s)".into())
            }
        }
    }

    /// Get cached config for a node.
    pub async fn get_cached_config(&self, node_id: &str) -> Option<serde_json::Value> {
        self.connections
            .get(node_id)
            .and_then(|c| c.cached_config.clone())
    }

    /// Request config from a connected node and wait for the response.
    /// Sends a GetConfig command and polls for the cached config_response
    /// for up to `timeout_ms` milliseconds.
    pub async fn request_config(&self, node_id: &str) -> Result<Option<serde_json::Value>, String> {
        // If config is already cached, return it immediately
        if let Some(config) = self.get_cached_config(node_id).await {
            return Ok(Some(config));
        }

        // Send GetConfig command (fire-and-forget — we poll for the config_response below)
        self.queue_command(node_id, serde_json::json!({"type": "get_config"})).await?;

        // Poll for up to 3 seconds for the config_response to arrive
        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if let Some(config) = self.get_cached_config(node_id).await {
                return Ok(Some(config));
            }
        }
        Ok(None)
    }

    /// Get a summary of all flow endpoints from online edge nodes.
    /// Reads only from cached configs — no WebSocket commands are sent.
    /// Used by the endpoint discovery UI to let users select compatible
    /// remote addresses from other nodes instead of typing them manually.
    pub fn get_all_flow_endpoints(&self) -> serde_json::Value {
        let mut result = Vec::new();
        for entry in self.connections.iter() {
            let node = entry.value();
            if node.device_type != "edge" {
                continue;
            }
            let config = match &node.cached_config {
                Some(c) => c,
                None => continue,
            };
            let flows = match config.get("flows").and_then(|f| f.as_array()) {
                Some(f) => f,
                None => continue,
            };
            let mut flow_summaries = Vec::new();
            for flow in flows {
                let flow_id = flow.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let flow_name = flow.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let enabled = flow.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);

                // Extract input summary
                let input_summary = flow.get("input").map(|inp| {
                    let itype = inp.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    let mut summary = serde_json::json!({"type": itype});
                    if let Some(v) = inp.get("mode").and_then(|v| v.as_str()) {
                        summary["mode"] = serde_json::json!(v);
                    }
                    if let Some(v) = inp.get("local_addr").and_then(|v| v.as_str()) {
                        summary["local_addr"] = serde_json::json!(v);
                    }
                    if let Some(v) = inp.get("bind_addr").and_then(|v| v.as_str()) {
                        summary["bind_addr"] = serde_json::json!(v);
                    }
                    if let Some(v) = inp.get("remote_addr").and_then(|v| v.as_str()) {
                        summary["remote_addr"] = serde_json::json!(v);
                    }
                    if let Some(v) = inp.get("listen_addr").and_then(|v| v.as_str()) {
                        summary["listen_addr"] = serde_json::json!(v);
                    }
                    summary
                });

                // Extract output summaries
                let output_summaries: Vec<serde_json::Value> = flow
                    .get("outputs")
                    .and_then(|o| o.as_array())
                    .map(|outputs| {
                        outputs.iter().map(|out| {
                            let otype = out.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            let mut summary = serde_json::json!({"type": otype});
                            if let Some(v) = out.get("id").and_then(|v| v.as_str()) {
                                summary["id"] = serde_json::json!(v);
                            }
                            if let Some(v) = out.get("name").and_then(|v| v.as_str()) {
                                summary["name"] = serde_json::json!(v);
                            }
                            if let Some(v) = out.get("mode").and_then(|v| v.as_str()) {
                                summary["mode"] = serde_json::json!(v);
                            }
                            if let Some(v) = out.get("local_addr").and_then(|v| v.as_str()) {
                                summary["local_addr"] = serde_json::json!(v);
                            }
                            if let Some(v) = out.get("remote_addr").and_then(|v| v.as_str()) {
                                summary["remote_addr"] = serde_json::json!(v);
                            }
                            if let Some(v) = out.get("dest_addr").and_then(|v| v.as_str()) {
                                summary["dest_addr"] = serde_json::json!(v);
                            }
                            summary
                        }).collect()
                    })
                    .unwrap_or_default();

                let mut flow_json = serde_json::json!({
                    "flow_id": flow_id,
                    "flow_name": flow_name,
                    "enabled": enabled,
                    "outputs": output_summaries,
                });
                if let Some(inp) = input_summary {
                    flow_json["input"] = inp;
                }
                flow_summaries.push(flow_json);
            }

            result.push(serde_json::json!({
                "node_id": node.node_id,
                "node_name": node.node_name,
                "flows": flow_summaries,
            }));
        }
        serde_json::json!(result)
    }

    /// Disconnect a node.
    pub async fn disconnect_node(&self, node_id: &str) {
        if let Some((_, conn)) = self.connections.remove(node_id) {
            drop(conn.command_tx);
        }
    }

    /// Broadcast aggregated stats to browser clients.
    /// Uses device drivers to extract metrics when available.
    pub fn broadcast_to_browsers(
        &self,
        driver_registry: Option<&manager_core::drivers::DriverRegistry>,
    ) {
        let mut nodes = Vec::new();
        for entry in self.connections.iter() {
            let node = entry.value();
            let uptime_secs = (Utc::now() - node.connected_at).num_seconds().max(0) as u64;

            let mut node_json = serde_json::json!({
                "node_id": node.node_id,
                "name": node.node_name,
                "device_type": node.device_type,
                "status": "online",
                "stats": node.cached_stats,
                "health": node.cached_health,
                "software_version": node.software_version,
                "protocol_version": node.protocol_version,
                "uptime_secs": uptime_secs,
            });

            // Extract driver-specific metrics if a driver is registered
            if let Some(registry) = driver_registry {
                if let Some(driver) = registry.get(&node.device_type) {
                    let empty = serde_json::Value::Null;
                    let stats = node.cached_stats.as_ref().unwrap_or(&empty);
                    let metrics = driver.extract_metrics(stats);
                    node_json["driver_metrics"] = serde_json::to_value(&metrics)
                        .unwrap_or(serde_json::Value::Null);
                }
            }

            nodes.push(node_json);
        }

        let update = serde_json::json!({
            "type": "dashboard_update",
            "nodes": nodes,
            "online_count": self.connections.len(),
        });

        let _ = self.browser_tx.send(update.to_string());
    }
}

// ───────────────────────────────────────────────────────
// WebSocket handler
// ───────────────────────────────────────────────────────

/// WebSocket handler for device node connections.
/// Credentials are NOT in query params — the node sends an "auth" message
/// as the first WebSocket frame after connecting.
pub async fn node_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_node_connection(socket, state))
}

async fn handle_node_connection(mut socket: WebSocket, state: AppState) {
    // Step 1: Wait for auth message (first message must be auth, within 10 seconds)
    let auth_result = match wait_for_auth(&mut socket, &state).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Node authentication failed: {e}");
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({"type": "auth_error", "message": e})
                        .to_string()
                        .into(),
                ))
                .await;
            return;
        }
    };

    let node_id = auth_result.node_id;

    // Send auth response (register_ack with credentials if first-time, or auth_ok)
    if let Some((ref nid, ref nsecret)) = auth_result.new_credentials {
        let ack = serde_json::json!({
            "type": "register_ack",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "payload": {
                "node_id": nid,
                "node_secret": nsecret
            }
        });
        if let Ok(json) = serde_json::to_string(&ack) {
            let _ = socket.send(Message::Text(json.into())).await;
        }
    } else {
        let ok = serde_json::json!({
            "type": "auth_ok",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        if let Ok(json) = serde_json::to_string(&ok) {
            let _ = socket.send(Message::Text(json.into())).await;
        }
    }

    // Clear any failure tracking on successful auth
    state.node_hub.auth_limiter.clear(&node_id);

    tracing::info!("Node {node_id} connected");

    // Update node status
    let _ =
        manager_core::db::nodes::update_node_status(&state.db, &node_id, NodeStatus::Online).await;

    // Log connection event
    let _ = manager_core::db::events::insert_event(
        &state.db,
        &node_id,
        EventSeverity::Info,
        "connection",
        "Node connected to manager",
        None,
        None,
    )
    .await;

    // Get node name and device type
    let (node_name, device_type) = manager_core::db::nodes::get_node_by_id(&state.db, &node_id)
        .await
        .map(|n| (n.name, n.device_type))
        .unwrap_or_else(|_| (node_id.clone(), "edge".to_string()));

    // Log protocol version mismatch warning
    if let Some(pv) = auth_result.protocol_version {
        if pv != manager_core::models::ws_protocol::WS_PROTOCOL_VERSION {
            tracing::warn!(
                "Node {node_id} protocol version ({pv}) differs from manager ({}). Consider upgrading.",
                manager_core::models::ws_protocol::WS_PROTOCOL_VERSION
            );
            let _ = manager_core::db::events::insert_event(
                &state.db,
                &node_id,
                EventSeverity::Warning,
                "compatibility",
                &format!(
                    "Node WebSocket protocol version ({pv}) differs from manager ({}). Consider upgrading.",
                    manager_core::models::ws_protocol::WS_PROTOCOL_VERSION
                ),
                None,
                None,
            )
            .await;
        }
    }

    // Create command channel
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<String>(64);

    // Register connection
    state.node_hub.connections.insert(
        node_id.clone(),
        ConnectedNode {
            node_id: node_id.clone(),
            node_name,
            device_type: device_type.clone(),
            command_tx: cmd_tx,
            cached_config: None,
            cached_stats: None,
            cached_health: None,
            software_version: None,
            protocol_version: auth_result.protocol_version,
            connected_at: Utc::now(),
        },
    );

    // Push pending/active tunnels to the reconnected node (edge and relay nodes)
    if device_type == "edge" || device_type == "relay" {
        let state_clone = state.clone();
        let nid = node_id.clone();
        tokio::spawn(async move {
            push_pending_tunnels_to_node(&state_clone, &nid).await;
        });
    }

    // Main message loop
    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_node_message(&state, &node_id, &text).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        tracing::warn!("WebSocket error from node {node_id}: {e}");
                        break;
                    }
                    _ => {}
                }
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(msg) => {
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    // Cleanup on disconnect
    tracing::info!("Node {node_id} disconnected");
    state.node_hub.connections.remove(&node_id);

    // Broadcast updated node list so dashboards/topology remove the disconnected node
    state
        .node_hub
        .broadcast_to_browsers(Some(&state.driver_registry));

    let _ =
        manager_core::db::nodes::update_node_status(&state.db, &node_id, NodeStatus::Offline)
            .await;

    // Reset tunnel push statuses so the retry task re-pushes configs on reconnection.
    // When a relay reboots it loses all in-memory state (auth tokens, bindings).
    // When an edge disconnects its tunnel tasks may have exited and removed tunnels.
    match manager_core::db::tunnels::reset_push_status_for_node(&state.db, &node_id).await {
        Ok(count) => {
            if count > 0 {
                tracing::info!(
                    "Reset {count} tunnel push status(es) for disconnected node {node_id}"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                "Failed to reset tunnel push statuses for node {node_id}: {e}"
            );
        }
    }

    let _ = manager_core::db::events::insert_event(
        &state.db,
        &node_id,
        EventSeverity::Critical,
        "connection",
        "Node disconnected from manager",
        None,
        None,
    )
    .await;
}

// ───────────────────────────────────────────────────────
// Authentication (via WebSocket message, not query params)
// ───────────────────────────────────────────────────────

struct AuthResult {
    node_id: String,
    new_credentials: Option<(String, String)>,
    protocol_version: Option<u32>,
}

/// Wait for the first WebSocket message which must be an auth frame.
/// Times out after 10 seconds.
async fn wait_for_auth(socket: &mut WebSocket, state: &AppState) -> Result<AuthResult, String> {
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), socket.recv()).await;

    let msg = match timeout {
        Ok(Some(Ok(Message::Text(text)))) => text,
        Ok(Some(Ok(_))) => return Err("First message must be a text auth frame".into()),
        Ok(Some(Err(e))) => return Err(format!("WebSocket error: {e}")),
        Ok(None) => return Err("Connection closed before auth".into()),
        Err(_) => return Err("Auth timeout (10s)".into()),
    };

    let auth: serde_json::Value =
        serde_json::from_str(&msg).map_err(|e| format!("Invalid auth JSON: {e}"))?;

    let msg_type = auth["type"].as_str().unwrap_or("");
    if msg_type != "auth" {
        return Err(format!(
            "Expected 'auth' message type, got '{msg_type}'"
        ));
    }

    let payload = &auth["payload"];
    let protocol_version = payload["protocol_version"].as_u64().map(|v| v as u32);

    // Check for registration token
    if let Some(token) = payload["registration_token"].as_str() {
        // Rate-limit token-based registration attempts too
        let limiter_key = format!("token:{}", &token[..token.len().min(8)]);
        if state.node_hub.auth_limiter.is_locked_out(&limiter_key) {
            return Err("Too many failed attempts. Try again later.".into());
        }

        let node = manager_core::db::nodes::get_node_by_token(&state.db, token)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| {
                state.node_hub.auth_limiter.record_failure(&limiter_key);
                "Invalid registration token".to_string()
            })?;

        // Reject expired nodes
        if node.is_expired() {
            return Err("Node has expired".into());
        }

        // Generate node secret
        let node_secret = Uuid::new_v4().to_string();
        let encrypted = manager_core::crypto::encrypt(&node_secret, &state.master_key)
            .map_err(|e| e.to_string())?;

        manager_core::db::nodes::complete_registration(&state.db, &node.id, &encrypted)
            .await
            .map_err(|e| e.to_string())?;

        state.node_hub.auth_limiter.clear(&limiter_key);

        return Ok(AuthResult {
            node_id: node.id.clone(),
            new_credentials: Some((node.id, node_secret)),
            protocol_version,
        });
    }

    // Reconnection with node_id + node_secret
    let node_id = payload["node_id"]
        .as_str()
        .ok_or("Missing node_id in auth payload")?;
    let node_secret = payload["node_secret"]
        .as_str()
        .ok_or("Missing node_secret in auth payload")?;

    // Check lockout BEFORE doing any crypto
    if state.node_hub.auth_limiter.is_locked_out(node_id) {
        tracing::warn!(
            "Node {node_id} is locked out due to repeated auth failures"
        );
        return Err("Too many failed authentication attempts. Locked out for 60 seconds.".into());
    }

    let stored_enc = manager_core::db::nodes::get_node_secret_enc(&state.db, node_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            state.node_hub.auth_limiter.record_failure(node_id);
            "Node not registered".to_string()
        })?;

    let stored_secret = manager_core::crypto::decrypt(&stored_enc, &state.master_key)
        .map_err(|e| e.to_string())?;

    if stored_secret != node_secret {
        let locked = state.node_hub.auth_limiter.record_failure(node_id);
        if locked {
            tracing::warn!(
                "Node {node_id} locked out after repeated failed auth attempts"
            );
            return Err(
                "Invalid node secret. Too many failures — locked out for 60 seconds.".into(),
            );
        }
        return Err("Invalid node secret".into());
    }

    // Check if node has expired
    if let Ok(Some(node)) = manager_core::db::nodes::get_node_by_node_id(&state.db, node_id).await {
        if node.is_expired() {
            return Err("Node has expired".into());
        }
    }

    // Success — clear any prior failures
    state.node_hub.auth_limiter.clear(node_id);

    Ok(AuthResult {
        node_id: node_id.to_string(),
        new_credentials: None,
        protocol_version,
    })
}

// ───────────────────────────────────────────────────────
// Message handling
// ───────────────────────────────────────────────────────

/// Maximum WebSocket message size from a node (5 MB).
const MAX_NODE_MESSAGE_SIZE: usize = 5 * 1024 * 1024;
/// Maximum event message field length.
const MAX_EVENT_MESSAGE_LEN: usize = 10_000;
/// Maximum event category field length.
const MAX_EVENT_CATEGORY_LEN: usize = 256;
/// Maximum software version string length.
const MAX_VERSION_LEN: usize = 256;

async fn handle_node_message(state: &AppState, node_id: &str, text: &str) {
    // Reject oversized messages
    if text.len() > MAX_NODE_MESSAGE_SIZE {
        tracing::warn!(
            "Dropping oversized message from node {node_id}: {} bytes (max {})",
            text.len(),
            MAX_NODE_MESSAGE_SIZE
        );
        return;
    }

    let envelope: WsEnvelope = match serde_json::from_str(text) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Invalid message from node {node_id}: {e}");
            return;
        }
    };

    match envelope.msg_type.as_str() {
        "stats" => {
            if let Some(mut conn) = state.node_hub.connections.get_mut(node_id) {
                conn.cached_stats = Some(envelope.payload.clone());
            }
            state.node_hub.broadcast_to_browsers(Some(&state.driver_registry));
        }
        "health" => {
            let version = envelope.payload["version"]
                .as_str()
                .map(|v| &v[..v.len().min(MAX_VERSION_LEN)])
                .map(String::from);
            if let Some(mut conn) = state.node_hub.connections.get_mut(node_id) {
                conn.cached_health = Some(envelope.payload.clone());
                if let Some(ref v) = version {
                    conn.software_version = Some(v.clone());
                }
            }
            let _ = manager_core::db::nodes::update_node_health(
                &state.db,
                node_id,
                &envelope.payload,
                version.as_deref(),
            )
            .await;
            state.node_hub.broadcast_to_browsers(Some(&state.driver_registry));
        }
        "event" => {
            let severity = envelope.payload["severity"]
                .as_str()
                .and_then(EventSeverity::from_str)
                .unwrap_or(EventSeverity::Info);
            let category = envelope.payload["category"]
                .as_str()
                .unwrap_or("unknown");
            let category = &category[..category.len().min(MAX_EVENT_CATEGORY_LEN)];
            let message = envelope.payload["message"]
                .as_str()
                .unwrap_or("Unknown event");
            let message = &message[..message.len().min(MAX_EVENT_MESSAGE_LEN)];
            let flow_id = envelope.payload["flow_id"].as_str();
            let details = envelope.payload.get("details");

            let _ = manager_core::db::events::insert_event(
                &state.db,
                node_id,
                severity,
                category,
                message,
                details,
                flow_id,
            )
            .await;
        }
        "config_response" => {
            if let Some(mut conn) = state.node_hub.connections.get_mut(node_id) {
                conn.cached_config = Some(envelope.payload);
            }
        }
        "command_ack" => {
            let command_id = envelope.payload["command_id"].as_str().unwrap_or("");
            let success = envelope.payload["success"].as_bool().unwrap_or(false);
            let error = envelope.payload["error"].as_str().map(String::from);

            tracing::info!(
                "Command ack from {node_id}: id={command_id} success={success} error={error:?}"
            );

            // Clear cached config so the next request fetches fresh data
            if let Some(mut conn) = state.node_hub.connections.get_mut(node_id) {
                conn.cached_config = None;
            }

            // Resolve the pending command if someone is waiting
            if !command_id.is_empty()
                && let Some((_, tx)) = state.node_hub.pending_commands.remove(command_id)
            {
                let data = envelope.payload.get("data").cloned();
                let ack = CommandAckPayload {
                    command_id: command_id.to_string(),
                    success,
                    error,
                    data,
                };
                let _ = tx.send(ack);
            }
        }
        "pong" => {}
        other => {
            tracing::debug!("Unknown message type '{other}' from node {node_id}");
        }
    }
}

/// Push all pending/active tunnels for a node that just (re)connected.
/// This ensures tunnels created while the node was offline get configured.
async fn push_pending_tunnels_to_node(state: &AppState, node_id: &str) {
    let tunnels = match manager_core::db::tunnels::list_tunnels_for_node_full(&state.db, node_id).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Failed to load tunnels for reconnected node {node_id}: {e}");
            return;
        }
    };

    let active_tunnels: Vec<_> = tunnels.into_iter()
        .filter(|t| matches!(t.status, manager_core::models::tunnel::TunnelStatus::Pending | manager_core::models::tunnel::TunnelStatus::Active))
        .collect();

    if active_tunnels.is_empty() {
        return;
    }

    tracing::info!("Pushing {} tunnel(s) to reconnected node {node_id}", active_tunnels.len());

    for tunnel in &active_tunnels {
        match crate::api::tunnels::push_tunnel_to_node(state, tunnel, node_id).await {
            Ok(()) => {
                tracing::info!("Pushed tunnel '{}' to node {node_id}", tunnel.id);
                // Mark active only when all participants are connected
                let ingress_ok = state.node_hub.connections.contains_key(&tunnel.ingress_node_id);
                let egress_ok = state.node_hub.connections.contains_key(&tunnel.egress_node_id);
                let relay_ok = tunnel.relay_node_id.as_ref()
                    .map(|r| state.node_hub.connections.contains_key(r))
                    .unwrap_or(true); // no relay needed for direct tunnels
                if ingress_ok && egress_ok && relay_ok {
                    let _ = manager_core::db::tunnels::update_tunnel_status(
                        &state.db, &tunnel.id, "active"
                    ).await;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to push tunnel '{}' to node {node_id}: {e}", tunnel.id);
            }
        }
    }
}
