// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

//! Device driver for bilbycast-edge transport nodes.

use super::{AiDeviceContext, CommandDescriptor, DeviceDriver, DeviceMetricsSummary};

/// Driver for bilbycast-edge media transport gateway nodes.
pub struct EdgeDriver;

impl EdgeDriver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EdgeDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// Known edge command types.
const EDGE_COMMANDS: &[(&str, &str, &str)] = &[
    ("get_config", "Request the node's current configuration", "operator"),
    ("update_config", "Push a new configuration to the node", "operator"),
    ("create_flow", "Create a new media flow", "operator"),
    ("update_flow", "Update an existing flow's configuration", "operator"),
    ("delete_flow", "Delete a media flow", "operator"),
    ("start_flow", "Start a stopped flow", "operator"),
    ("stop_flow", "Stop a running flow", "operator"),
    ("restart_flow", "Restart a running flow", "operator"),
    ("add_output", "Add an output to an existing flow", "operator"),
    ("remove_output", "Remove an output from a flow", "operator"),
];

impl DeviceDriver for EdgeDriver {
    fn device_type(&self) -> &str {
        "edge"
    }

    fn display_name(&self) -> &str {
        "Edge Transport Node"
    }

    fn extract_metrics(&self, stats: &serde_json::Value) -> DeviceMetricsSummary {
        let flows = stats.get("flows").and_then(|f| f.as_array());
        let tunnels = stats.get("tunnels").and_then(|t| t.as_array());

        let total_flows = flows.map(|f| f.len() as u64).unwrap_or(0);
        let active_flows = flows
            .map(|f| {
                f.iter()
                    .filter(|flow| {
                        flow.get("state")
                            .and_then(|s| s.as_str())
                            .map(|s| s == "running")
                            .unwrap_or(false)
                    })
                    .count() as u64
            })
            .unwrap_or(0);

        let flow_bitrate: u64 = flows
            .map(|f| {
                f.iter()
                    .filter_map(|flow| {
                        flow.get("input")
                            .and_then(|i| i.get("bitrate_bps"))
                            .and_then(|b| b.as_u64())
                    })
                    .sum()
            })
            .unwrap_or(0);

        // Extract tunnel metrics
        let total_tunnels = tunnels.map(|t| t.len() as u64).unwrap_or(0);
        let active_tunnels = tunnels
            .map(|t| {
                t.iter()
                    .filter(|tun| {
                        tun.get("state")
                            .and_then(|s| s.as_str())
                            .map(|s| s == "ready")
                            .unwrap_or(false)
                    })
                    .count() as u64
            })
            .unwrap_or(0);

        let tunnel_bitrate: u64 = tunnels
            .map(|t| {
                t.iter()
                    .filter_map(|tun| {
                        let stats = tun.get("stats")?;
                        let in_bps = stats.get("bitrate_in_bps").and_then(|b| b.as_u64()).unwrap_or(0);
                        let out_bps = stats.get("bitrate_out_bps").and_then(|b| b.as_u64()).unwrap_or(0);
                        Some(in_bps.max(out_bps))
                    })
                    .sum()
            })
            .unwrap_or(0);

        let total_bitrate = flow_bitrate + tunnel_bitrate;

        // Combine flows and tunnels into items
        let mut items = flows.cloned().unwrap_or_default();
        if let Some(tunnel_items) = tunnels {
            items.extend(tunnel_items.iter().cloned());
        }

        DeviceMetricsSummary {
            metrics: vec![
                ("active_flows".into(), serde_json::json!(active_flows)),
                ("total_flows".into(), serde_json::json!(total_flows)),
                ("active_tunnels".into(), serde_json::json!(active_tunnels)),
                ("total_tunnels".into(), serde_json::json!(total_tunnels)),
                ("total_bitrate_bps".into(), serde_json::json!(total_bitrate)),
                (
                    "uptime_secs".into(),
                    serde_json::json!(stats.get("uptime_secs").and_then(|u| u.as_u64()).unwrap_or(0)),
                ),
            ],
            items,
        }
    }

    fn extract_health_status(&self, health: &serde_json::Value) -> Option<String> {
        health
            .get("status")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string())
    }

    fn supported_commands(&self) -> Vec<CommandDescriptor> {
        EDGE_COMMANDS
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

        let known = EDGE_COMMANDS.iter().any(|(name, _, _)| *name == cmd_type);
        if !known {
            return Err(format!(
                "Unknown edge command type '{}'. Known commands: {}",
                cmd_type,
                EDGE_COMMANDS
                    .iter()
                    .map(|(n, _, _)| *n)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Ok(())
    }

    fn ai_context(&self) -> Option<AiDeviceContext> {
        Some(AiDeviceContext {
            protocol_docs: crate::ai::config_gen::PROTOCOL_DOCS.to_string(),
            config_schema: crate::ai::config_gen::FLOW_CONFIG_SCHEMA.to_string(),
        })
    }
}
