// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::tunnel::*;

pub async fn create_tunnel(pool: &SqlitePool, req: &CreateTunnelRequest) -> Result<Tunnel> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let protocol = req.protocol.as_str();
    let mode = req.mode.as_str();
    let port = req.ingress_listen_port as i64;
    let flow_ids_json = req.associated_flow_ids.as_ref().map(|ids| serde_json::to_string(ids).unwrap_or_default());

    sqlx::query(
        "INSERT INTO tunnels (id, name, protocol, mode, ingress_node_id, ingress_listen_port, egress_node_id, egress_forward_addr, relay_addr, associated_flow_ids, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?)"
    )
    .bind(&id)
    .bind(&req.name)
    .bind(protocol)
    .bind(mode)
    .bind(&req.ingress_node_id)
    .bind(port)
    .bind(&req.egress_node_id)
    .bind(&req.egress_forward_addr)
    .bind(&req.relay_addr)
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
        "SELECT t.*, n1.name as ingress_node_name, n2.name as egress_node_name FROM tunnels t LEFT JOIN nodes n1 ON t.ingress_node_id = n1.id LEFT JOIN nodes n2 ON t.egress_node_id = n2.id WHERE t.ingress_node_id = ? OR t.egress_node_id = ? ORDER BY t.created_at DESC"
    )
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
    let flow_ids_json = req.associated_flow_ids.as_ref()
        .map(|ids| serde_json::to_string(ids).unwrap_or_default())
        .or_else(|| existing.associated_flow_ids.as_ref().map(|ids| serde_json::to_string(ids).unwrap_or_default()));

    sqlx::query(
        "UPDATE tunnels SET name = ?, status = ?, relay_addr = ?, ingress_listen_port = ?, egress_forward_addr = ?, associated_flow_ids = ?, updated_at = ? WHERE id = ?"
    )
    .bind(name)
    .bind(&status)
    .bind(relay_addr)
    .bind(port)
    .bind(forward_addr)
    .bind(&flow_ids_json)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get_tunnel(pool, id).await
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
    relay_addr: Option<String>,
    #[allow(dead_code)]
    tunnel_psk_enc: Option<String>,
    status: String,
    associated_flow_ids: Option<String>,
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
            relay_addr: self.relay_addr,
            status: TunnelStatus::from_str(&self.status).unwrap_or(TunnelStatus::Pending),
            associated_flow_ids: flow_ids,
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
    relay_addr: Option<String>,
    #[allow(dead_code)]
    tunnel_psk_enc: Option<String>,
    status: String,
    associated_flow_ids: Option<String>,
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
            relay_addr: self.relay_addr,
            status: self.status,
            associated_flow_ids: flow_ids,
        }
    }
}
