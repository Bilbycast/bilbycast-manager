// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::tunnel::*;

/// Encrypted secrets to persist alongside the tunnel record.
pub struct TunnelSecrets<'a> {
    pub tunnel_key_enc: Option<&'a str>,
    pub tunnel_bind_secret_enc: Option<&'a str>,
    pub tunnel_psk_enc: Option<&'a str>,
}

pub async fn create_tunnel(pool: &SqlitePool, req: &CreateTunnelRequest, secrets: &TunnelSecrets<'_>) -> Result<Tunnel> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let protocol = req.protocol.as_str();
    let mode = req.mode.as_str();
    let port = req.ingress_listen_port as i64;
    let flow_ids_json = req.associated_flow_ids.as_ref().map(|ids| serde_json::to_string(ids).unwrap_or_default());

    sqlx::query(
        "INSERT INTO tunnels (id, name, protocol, mode, ingress_node_id, ingress_listen_port, egress_node_id, egress_forward_addr, egress_peer_addr, relay_addr, relay_node_id, tunnel_key_enc, tunnel_psk_enc, tunnel_bind_secret_enc, associated_flow_ids, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?)"
    )
    .bind(&id)
    .bind(&req.name)
    .bind(protocol)
    .bind(mode)
    .bind(&req.ingress_node_id)
    .bind(port)
    .bind(&req.egress_node_id)
    .bind(&req.egress_forward_addr)
    .bind(&req.egress_peer_addr)
    .bind(&req.relay_addr)
    .bind(&req.relay_node_id)
    .bind(secrets.tunnel_key_enc)
    .bind(secrets.tunnel_psk_enc)
    .bind(secrets.tunnel_bind_secret_enc)
    .bind(&flow_ids_json)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    get_tunnel(pool, &id).await?.ok_or_else(|| anyhow::anyhow!("tunnel not found after insert"))
}

pub async fn get_tunnel(pool: &SqlitePool, id: &str) -> Result<Option<Tunnel>> {
    let row = sqlx::query_as::<_, TunnelRow>("SELECT * FROM tunnels WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.into_tunnel()))
}

pub async fn list_tunnels(pool: &SqlitePool) -> Result<Vec<TunnelSummary>> {
    let rows = sqlx::query_as::<_, TunnelWithNodesRow>(
        "SELECT t.*, n1.name as ingress_node_name, n2.name as egress_node_name FROM tunnels t LEFT JOIN nodes n1 ON t.ingress_node_id = n1.id LEFT JOIN nodes n2 ON t.egress_node_id = n2.id ORDER BY t.created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.into_summary()).collect())
}

pub async fn list_tunnels_for_node(pool: &SqlitePool, node_id: &str) -> Result<Vec<TunnelSummary>> {
    let rows = sqlx::query_as::<_, TunnelWithNodesRow>(
        "SELECT t.*, n1.name as ingress_node_name, n2.name as egress_node_name FROM tunnels t LEFT JOIN nodes n1 ON t.ingress_node_id = n1.id LEFT JOIN nodes n2 ON t.egress_node_id = n2.id WHERE t.ingress_node_id = ? OR t.egress_node_id = ? OR t.relay_node_id = ? ORDER BY t.created_at DESC"
    )
    .bind(node_id)
    .bind(node_id)
    .bind(node_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.into_summary()).collect())
}

pub async fn update_tunnel(pool: &SqlitePool, id: &str, req: &UpdateTunnelRequest) -> Result<Option<Tunnel>> {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let existing = get_tunnel(pool, id).await?;
    let Some(existing) = existing else { return Ok(None); };

    let name = req.name.as_deref().unwrap_or(&existing.name);
    let status = req.status.map(|s| s.as_str().to_string()).unwrap_or_else(|| existing.status.as_str().to_string());
    let relay_addr = req.relay_addr.as_deref().or(existing.relay_addr.as_deref());
    let port = req.ingress_listen_port.unwrap_or(existing.ingress_listen_port) as i64;
    let forward_addr = req.egress_forward_addr.as_deref().unwrap_or(&existing.egress_forward_addr);
    let egress_peer_addr = req.egress_peer_addr.as_deref().or(existing.egress_peer_addr.as_deref());
    let flow_ids_json = req.associated_flow_ids.as_ref()
        .map(|ids| serde_json::to_string(ids).unwrap_or_default())
        .or_else(|| existing.associated_flow_ids.as_ref().map(|ids| serde_json::to_string(ids).unwrap_or_default()));

    sqlx::query(
        "UPDATE tunnels SET name = ?, status = ?, relay_addr = ?, ingress_listen_port = ?, egress_forward_addr = ?, egress_peer_addr = ?, associated_flow_ids = ?, updated_at = ? WHERE id = ?"
    )
    .bind(name)
    .bind(&status)
    .bind(relay_addr)
    .bind(port)
    .bind(forward_addr)
    .bind(egress_peer_addr)
    .bind(&flow_ids_json)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get_tunnel(pool, id).await
}

/// List full tunnel records for a node (used for reconnection push).
/// Includes tunnels where the node is ingress, egress, or relay.
pub async fn list_tunnels_for_node_full(pool: &SqlitePool, node_id: &str) -> Result<Vec<Tunnel>> {
    let rows = sqlx::query_as::<_, TunnelRow>(
        "SELECT * FROM tunnels WHERE ingress_node_id = ? OR egress_node_id = ? OR relay_node_id = ? ORDER BY created_at DESC"
    )
    .bind(node_id)
    .bind(node_id)
    .bind(node_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.into_tunnel()).collect())
}

pub async fn update_tunnel_status(pool: &SqlitePool, id: &str, status: &str) -> Result<()> {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    sqlx::query("UPDATE tunnels SET status = ?, updated_at = ? WHERE id = ?")
        .bind(status)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update the push status for a specific leg of a tunnel.
/// `leg` must be one of "ingress", "egress", or "relay".
pub async fn update_tunnel_push_status(
    pool: &SqlitePool,
    id: &str,
    leg: &str,
    status: &str,
    error: Option<&str>,
) -> Result<()> {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    match leg {
        "ingress" => {
            sqlx::query("UPDATE tunnels SET ingress_push_status = ?, ingress_push_error = ?, updated_at = ? WHERE id = ?")
                .bind(status).bind(error).bind(&now).bind(id)
                .execute(pool).await?;
        }
        "egress" => {
            sqlx::query("UPDATE tunnels SET egress_push_status = ?, egress_push_error = ?, updated_at = ? WHERE id = ?")
                .bind(status).bind(error).bind(&now).bind(id)
                .execute(pool).await?;
        }
        "relay" => {
            sqlx::query("UPDATE tunnels SET relay_push_status = ?, relay_push_error = ?, updated_at = ? WHERE id = ?")
                .bind(status).bind(error).bind(&now).bind(id)
                .execute(pool).await?;
        }
        _ => return Err(anyhow::anyhow!("Invalid leg: {leg}")),
    }
    Ok(())
}

/// List tunnels where any leg has pending or failed push status.
pub async fn list_pending_tunnels(pool: &SqlitePool) -> Result<Vec<Tunnel>> {
    let rows = sqlx::query_as::<_, TunnelRow>(
        "SELECT * FROM tunnels WHERE (status = 'pending' OR status = 'active') AND (ingress_push_status IN ('pending', 'failed') OR egress_push_status IN ('pending', 'failed') OR relay_push_status IN ('pending', 'failed')) ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.into_tunnel()).collect())
}

/// Reset push statuses to "pending" for all active/pending tunnels involving a node.
/// Called when a node disconnects so the retry task re-pushes configs on reconnection.
pub async fn reset_push_status_for_node(pool: &SqlitePool, node_id: &str) -> Result<u64> {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let mut total = 0u64;

    let r = sqlx::query(
        "UPDATE tunnels SET ingress_push_status = 'pending', ingress_push_error = NULL, updated_at = ? \
         WHERE ingress_node_id = ? AND status IN ('pending', 'active') AND ingress_push_status != 'pending'"
    )
    .bind(&now).bind(node_id)
    .execute(pool).await?;
    total += r.rows_affected();

    let r = sqlx::query(
        "UPDATE tunnels SET egress_push_status = 'pending', egress_push_error = NULL, updated_at = ? \
         WHERE egress_node_id = ? AND status IN ('pending', 'active') AND egress_push_status != 'pending'"
    )
    .bind(&now).bind(node_id)
    .execute(pool).await?;
    total += r.rows_affected();

    let r = sqlx::query(
        "UPDATE tunnels SET relay_push_status = 'pending', relay_push_error = NULL, updated_at = ? \
         WHERE relay_node_id = ? AND status IN ('pending', 'active') AND relay_push_status != 'pending'"
    )
    .bind(&now).bind(node_id)
    .execute(pool).await?;
    total += r.rows_affected();

    Ok(total)
}

pub async fn delete_tunnel(pool: &SqlitePool, id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM tunnels WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ── Internal row types for sqlx ──

#[derive(sqlx::FromRow)]
struct TunnelRow {
    id: String,
    name: String,
    protocol: String,
    mode: String,
    ingress_node_id: String,
    ingress_listen_port: i64,
    egress_node_id: String,
    egress_forward_addr: String,
    egress_peer_addr: Option<String>,
    relay_addr: Option<String>,
    relay_node_id: Option<String>,
    tunnel_psk_enc: Option<String>,
    tunnel_key_enc: Option<String>,
    tunnel_bind_secret_enc: Option<String>,
    status: String,
    associated_flow_ids: Option<String>,
    ingress_push_status: String,
    egress_push_status: String,
    relay_push_status: Option<String>,
    ingress_push_error: Option<String>,
    egress_push_error: Option<String>,
    relay_push_error: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TunnelRow {
    fn into_tunnel(self) -> Tunnel {
        let flow_ids: Option<Vec<String>> = self.associated_flow_ids
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());
        Tunnel {
            id: self.id,
            name: self.name,
            protocol: match self.protocol.as_str() {
                "tcp" => TunnelProtocol::Tcp,
                _ => TunnelProtocol::Udp,
            },
            mode: match self.mode.as_str() {
                "direct" => TunnelMode::Direct,
                _ => TunnelMode::Relay,
            },
            ingress_node_id: self.ingress_node_id,
            ingress_listen_port: self.ingress_listen_port as u16,
            egress_node_id: self.egress_node_id,
            egress_forward_addr: self.egress_forward_addr,
            egress_peer_addr: self.egress_peer_addr,
            relay_addr: self.relay_addr,
            relay_node_id: self.relay_node_id,
            tunnel_key_enc: self.tunnel_key_enc,
            tunnel_psk_enc: self.tunnel_psk_enc,
            tunnel_bind_secret_enc: self.tunnel_bind_secret_enc,
            status: TunnelStatus::from_str(&self.status).unwrap_or(TunnelStatus::Pending),
            associated_flow_ids: flow_ids,
            ingress_push_status: self.ingress_push_status,
            egress_push_status: self.egress_push_status,
            relay_push_status: self.relay_push_status,
            ingress_push_error: self.ingress_push_error,
            egress_push_error: self.egress_push_error,
            relay_push_error: self.relay_push_error,
            created_at: chrono::DateTime::parse_from_rfc3339(&format!("{}+00:00", self.created_at.trim_end_matches('Z')))
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&format!("{}+00:00", self.updated_at.trim_end_matches('Z')))
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}

#[derive(sqlx::FromRow)]
struct TunnelWithNodesRow {
    id: String,
    name: String,
    protocol: String,
    mode: String,
    ingress_node_id: String,
    ingress_listen_port: i64,
    egress_node_id: String,
    egress_forward_addr: String,
    egress_peer_addr: Option<String>,
    relay_addr: Option<String>,
    relay_node_id: Option<String>,
    #[allow(dead_code)]
    tunnel_psk_enc: Option<String>,
    #[allow(dead_code)]
    tunnel_key_enc: Option<String>,
    #[allow(dead_code)]
    tunnel_bind_secret_enc: Option<String>,
    status: String,
    associated_flow_ids: Option<String>,
    ingress_push_status: String,
    egress_push_status: String,
    relay_push_status: Option<String>,
    ingress_push_error: Option<String>,
    egress_push_error: Option<String>,
    relay_push_error: Option<String>,
    #[allow(dead_code)]
    created_at: String,
    #[allow(dead_code)]
    updated_at: String,
    ingress_node_name: Option<String>,
    egress_node_name: Option<String>,
}

impl TunnelWithNodesRow {
    fn into_summary(self) -> TunnelSummary {
        let flow_ids: Option<Vec<String>> = self.associated_flow_ids
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());
        TunnelSummary {
            id: self.id,
            name: self.name,
            protocol: self.protocol,
            mode: self.mode,
            ingress_node_id: self.ingress_node_id,
            ingress_node_name: self.ingress_node_name,
            ingress_listen_port: self.ingress_listen_port as u16,
            egress_node_id: self.egress_node_id,
            egress_node_name: self.egress_node_name,
            egress_forward_addr: self.egress_forward_addr,
            egress_peer_addr: self.egress_peer_addr,
            relay_addr: self.relay_addr,
            relay_node_id: self.relay_node_id,
            status: self.status,
            associated_flow_ids: flow_ids,
            ingress_push_status: self.ingress_push_status,
            egress_push_status: self.egress_push_status,
            relay_push_status: self.relay_push_status,
            ingress_push_error: self.ingress_push_error,
            egress_push_error: self.egress_push_error,
            relay_push_error: self.relay_push_error,
        }
    }
}
