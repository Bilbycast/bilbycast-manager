// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use serde::{Deserialize, Serialize};

/// Edge node health response (mirrors bilbycast-edge /health).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeHealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub active_flows: u32,
    pub total_flows: u32,
}

/// Edge node configuration response (mirrors bilbycast-edge AppConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeConfigResponse {
    pub version: u32,
    pub server: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor: Option<serde_json::Value>,
    #[serde(default)]
    pub flows: Vec<crate::models::FlowConfig>,
}

/// Standard envelope response from edge API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}
