// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Envelope for all WebSocket messages between manager and device nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEnvelope {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub timestamp: DateTime<Utc>,
    pub payload: serde_json::Value,
}

// ── Node → Manager messages ──

/// Initial registration message from a new device node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterPayload {
    pub token: String,
    pub software_version: Option<String>,
    pub hostname: Option<String>,
}

/// Authentication message for reconnecting nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatePayload {
    pub node_id: String,
    pub node_secret: String,
    pub software_version: Option<String>,
}

/// Stats snapshot from a device node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsPayload {
    pub flows: Vec<super::FlowStats>,
    pub uptime_secs: u64,
    pub active_flows: u32,
    pub total_flows: u32,
}

/// Health check from a device node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthPayload {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub active_flows: u32,
    pub total_flows: u32,
}

/// An event/alarm from a device node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPayload {
    pub severity: String,
    pub category: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub flow_id: Option<String>,
}

/// Response to a config request from manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponsePayload {
    pub config: serde_json::Value,
}

/// Acknowledgement of a command from manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandAckPayload {
    pub command_id: String,
    pub success: bool,
    pub error: Option<String>,
}

// ── Manager → Node messages ──

/// Registration acknowledgement with assigned credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterAckPayload {
    pub node_id: String,
    pub node_secret: String,
}

/// Command to execute on a device node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPayload {
    pub command_id: String,
    pub action: CommandAction,
}

/// Actions that the manager can send to device nodes (currently edge-specific, will be generalized via drivers).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CommandAction {
    #[serde(rename = "get_config")]
    GetConfig,
    #[serde(rename = "update_config")]
    UpdateConfig { config: serde_json::Value },
    #[serde(rename = "create_flow")]
    CreateFlow { flow: serde_json::Value },
    #[serde(rename = "update_flow")]
    UpdateFlow { flow_id: String, flow: serde_json::Value },
    #[serde(rename = "delete_flow")]
    DeleteFlow { flow_id: String },
    #[serde(rename = "start_flow")]
    StartFlow { flow_id: String },
    #[serde(rename = "stop_flow")]
    StopFlow { flow_id: String },
    #[serde(rename = "restart_flow")]
    RestartFlow { flow_id: String },
    #[serde(rename = "add_output")]
    AddOutput { flow_id: String, output: serde_json::Value },
    #[serde(rename = "remove_output")]
    RemoveOutput { flow_id: String, output_id: String },
}

// ── Browser-facing messages ──

/// Aggregated dashboard update pushed to browser WebSocket clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardUpdate {
    pub nodes: Vec<NodeDashboardEntry>,
    pub total_nodes: u32,
    pub online_nodes: u32,
    pub active_alarms: u32,
}

/// Per-node entry in the dashboard update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDashboardEntry {
    pub node_id: String,
    pub name: String,
    pub status: String,
    pub software_version: Option<String>,
    pub uptime_secs: u64,
    pub flows: Vec<super::FlowStats>,
    pub active_flows: u32,
    pub total_flows: u32,
    pub total_bitrate_bps: u64,
}

impl WsEnvelope {
    pub fn new(msg_type: &str, payload: serde_json::Value) -> Self {
        Self {
            msg_type: msg_type.to_string(),
            timestamp: Utc::now(),
            payload,
        }
    }

    pub fn ping() -> Self {
        Self::new("ping", serde_json::Value::Null)
    }

    pub fn pong() -> Self {
        Self::new("pong", serde_json::Value::Null)
    }
}
