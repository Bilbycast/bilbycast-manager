use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;

#[derive(Deserialize)]
pub struct GenerateConfigRequest {
    pub prompt: String,
    pub provider: Option<String>,
    /// Optional: existing flow configs on the node for context
    pub existing_flows: Option<Vec<serde_json::Value>>,
    pub node_id: Option<String>,
}

pub async fn generate_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<GenerateConfigRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let provider_name = req.provider.as_deref().unwrap_or("openai");

    // Get the user's API key for this provider
    let api_key = get_user_api_key(&state, &auth.user_id, provider_name).await?;

    // Build context from device drivers (or fall back to defaults)
    let (protocol_docs, config_schema) = {
        let mut docs = String::new();
        let mut schema = String::new();
        for driver in state.driver_registry.all() {
            if let Some(ctx) = driver.ai_context() {
                docs.push_str(&ctx.protocol_docs);
                schema.push_str(&ctx.config_schema);
            }
        }
        if docs.is_empty() {
            docs = manager_core::ai::config_gen::PROTOCOL_DOCS.to_string();
        }
        if schema.is_empty() {
            schema = manager_core::ai::config_gen::FLOW_CONFIG_SCHEMA.to_string();
        }
        (docs, schema)
    };

    let mut system_prompt = format!(
        "You are a bilbycast media streaming configuration assistant.\n\
         You generate valid JSON flow configurations for bilbycast-edge nodes.\n\n\
         {}\n\n\
         FlowConfig JSON schema:\n{}\n\n",
        protocol_docs,
        config_schema,
    );

    if let Some(ref flows) = req.existing_flows {
        system_prompt.push_str(&format!(
            "Current flows on this node:\n{}\n\n",
            serde_json::to_string_pretty(flows).unwrap_or_default()
        ));
    }

    system_prompt.push_str(
        "CRITICAL INSTRUCTIONS:\n\
         1. Respond with ONLY valid JSON — no markdown fences, no explanations, no extra text.\n\
         2. The JSON MUST be a single FlowConfig object with these exact fields:\n\
            - \"id\": string (unique, lowercase, e.g. \"srt-listener-1\")\n\
            - \"name\": string (human readable)\n\
            - \"enabled\": true\n\
            - \"input\": object with \"type\" field (\"srt\" or \"rtp\") and type-specific fields\n\
            - \"outputs\": array of output objects, each with \"type\", \"id\", \"name\" and type-specific fields\n\
         3. For SRT inputs: include \"mode\" (\"listener\"/\"caller\"/\"rendezvous\"), \"local_addr\" (e.g. \"0.0.0.0:9000\"), \"latency_ms\" (integer)\n\
            Optional SRT fields: \"passphrase\" (string, 10-79 chars), \"aes_key_len\" (integer: 16, 24, or 32), \"remote_addr\" (for caller mode)\n\
         4. For SRT outputs: include \"mode\", \"local_addr\", \"latency_ms\". Add \"remote_addr\" for caller mode\n\
         5. For RTP outputs: include \"dest_addr\" (e.g. \"239.1.1.1:5004\"), \"dscp\" (integer, default 46)\n\
         6. For RTMP outputs: include \"dest_url\", \"stream_key\", \"reconnect_delay_secs\"\n\
         7. For HLS outputs: include \"ingest_url\", \"segment_duration_secs\", \"max_segments\"\n\
         8. For WebRTC outputs: include \"whip_url\", optionally \"bearer_token\", \"video_only\"\n\
         9. IMPORTANT: Use exact field names as shown. NOT \"key_length\" — use \"aes_key_len\". NOT \"address\" — use \"local_addr\" or \"dest_addr\".\n\
         10. Generate realistic port numbers (9000-9999) and reasonable latency (120-500ms)\n\
         11. If user asks to modify an existing flow (e.g. add output, change settings), return the COMPLETE updated FlowConfig with the SAME \"id\" as the existing flow. Include the original input and ALL outputs (existing + new). The system will automatically replace the old flow with the updated config.\n\
         12. NEVER create a new flow ID when the user is asking to modify an existing flow. Reuse the existing flow's \"id\"."
    );

    // Call the AI provider
    let result = call_ai_provider(provider_name, &api_key, &system_prompt, &req.prompt).await;

    match result {
        Ok(response_text) => {
            // Try to parse the response as JSON
            match serde_json::from_str::<serde_json::Value>(&response_text) {
                Ok(config) => Ok(Json(serde_json::json!({
                    "success": true,
                    "config": config,
                    "raw_response": response_text
                }))),
                Err(_) => Ok(Json(serde_json::json!({
                    "success": true,
                    "raw_response": response_text,
                    "config": null,
                    "note": "Response was not valid JSON - showing raw AI response"
                }))),
            }
        }
        Err(e) => Ok(Json(serde_json::json!({
            "success": false,
            "error": e
        }))),
    }
}

#[derive(Deserialize)]
pub struct AnalyzeRequest {
    pub description: String,
    pub node_id: Option<String>,
    pub provider: Option<String>,
}

pub async fn analyze_anomaly(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let provider_name = req.provider.as_deref().unwrap_or("openai");
    let api_key = get_user_api_key(&state, &auth.user_id, provider_name).await?;

    let system = "You are a media streaming expert. Analyze the situation and provide findings and suggestions. Be concise and actionable.";
    let result = call_ai_provider(provider_name, &api_key, system, &req.description).await;

    match result {
        Ok(text) => Ok(Json(serde_json::json!({ "success": true, "response": text }))),
        Err(e) => Ok(Json(serde_json::json!({ "success": false, "error": e }))),
    }
}

#[derive(Deserialize)]
pub struct QueryRequest {
    pub query: String,
    pub provider: Option<String>,
}

pub async fn answer_query(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<QueryRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let provider_name = req.provider.as_deref().unwrap_or("openai");
    let api_key = get_user_api_key(&state, &auth.user_id, provider_name).await?;

    let system = "You are a bilbycast media streaming system assistant. Answer questions about the system concisely.";
    let result = call_ai_provider(provider_name, &api_key, system, &req.query).await;

    match result {
        Ok(text) => Ok(Json(serde_json::json!({ "success": true, "response": text }))),
        Err(e) => Ok(Json(serde_json::json!({ "success": false, "error": e }))),
    }
}

pub async fn list_keys(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let rows: Vec<(String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT id, provider, model_preference, updated_at FROM ai_keys WHERE user_id = ? ORDER BY provider",
    )
    .bind(&auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let keys: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(id, provider, model, updated)| {
            serde_json::json!({
                "id": id,
                "provider": provider,
                "model_preference": model,
                "configured": true,
                "updated_at": updated
            })
        })
        .collect();

    Ok(Json(keys))
}

#[derive(Deserialize)]
pub struct SetKeyRequest {
    pub provider: String,
    pub api_key: String,
    pub model_preference: Option<String>,
}

pub async fn set_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SetKeyRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Validate provider
    if !["openai", "anthropic", "gemini"].contains(&req.provider.as_str()) {
        return Ok(Json(serde_json::json!({ "success": false, "error": "Invalid provider" })));
    }

    // Encrypt the API key
    let encrypted = manager_core::crypto::encrypt(&req.api_key, &state.master_key)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO ai_keys (id, user_id, provider, api_key_enc, model_preference, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(user_id, provider) DO UPDATE SET api_key_enc = excluded.api_key_enc, \
         model_preference = excluded.model_preference, updated_at = excluded.updated_at",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .bind(&req.provider)
    .bind(&encrypted)
    .bind(&req.model_preference)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Deserialize)]
pub struct DeleteKeyRequest {
    pub provider: String,
}

pub async fn delete_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<DeleteKeyRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    sqlx::query("DELETE FROM ai_keys WHERE user_id = ? AND provider = ?")
        .bind(&auth.user_id)
        .bind(&req.provider)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({ "success": true })))
}

// ── Internal helpers ──

async fn get_user_api_key(
    state: &AppState,
    user_id: &str,
    provider: &str,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT api_key_enc FROM ai_keys WHERE user_id = ? AND provider = ?",
    )
    .bind(user_id)
    .bind(provider)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "success": false, "error": "Database error while fetching API key" })),
    ))?;

    let encrypted = row
        .ok_or_else(|| (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "error": format!(
                    "No API key configured for '{}'. Go to AI Keys in the sidebar to add your {} API key.",
                    provider,
                    match provider {
                        "openai" => "OpenAI",
                        "anthropic" => "Anthropic Claude",
                        "gemini" => "Google Gemini",
                        _ => provider,
                    }
                )
            })),
        ))?
        .0;

    manager_core::crypto::decrypt(&encrypted, &state.master_key)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "success": false, "error": "Failed to decrypt API key. The master key may have changed." })),
        ))
}

async fn call_ai_provider(
    provider: &str,
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();

    match provider {
        "openai" => call_openai(&client, api_key, system_prompt, user_prompt).await,
        "anthropic" => call_anthropic(&client, api_key, system_prompt, user_prompt).await,
        "gemini" => call_gemini(&client, api_key, system_prompt, user_prompt).await,
        _ => Err(format!("Unknown provider: {provider}")),
    }
}

async fn call_openai(
    client: &reqwest::Client,
    api_key: &str,
    system: &str,
    prompt: &str,
) -> Result<String, String> {
    let body = serde_json::json!({
        "model": "gpt-4.1",
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.3
    });

    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Invalid response: {e}"))?;

    if !status.is_success() {
        return Err(format!("OpenAI API error {}: {}", status, json["error"]["message"].as_str().unwrap_or("unknown")));
    }

    json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "No content in response".into())
}

async fn call_anthropic(
    client: &reqwest::Client,
    api_key: &str,
    system: &str,
    prompt: &str,
) -> Result<String, String> {
    let body = serde_json::json!({
        "model": "claude-sonnet-4-6-20260318",
        "max_tokens": 4096,
        "system": system,
        "messages": [{"role": "user", "content": prompt}]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Invalid response: {e}"))?;

    if !status.is_success() {
        return Err(format!("Anthropic API error {}: {}", status, json["error"]["message"].as_str().unwrap_or("unknown")));
    }

    json["content"][0]["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "No content in response".into())
}

async fn call_gemini(
    client: &reqwest::Client,
    api_key: &str,
    system: &str,
    prompt: &str,
) -> Result<String, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={api_key}"
    );

    let body = serde_json::json!({
        "system_instruction": {"parts": [{"text": system}]},
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": {"temperature": 0.3}
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Invalid response: {e}"))?;

    if !status.is_success() {
        return Err(format!("Gemini API error {}: {}", status, json));
    }

    json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "No content in response".into())
}
