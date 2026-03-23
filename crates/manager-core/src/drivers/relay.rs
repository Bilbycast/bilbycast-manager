// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

//! Device driver for bilbycast-relay servers.

use super::{AiDeviceContext, CommandDescriptor, DeviceDriver, DeviceMetricsSummary};

/// Driver for bilbycast-relay QUIC tunnel servers.
pub struct RelayDriver;

impl RelayDriver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RelayDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// Known relay command types: (name, description, minimum_role).
const RELAY_COMMANDS: &[(&str, &str, &str)] = &[
    (
        "get_config",
        "Request the relay's current configuration",
        "operator",
    ),
    (
        "disconnect_edge",
        "Force-disconnect a specific edge node from the relay",
        "admin",
    ),
    (
        "close_tunnel",
        "Force-close a specific tunnel (unbinds both sides)",
        "admin",
    ),
    (
        "list_tunnels",
        "List all active and pending tunnels",
        "operator",
    ),
    (
        "list_edges",
        "List all connected edge nodes",
        "operator",
    ),
];

impl DeviceDriver for RelayDriver {
    fn device_type(&self) -> &str {
        "relay"
    }

    fn display_name(&self) -> &str {
        "Relay Server"
    }

    fn extract_metrics(&self, stats: &serde_json::Value) -> DeviceMetricsSummary {
        let tunnels = stats.get("tunnels").and_then(|t| t.as_array());

        let active_tunnels = stats
            .get("active_tunnels")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let total_tunnels = stats
            .get("total_tunnels")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let connected_edges = stats
            .get("connected_edges")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let total_bytes_ingress = stats
            .get("total_bytes_ingress")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let total_bytes_egress = stats
            .get("total_bytes_egress")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let uptime_secs = stats
            .get("uptime_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        DeviceMetricsSummary {
            metrics: vec![
                (
                    "active_tunnels".into(),
                    serde_json::json!(active_tunnels),
                ),
                (
                    "total_tunnels".into(),
                    serde_json::json!(total_tunnels),
                ),
                (
                    "connected_edges".into(),
                    serde_json::json!(connected_edges),
                ),
                (
                    "total_bytes_ingress".into(),
                    serde_json::json!(total_bytes_ingress),
                ),
                (
                    "total_bytes_egress".into(),
                    serde_json::json!(total_bytes_egress),
                ),
                (
                    "uptime_secs".into(),
                    serde_json::json!(uptime_secs),
                ),
            ],
            items: tunnels.cloned().unwrap_or_default(),
        }
    }

    fn extract_health_status(&self, health: &serde_json::Value) -> Option<String> {
        health
            .get("status")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string())
    }

    fn supported_commands(&self) -> Vec<CommandDescriptor> {
        RELAY_COMMANDS
            .iter()
            .map(|(name, desc, role)| CommandDescriptor {
                name: name.to_string(),
                description: desc.to_string(),
                requires_role: role.to_string(),
            })
            .collect()
    }

    fn validate_command(&self, action: &serde_json::Value) -> Result<(), String> {
        let cmd_type = action
            .get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| "Command must have a 'type' field".to_string())?;

        let known = RELAY_COMMANDS.iter().any(|(name, _, _)| *name == cmd_type);
        if !known {
            return Err(format!(
                "Unknown relay command type '{}'. Known commands: {}",
                cmd_type,
                RELAY_COMMANDS
                    .iter()
                    .map(|(n, _, _)| *n)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Validate required parameters
        match cmd_type {
            "disconnect_edge" => {
                action
                    .get("edge_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "disconnect_edge requires 'edge_id' field".to_string())?;
                Ok(())
            }
            "close_tunnel" => {
                action
                    .get("tunnel_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "close_tunnel requires 'tunnel_id' field".to_string())?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn ai_context(&self) -> Option<AiDeviceContext> {
        None
    }
}
