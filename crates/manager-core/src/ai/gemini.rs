use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use super::{AiContext, AiError, AiProviderTrait, AnomalyReport, FlowConfigSuggestion};

pub struct GeminiProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "gemini-3-flash".to_string()),
        }
    }

    async fn generate_content(
        &self,
        system: &str,
        prompt: &str,
    ) -> Result<String, AiError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let body = json!({
            "system_instruction": {
                "parts": [{"text": system}]
            },
            "contents": [
                {"parts": [{"text": prompt}]}
            ],
            "generationConfig": {
                "temperature": 0.3,
                "responseMimeType": "application/json"
            }
        });

        let resp = self
            .client
            .post(&url)
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

        json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| AiError::InvalidResponse("No content in response".into()))
    }
}

#[async_trait]
impl AiProviderTrait for GeminiProvider {
    async fn generate_flow_config(
        &self,
        prompt: &str,
        context: &AiContext,
    ) -> Result<FlowConfigSuggestion, AiError> {
        let system = format!(
            "You are a bilbycast media streaming configuration assistant. \
             Generate valid JSON flow configurations.\n\n\
             Protocol docs:\n{}\n\nSchema:\n{}\n\n\
             Respond with JSON: {{\"config\": <FlowConfig>, \"explanation\": \"...\"}}",
            context.protocol_docs, context.flow_config_schema
        );

        let content = self.generate_content(&system, prompt).await?;
        let parsed: serde_json::Value = serde_json::from_str(&content)
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
        let system = "Analyze media streaming anomalies. Respond with JSON: {\"findings\": [...], \"suggestions\": [...]}";
        let prompt = format!("Situation:\n{description}\n\nContext:\n{context}");

        let content = self.generate_content(system, &prompt).await?;
        let parsed: serde_json::Value = serde_json::from_str(&content)
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
        let system = format!("You are a bilbycast system assistant. State:\n{system_state}");
        self.generate_content(&system, query).await
    }
}
