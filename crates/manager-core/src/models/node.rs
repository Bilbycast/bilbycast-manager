// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of a managed device node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    /// Registered but never connected.
    Pending,
    /// Currently connected via WebSocket.
    Online,
    /// Was connected but WebSocket dropped.
    Offline,
    /// Connected but reporting issues (e.g., flow errors).
    Degraded,
    /// Connected but in critical error state.
    Error,
}

impl NodeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Online => "online",
            Self::Offline => "offline",
            Self::Degraded => "degraded",
            Self::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "online" => Some(Self::Online),
            "offline" => Some(Self::Offline),
            "degraded" => Some(Self::Degraded),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A managed device node registered with the manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Device type identifier (e.g., "edge", "relay"). Maps to a registered DeviceDriver.
    #[serde(default = "default_device_type")]
    pub device_type: String,
    #[serde(skip_serializing)]
    pub registration_token: Option<String>,
    pub status: NodeStatus,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub last_health: Option<serde_json::Value>,
    pub software_version: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Node {
    /// Returns true if the node has an expiry time that has passed.
    pub fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |exp| Utc::now() > exp)
    }
}

fn default_device_type() -> String {
    "edge".to_string()
}

/// Request to create a new node.
#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    pub name: String,
    pub description: Option<String>,
    /// Device type identifier. Defaults to "edge" if not provided.
    pub device_type: Option<String>,
    /// Optional expiry time (ISO 8601). Node registration is rejected after this time.
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request to update a node.
#[derive(Debug, Deserialize)]
pub struct UpdateNodeRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    /// Set or clear expiry time. Use `null` to remove expiry.
    pub expires_at: Option<Option<DateTime<Utc>>>,
}

/// Node info with connection status for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: NodeStatus,
    pub software_version: Option<String>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub active_flows: u32,
    pub total_flows: u32,
    pub total_bitrate_bps: u64,
}

/// Tracks an active WebSocket connection from a node.
#[derive(Debug, Clone, Serialize)]
pub struct NodeConnection {
    pub node_id: String,
    pub connected_at: DateTime<Utc>,
    pub remote_addr: Option<String>,
    pub ws_session_id: String,
}
