// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

//! Device driver trait and registry for managing different types of network devices.
//!
//! Each device type (edge transport node, relay server, encoder, etc.) implements
//! the `DeviceDriver` trait. Drivers are registered at startup and provide
//! device-specific behavior: metrics extraction, command validation, AI context, etc.

pub mod edge;
pub mod relay;

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

// ───────────────────────────────────────────────────────
// Types
// ───────────────────────────────────────────────────────

/// Summary metrics extracted from device-specific stats for dashboard display.
/// Each driver decides what metrics are relevant for its device type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceMetricsSummary {
    /// Key-value pairs of named metrics (e.g., "active_flows" → 3).
    pub metrics: Vec<(String, serde_json::Value)>,
    /// Optional structured sub-items (flows for edge, tunnels for relay, etc.).
    pub items: Vec<serde_json::Value>,
}

/// Describes a command supported by a device type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDescriptor {
    /// Command type identifier (e.g., "create_flow", "get_config").
    pub name: String,
    /// Human-readable description of what the command does.
    pub description: String,
    /// Minimum role required to execute this command ("operator" or "admin").
    pub requires_role: String,
}

/// AI context provided by a device driver for config generation and queries.
#[derive(Debug, Clone)]
pub struct AiDeviceContext {
    /// Protocol documentation (input/output types, parameters, etc.).
    pub protocol_docs: String,
    /// JSON schema for the device's configuration format.
    pub config_schema: String,
}

// ───────────────────────────────────────────────────────
// Trait
// ───────────────────────────────────────────────────────

/// Trait that each managed device type implements.
///
/// Drivers live in `manager-core` and have no framework (Axum) dependency.
/// The server uses drivers through the `DriverRegistry` on `AppState`.
pub trait DeviceDriver: Send + Sync + 'static {
    /// Unique identifier for this device type (stored in DB `device_type` column).
    fn device_type(&self) -> &str;

    /// Human-readable display name (e.g., "Edge Transport Node").
    fn display_name(&self) -> &str;

    /// Extract summary metrics from a device's cached stats payload.
    fn extract_metrics(&self, stats: &serde_json::Value) -> DeviceMetricsSummary;

    /// Extract a health status string from a health payload (e.g., "healthy", "degraded").
    fn extract_health_status(&self, health: &serde_json::Value) -> Option<String>;

    /// List of commands supported by this device type.
    fn supported_commands(&self) -> Vec<CommandDescriptor>;

    /// Validate a command action before sending it to the device.
    /// Returns Ok(()) if valid, Err with reason if not.
    fn validate_command(&self, action: &serde_json::Value) -> Result<(), String>;

    /// Provide AI context documentation for this device type.
    /// Returns None if AI assistance is not available for this device.
    fn ai_context(&self) -> Option<AiDeviceContext>;
}

// ───────────────────────────────────────────────────────
// Registry
// ───────────────────────────────────────────────────────

/// Registry of all available device drivers, keyed by device type string.
/// Built at startup, immutable after construction.
pub struct DriverRegistry {
    drivers: HashMap<String, Arc<dyn DeviceDriver>>,
}

impl DriverRegistry {
    pub fn new() -> Self {
        Self {
            drivers: HashMap::new(),
        }
    }

    /// Register a device driver. Panics if a driver with the same type is already registered.
    pub fn register(&mut self, driver: Arc<dyn DeviceDriver>) {
        let key = driver.device_type().to_string();
        if self.drivers.contains_key(&key) {
            panic!("Device driver '{}' is already registered", key);
        }
        self.drivers.insert(key, driver);
    }

    /// Look up a driver by device type.
    pub fn get(&self, device_type: &str) -> Option<Arc<dyn DeviceDriver>> {
        self.drivers.get(device_type).cloned()
    }

    /// List all registered drivers.
    pub fn all(&self) -> Vec<Arc<dyn DeviceDriver>> {
        self.drivers.values().cloned().collect()
    }

    /// List all registered device type identifiers.
    pub fn device_types(&self) -> Vec<&str> {
        self.drivers.keys().map(|k| k.as_str()).collect()
    }

    /// Check if a device type is registered.
    pub fn is_registered(&self, device_type: &str) -> bool {
        self.drivers.contains_key(device_type)
    }
}

impl Default for DriverRegistry {
    fn default() -> Self {
        Self::new()
    }
}
