use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use super::{AiContext, AiError, AiProviderTrait, AnomalyReport, FlowConfigSuggestion};

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "gpt-5.4-mini".to_string()),
        }
    }
}

#[async_trait]
impl AiProviderTrait for OpenAiProvider {
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
             and 'explanation' (brief description of what was configured).",
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
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.3,
            "response_format": {"type": "json_object"}
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
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

        let content = json["choices"][0]["message"]["content"]
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
            "messages": [
                {"role": "system", "content": "You are a media streaming expert analyzing system anomalies. Respond with JSON containing 'findings' (array of strings) and 'suggestions' (array of strings)."},
                {"role": "user", "content": format!("Analyze this situation:\n{description}\n\nContext:\n{context}")}
            ],
            "temperature": 0.3,
            "response_format": {"type": "json_object"}
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::RequestFailed(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;

        let content = json["choices"][0]["message"]["content"]
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
            "messages": [
                {"role": "system", "content": format!("You are a bilbycast media streaming system assistant. Current system state:\n{system_state}")},
                {"role": "user", "content": query}
            ],
            "temperature": 0.5
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::RequestFailed(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| AiError::InvalidResponse("No content".into()))
    }
}
