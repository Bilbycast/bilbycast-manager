use serde::{Deserialize, Serialize};

/// System settings stored in the database key-value store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    /// How many days to keep events before cleanup (default: 30).
    pub events_retention_days: u32,
    /// WebSocket keepalive ping interval in seconds (default: 15).
    pub ws_keepalive_interval_secs: u32,
    /// User session lifetime in hours (default: 24).
    pub session_lifetime_hours: u32,
    /// Max login attempts per IP per minute (default: 5).
    pub max_login_attempts: u32,
    /// Seconds before a node is considered offline after last heartbeat (default: 30).
    pub node_offline_threshold_secs: u32,
    /// How often aggregated stats are broadcast to browsers, in ms (default: 1000).
    pub stats_broadcast_interval_ms: u32,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            events_retention_days: 30,
            ws_keepalive_interval_secs: 15,
            session_lifetime_hours: 24,
            max_login_attempts: 5,
            node_offline_threshold_secs: 30,
            stats_broadcast_interval_ms: 1000,
        }
    }
}

/// A single key-value setting row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingRow {
    pub key: String,
    pub value: serde_json::Value,
}

/// AI API key entry for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiKeyEntry {
    pub id: String,
    pub user_id: String,
    pub provider: AiProvider,
    pub model_preference: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Supported AI providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    Openai,
    Anthropic,
    Gemini,
}

impl AiProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "openai" => Some(Self::Openai),
            "anthropic" => Some(Self::Anthropic),
            "gemini" => Some(Self::Gemini),
            _ => None,
        }
    }
}

/// Configuration template for reusable flow setups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub template: serde_json::Value,
    pub created_by: Option<String>,
    pub created_at: String,
}
