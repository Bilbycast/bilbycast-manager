// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of a tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TunnelStatus {
    Pending,
    Active,
    Error,
    Disabled,
}

impl TunnelStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Error => "error",
            Self::Disabled => "disabled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "active" => Some(Self::Active),
            "error" => Some(Self::Error),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }
}

impl std::fmt::Display for TunnelStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Connection mode for a tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TunnelMode {
    /// Traffic goes through a bilbycast-relay server (for NAT-to-NAT).
    Relay,
    /// Direct edge-to-edge connection (one side has a public IP/open port).
    Direct,
}

impl TunnelMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Relay => "relay",
            Self::Direct => "direct",
        }
    }
}

impl std::fmt::Display for TunnelMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Transport protocol tunneled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TunnelProtocol {
    Tcp,
    Udp,
}

impl TunnelProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }
}

impl std::fmt::Display for TunnelProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An IP tunnel between two edge nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tunnel {
    pub id: String,
    pub name: String,
    pub protocol: TunnelProtocol,
    pub mode: TunnelMode,
    pub ingress_node_id: String,
    pub ingress_listen_port: u16,
    pub egress_node_id: String,
    pub egress_forward_addr: String,
    pub relay_addr: Option<String>,
    /// Node ID of the relay server (for relay mode tunnels).
    pub relay_node_id: Option<String>,
    /// Encrypted tunnel encryption key (ChaCha20-Poly1305), stored encrypted at rest.
    pub tunnel_key_enc: Option<String>,
    pub status: TunnelStatus,
    pub associated_flow_ids: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a tunnel.
#[derive(Debug, Deserialize)]
pub struct CreateTunnelRequest {
    pub name: String,
    pub protocol: TunnelProtocol,
    pub mode: TunnelMode,
    pub ingress_node_id: String,
    pub ingress_listen_port: u16,
    pub egress_node_id: String,
    pub egress_forward_addr: String,
    pub relay_addr: Option<String>,
    /// Node ID of the relay server (required for relay mode).
    /// The manager will automatically authorize both edges on this relay.
    pub relay_node_id: Option<String>,
    pub associated_flow_ids: Option<Vec<String>>,
}

/// Request to update a tunnel.
#[derive(Debug, Deserialize)]
pub struct UpdateTunnelRequest {
    pub name: Option<String>,
    pub status: Option<TunnelStatus>,
    pub relay_addr: Option<String>,
    pub ingress_listen_port: Option<u16>,
    pub egress_forward_addr: Option<String>,
    pub associated_flow_ids: Option<Vec<String>>,
}

/// Tunnel summary for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelSummary {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub mode: String,
    pub ingress_node_id: String,
    pub ingress_node_name: Option<String>,
    pub ingress_listen_port: u16,
    pub egress_node_id: String,
    pub egress_node_name: Option<String>,
    pub egress_forward_addr: String,
    pub relay_addr: Option<String>,
    pub relay_node_id: Option<String>,
    pub status: String,
    pub associated_flow_ids: Option<Vec<String>>,
}
