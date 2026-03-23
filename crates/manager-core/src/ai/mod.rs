// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

pub mod openai;
pub mod anthropic;
pub mod gemini;
pub mod config_gen;

use async_trait::async_trait;

/// Context provided to AI for configuration generation.
#[derive(Debug, Clone)]
pub struct AiContext {
    /// Available protocol types and their descriptions.
    pub protocol_docs: String,
    /// FlowConfig JSON schema.
    pub flow_config_schema: String,
    /// Current node names and capabilities for reference.
    pub node_info: Vec<NodeInfo>,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub name: String,
    pub node_id: String,
    pub active_flows: Vec<String>,
}

/// Result of an AI flow configuration generation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowConfigSuggestion {
    pub config_json: String,
    pub explanation: String,
}

/// Result of an AI anomaly analysis.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnomalyReport {
    pub findings: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Trait for AI providers (OpenAI, Anthropic, Google Gemini).
#[async_trait]
pub trait AiProviderTrait: Send + Sync {
    /// Generate a flow configuration from a natural language prompt.
    async fn generate_flow_config(
        &self,
        prompt: &str,
        context: &AiContext,
    ) -> Result<FlowConfigSuggestion, AiError>;

    /// Analyze stats and events for anomalies.
    async fn analyze_anomaly(
        &self,
        description: &str,
        context: &str,
    ) -> Result<AnomalyReport, AiError>;

    /// Answer a general query about the system.
    async fn answer_query(
        &self,
        query: &str,
        system_state: &str,
    ) -> Result<String, AiError>;
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("AI API request failed: {0}")]
    RequestFailed(String),
    #[error("AI API returned invalid response: {0}")]
    InvalidResponse(String),
    #[error("No API key configured for provider")]
    NoApiKey,
    #[error("Rate limited by AI provider")]
    RateLimited,
}
