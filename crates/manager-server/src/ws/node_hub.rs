use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use chrono::Utc;
use dashmap::DashMap;
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::app_state::AppState;
use manager_core::models::{EventSeverity, NodeStatus, WsEnvelope};

/// Query parameters for WebSocket connection.
#[derive(Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
    pub node_id: Option<String>,
    pub node_secret: Option<String>,
}

/// State for a connected edge node.
struct ConnectedNode {
    node_id: String,
    node_name: String,
    command_tx: mpsc::Sender<String>,
    cached_config: Option<serde_json::Value>,
    cached_stats: Option<serde_json::Value>,
    cached_health: Option<serde_json::Value>,
    software_version: Option<String>,
    connected_at: chrono::DateTime<chrono::Utc>,
}

/// Hub managing all edge node WebSocket connections.
pub struct NodeHub {
    connections: DashMap<String, ConnectedNode>,
    browser_tx: broadcast::Sender<String>,
    db: SqlitePool,
}

impl NodeHub {
    pub fn new(db: SqlitePool, browser_tx: broadcast::Sender<String>) -> Self {
        Self {
            connections: DashMap::new(),
            browser_tx,
            db,
        }
    }

    /// Send a command to a connected node. Returns the command ack or error.
    pub async fn send_command(
        &self,
        node_id: &str,
        action: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
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

        Ok(serde_json::json!({
            "command_id": command_id,
            "sent": true
        }))
    }

    /// Get cached config for a node.
    pub async fn get_cached_config(&self, node_id: &str) -> Option<serde_json::Value> {
        self.connections
            .get(node_id)
            .and_then(|c| c.cached_config.clone())
    }

    /// Disconnect a node (e.g., when deleting it).
    pub async fn disconnect_node(&self, node_id: &str) {
        if let Some((_, conn)) = self.connections.remove(node_id) {
            drop(conn.command_tx);
        }
    }

    /// Get the number of connected nodes.
    pub fn connected_count(&self) -> usize {
        self.connections.len()
    }

    /// Broadcast aggregated stats to browser clients.
    fn broadcast_to_browsers(&self) {
        let mut nodes = Vec::new();
        for entry in self.connections.iter() {
            let node = entry.value();
            let uptime_secs = (Utc::now() - node.connected_at).num_seconds().max(0) as u64;
            nodes.push(serde_json::json!({
                "node_id": node.node_id,
                "name": node.node_name,
                "status": "online",
                "stats": node.cached_stats,
                "health": node.cached_health,
                "software_version": node.software_version,
                "uptime_secs": uptime_secs,
            }));
        }

        let update = serde_json::json!({
            "type": "dashboard_update",
            "nodes": nodes,
            "online_count": self.connections.len(),
        });

        let _ = self.browser_tx.send(update.to_string());
    }
}

/// WebSocket handler for edge node connections.
pub async fn node_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_node_connection(socket, state, query))
}

async fn handle_node_connection(mut socket: WebSocket, state: AppState, query: WsQuery) {
    // Step 1: Authenticate the node
    let auth_result = match authenticate_node(&state, &query).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Node authentication failed: {e}");
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({"type": "error", "message": e}).to_string().into(),
                ))
                .await;
            return;
        }
    };

    let node_id = auth_result.node_id;

    // Send register_ack with credentials if this was a first-time registration
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
    }

    tracing::info!("Edge node {node_id} connected");

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

    // Get node name
    let node_name = manager_core::db::nodes::get_node_by_id(&state.db, &node_id)
        .await
        .map(|n| n.name)
        .unwrap_or_else(|_| node_id.clone());

    // Create command channel
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<String>(64);

    // Register connection
    state.node_hub.connections.insert(
        node_id.clone(),
        ConnectedNode {
            node_id: node_id.clone(),
            node_name,
            command_tx: cmd_tx,
            cached_config: None,
            cached_stats: None,
            cached_health: None,
            software_version: None,
            connected_at: Utc::now(),
        },
    );

    // Main message loop
    loop {
        tokio::select! {
            // Receive from edge node
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
            // Send commands to edge node
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
    tracing::info!("Edge node {node_id} disconnected");
    state.node_hub.connections.remove(&node_id);

    let _ =
        manager_core::db::nodes::update_node_status(&state.db, &node_id, NodeStatus::Offline)
            .await;

    let _ = manager_core::db::events::insert_event(
        &state.db,
        &node_id,
        EventSeverity::Warning,
        "connection",
        "Node disconnected from manager",
        None,
        None,
    )
    .await;
}

/// Authentication result: node_id and optionally new credentials for first-time registration.
struct AuthResult {
    node_id: String,
    /// Set only on first-time registration (token-based), so we can send register_ack.
    new_credentials: Option<(String, String)>, // (node_id, node_secret)
}

async fn authenticate_node(state: &AppState, query: &WsQuery) -> Result<AuthResult, String> {
    // Registration with token
    if let Some(ref token) = query.token {
        let node = manager_core::db::nodes::get_node_by_token(&state.db, token)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Invalid registration token")?;

        // Generate node secret
        let node_secret = Uuid::new_v4().to_string();
        let encrypted =
            manager_core::crypto::encrypt(&node_secret, &state.master_key).map_err(|e| e.to_string())?;

        manager_core::db::nodes::complete_registration(&state.db, &node.id, &encrypted)
            .await
            .map_err(|e| e.to_string())?;

        return Ok(AuthResult {
            node_id: node.id.clone(),
            new_credentials: Some((node.id, node_secret)),
        });
    }

    // Reconnection with node_id + node_secret
    if let (Some(node_id), Some(node_secret)) = (&query.node_id, &query.node_secret) {
        let stored_enc = manager_core::db::nodes::get_node_secret_enc(&state.db, node_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Node not registered")?;

        let stored_secret =
            manager_core::crypto::decrypt(&stored_enc, &state.master_key).map_err(|e| e.to_string())?;

        if stored_secret != *node_secret {
            return Err("Invalid node secret".into());
        }

        return Ok(AuthResult {
            node_id: node_id.clone(),
            new_credentials: None,
        });
    }

    Err("Missing authentication parameters (token or node_id+node_secret)".into())
}

async fn handle_node_message(state: &AppState, node_id: &str, text: &str) {
    let envelope: WsEnvelope = match serde_json::from_str(text) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Invalid message from node {node_id}: {e}");
            return;
        }
    };

    match envelope.msg_type.as_str() {
        "stats" => {
            // Cache the latest stats
            if let Some(mut conn) = state.node_hub.connections.get_mut(node_id) {
                conn.cached_stats = Some(envelope.payload.clone());
            }
            // Broadcast to browsers
            state.node_hub.broadcast_to_browsers();
        }
        "health" => {
            let version = envelope.payload["version"].as_str().map(String::from);
            // Cache health on connection for browser broadcasts
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
            // Also broadcast after health update
            state.node_hub.broadcast_to_browsers();
        }
        "event" => {
            let severity = envelope.payload["severity"]
                .as_str()
                .and_then(EventSeverity::from_str)
                .unwrap_or(EventSeverity::Info);
            let category = envelope.payload["category"]
                .as_str()
                .unwrap_or("unknown");
            let message = envelope.payload["message"]
                .as_str()
                .unwrap_or("Unknown event");
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
            tracing::info!(
                "Command ack from {node_id}: {}",
                envelope.payload
            );
        }
        "pong" => {}
        other => {
            tracing::debug!("Unknown message type '{other}' from node {node_id}");
        }
    }
}
