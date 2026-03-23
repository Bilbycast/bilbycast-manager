// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use super::{AiContext, AiError, AiProviderTrait, AnomalyReport, FlowConfigSuggestion};

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "claude-sonnet-4-6".to_string()),
        }
    }
}

#[async_trait]
impl AiProviderTrait for AnthropicProvider {
    async fn generate_flow_config(
        &self,
        prompt: &str,
        context: &AiContext,
    ) -> Result<FlowConfigSuggestion, AiError> {
        let system_prompt = format!(
            "You are a bilbycast media streaming configuration assistant. \
             Generate valid JSON flow configurations for bilbycast-edge nodes.\n\n\
             Protocol documentation:\n{}\n\n\
             FlowConfig JSON schema:\n{}\n\n\
             Available nodes:\n{}\n\n\
             Respond with a JSON object containing 'config' (the FlowConfig JSON) \
             and 'explanation' (brief description of what was configured). \
             Output ONLY the JSON, no markdown fences.",
            context.protocol_docs,
            context.flow_config_schema,
            context
                .node_info
                .iter()
                .map(|n| format!("- {} ({})", n.name, n.node_id))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let body = json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": system_prompt,
            "messages": [
                {"role": "user", "content": prompt}
            ]
        });

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::RequestFailed(e.to_string()))?;

        if resp.status() == 429 {
            return Err(AiError::RateLimited);
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;

        let content = json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| AiError::InvalidResponse("No content in response".into()))?;

        let parsed: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;

        Ok(FlowConfigSuggestion {
            config_json: parsed["config"].to_string(),
            explanation: parsed["explanation"]
                .as_str()
                .unwrap_or("Configuration generated")
                .to_string(),
        })
    }

    async fn analyze_anomaly(
        &self,
        description: &str,
        context: &str,
    ) -> Result<AnomalyReport, AiError> {
        let body = json!({
            "model": self.model,
            "max_tokens": 2048,
            "system": "You are a media streaming expert analyzing system anomalies. Respond with JSON containing 'findings' (array of strings) and 'suggestions' (array of strings). Output ONLY JSON.",
            "messages": [
                {"role": "user", "content": format!("Analyze this situation:\n{description}\n\nContext:\n{context}")}
            ]
        });

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::RequestFailed(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;

        let content = json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| AiError::InvalidResponse("No content".into()))?;

        let parsed: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;

        Ok(AnomalyReport {
            findings: parsed["findings"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            suggestions: parsed["suggestions"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        })
    }

    async fn answer_query(&self, query: &str, system_state: &str) -> Result<String, AiError> {
        let body = json!({
            "model": self.model,
            "max_tokens": 2048,
            "system": format!("You are a bilbycast media streaming system assistant. Current system state:\n{system_state}"),
            "messages": [
                {"role": "user", "content": query}
            ]
        });

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::RequestFailed(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;

        json["content"][0]["text"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| AiError::InvalidResponse("No content".into()))
    }
}
