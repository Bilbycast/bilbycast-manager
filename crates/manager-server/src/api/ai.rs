// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::validation;

#[derive(Deserialize)]
pub struct GenerateConfigRequest {
    pub prompt: String,
    pub provider: Option<String>,
    /// Node ID for fetching real flow configs from the hub
    pub node_id: Option<String>,
    /// Optional: existing flow configs on the node for context (sent from UI)
    pub existing_flows: Option<Vec<serde_json::Value>>,
}

pub async fn generate_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<GenerateConfigRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Input validation
    validation::validate_string_length(&req.prompt, "prompt", 10000)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e}))))?;

    let provider_name = req.provider.as_deref().unwrap_or("openai");

    // Get the user's API key and model preference for this provider
    let (api_key, model_preference) =
        get_user_api_key_and_model(&state, &auth.user_id, provider_name).await?;

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

    // Try to fetch real flow configs from the node hub (not just stats)
    let flow_context = if let Some(ref node_id) = req.node_id {
        if auth.can_access_node(node_id) {
            // Try cached config first (fast, no waiting)
            if let Some(config) = state.node_hub.get_cached_config(node_id).await {
                if let Some(flows) = config.get("flows") {
                    Some(serde_json::to_string_pretty(flows).unwrap_or_default())
                } else {
                    None
                }
            } else {
                // Fall back to UI-provided flows
                req.existing_flows.as_ref().map(|f| {
                    serde_json::to_string_pretty(f).unwrap_or_default()
                })
            }
        } else {
            None
        }
    } else {
        req.existing_flows.as_ref().map(|f| {
            serde_json::to_string_pretty(f).unwrap_or_default()
        })
    };

    let mut system_prompt = format!(
        "You are a bilbycast media streaming configuration assistant.\n\
         You help manage flow configurations on bilbycast-edge nodes.\n\n\
         {}\n\n\
         FlowConfig JSON schema:\n{}\n\n",
        protocol_docs,
        config_schema,
    );

    if let Some(ref flows_str) = flow_context {
        if !flows_str.is_empty() && flows_str != "[]" && flows_str != "null" {
            system_prompt.push_str(&format!(
                "Current flows on this node:\n{}\n\n",
                flows_str
            ));
        }
    }

    // Add available nodes context for tunnel creation
    if let Ok(nodes) = manager_core::db::nodes::list_nodes(&state.db).await {
        let nodes_ctx: Vec<serde_json::Value> = nodes.iter().map(|n| serde_json::json!({
            "id": n.id, "name": n.name, "device_type": n.device_type, "status": n.status
        })).collect();
        if !nodes_ctx.is_empty() {
            system_prompt.push_str(&format!(
                "Available nodes (use these IDs for tunnel ingress/egress):\n{}\n\n",
                serde_json::to_string_pretty(&nodes_ctx).unwrap_or_default()
            ));
        }
    }

    system_prompt.push_str(
        "CRITICAL INSTRUCTIONS:\n\
         1. Respond with ONLY valid JSON — no markdown fences, no explanations, no extra text.\n\
         2. Your response MUST be a JSON object with an \"action\" field indicating what to do.\n\n\
         SUPPORTED ACTIONS:\n\n\
         CREATE a new flow:\n\
         {\"action\": \"create_flow\", \"flow\": {<FlowConfig>}, \"message\": \"description of what was created\"}\n\n\
         UPDATE an existing flow (change settings, add/remove outputs):\n\
         {\"action\": \"update_flow\", \"flow\": {<complete updated FlowConfig with SAME id>}, \"message\": \"description of changes\"}\n\
         IMPORTANT: Include the COMPLETE FlowConfig with ALL fields — the original input AND all outputs (existing + any new ones). Use the SAME \"id\" as the existing flow.\n\n\
         DELETE a flow:\n\
         {\"action\": \"delete_flow\", \"flow_id\": \"the-flow-id\", \"message\": \"description\"}\n\n\
         ADD an output to an existing flow:\n\
         {\"action\": \"add_output\", \"flow_id\": \"the-flow-id\", \"output\": {<output object>}, \"message\": \"description\"}\n\n\
         REMOVE an output from an existing flow:\n\
         {\"action\": \"remove_output\", \"flow_id\": \"the-flow-id\", \"output_id\": \"output-id\", \"message\": \"description\"}\n\n\
         START a stopped flow:\n\
         {\"action\": \"start_flow\", \"flow_id\": \"the-flow-id\", \"message\": \"description\"}\n\n\
         STOP a running flow:\n\
         {\"action\": \"stop_flow\", \"flow_id\": \"the-flow-id\", \"message\": \"description\"}\n\n\
         RESTART a flow:\n\
         {\"action\": \"restart_flow\", \"flow_id\": \"the-flow-id\", \"message\": \"description\"}\n\n\
         CREATE a tunnel between two edge nodes:\n\
         {\"action\": \"create_tunnel\", \"tunnel\": {\"name\": \"tunnel-name\", \"protocol\": \"tcp\", \"mode\": \"direct\", \"ingress_node_id\": \"node-id\", \"ingress_listen_port\": 9100, \"egress_node_id\": \"node-id\", \"egress_forward_addr\": \"127.0.0.1:9100\", \"egress_peer_addr\": \"203.0.113.50:10100\"}, \"message\": \"description\"}\n\
         For relay mode, add \"relay_addr\": \"relay-host:4433\" to the tunnel object (omit egress_peer_addr).\n\n\
         DELETE a tunnel:\n\
         {\"action\": \"delete_tunnel\", \"tunnel_id\": \"the-tunnel-id\", \"message\": \"description\"}\n\n\
         ANSWER a question or provide information (no config change):\n\
         {\"action\": \"info\", \"message\": \"your answer here\"}\n\n\
         MULTIPLE actions at once:\n\
         {\"action\": \"multiple\", \"actions\": [<array of action objects>], \"message\": \"summary\"}\n\n\
         RULES FOR TUNNEL FIELDS:\n\
         - \"name\": descriptive tunnel name (lowercase with hyphens)\n\
         - \"protocol\": \"tcp\" (reliable, ordered) or \"udp\" (media/SRT)\n\
         - \"mode\": \"relay\" (both behind NAT, requires relay_addr) or \"direct\" (one side has public IP)\n\
         - \"ingress_node_id\": node ID that RECEIVES tunnel traffic (entry point) — use exact IDs from available nodes list\n\
         - \"ingress_listen_port\": port on ingress node to listen on (1024-65535)\n\
         - \"egress_node_id\": node ID that SENDS traffic into the tunnel (exit point) — use exact IDs from available nodes list\n\
         - \"egress_forward_addr\": where egress node delivers tunnel traffic, e.g. \"127.0.0.1:9100\"\n\
         - \"egress_peer_addr\": reachable IP:port of egress node's QUIC listener (REQUIRED for direct mode, omit for relay mode). This is the public/routable address the ingress node connects to\n\
         - \"relay_addr\": relay server QUIC address (REQUIRED for relay mode, omit for direct mode)\n\n\
         RULES FOR FlowConfig FIELDS:\n\
         - \"id\": string (unique, lowercase with hyphens, e.g. \"srt-listener-1\")\n\
         - \"name\": string (human readable)\n\
         - \"enabled\": true\n\
         - \"input\": object with \"type\" field and type-specific fields\n\
         - \"outputs\": array of output objects, each with \"type\", \"id\", \"name\" and type-specific fields\n\
         - For SRT: \"mode\" (\"listener\"/\"caller\"/\"rendezvous\"), \"local_addr\", \"latency_ms\" (integer)\n\
         - Optional SRT: \"passphrase\" (10-79 chars), \"aes_key_len\" (16, 24, or 32), \"remote_addr\" (for caller)\n\
         - For RTP outputs: \"dest_addr\", \"dscp\" (default 46)\n\
         - For RTMP outputs: \"dest_url\", \"stream_key\"\n\
         - IMPORTANT: Use exact field names. NOT \"key_length\" — use \"aes_key_len\". NOT \"address\" — use \"local_addr\".\n\
         - Generate realistic port numbers (9000-9999) and reasonable latency (120-500ms)\n\
         - When referencing existing flows, use their EXACT \"id\" values from the current flows listed above."
    );

    // Call the AI provider with model preference
    let result = call_ai_provider(
        provider_name,
        &api_key,
        model_preference.as_deref(),
        &system_prompt,
        &req.prompt,
    )
    .await;

    match result {
        Ok(response_text) => {
            // Try to parse the response as JSON
            match serde_json::from_str::<serde_json::Value>(&response_text) {
                Ok(parsed) => {
                    // Check if it's already an action envelope
                    if parsed.get("action").is_some() {
                        Ok(Json(serde_json::json!({
                            "success": true,
                            "config": parsed,
                            "raw_response": response_text
                        })))
                    } else if parsed.get("id").is_some() && parsed.get("input").is_some() {
                        // Legacy format: raw FlowConfig — wrap in create_flow action
                        Ok(Json(serde_json::json!({
                            "success": true,
                            "config": {
                                "action": "create_flow",
                                "flow": parsed,
                                "message": "Generated flow configuration"
                            },
                            "raw_response": response_text
                        })))
                    } else {
                        // Unknown JSON shape — return as-is
                        Ok(Json(serde_json::json!({
                            "success": true,
                            "config": parsed,
                            "raw_response": response_text
                        })))
                    }
                }
                Err(_) => {
                    // Not valid JSON — treat as info response
                    Ok(Json(serde_json::json!({
                        "success": true,
                        "config": {
                            "action": "info",
                            "message": response_text
                        },
                        "raw_response": response_text
                    })))
                }
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
    pub provider: Option<String>,
}

pub async fn analyze_anomaly(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Input validation
    validation::validate_string_length(&req.description, "description", 10000)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e}))))?;

    let provider_name = req.provider.as_deref().unwrap_or("openai");
    let (api_key, model_preference) =
        get_user_api_key_and_model(&state, &auth.user_id, provider_name).await?;

    let system = "You are a media streaming expert. Analyze the situation and provide findings and suggestions. Be concise and actionable.";
    let result = call_ai_provider(provider_name, &api_key, model_preference.as_deref(), system, &req.description).await;

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
    // Input validation
    validation::validate_string_length(&req.query, "query", 10000)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e}))))?;

    let provider_name = req.provider.as_deref().unwrap_or("openai");
    let (api_key, model_preference) =
        get_user_api_key_and_model(&state, &auth.user_id, provider_name).await?;

    let system = "You are a bilbycast media streaming system assistant. Answer questions about the system concisely.";
    let result = call_ai_provider(provider_name, &api_key, model_preference.as_deref(), system, &req.query).await;

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
    // Input validation
    if req.api_key.is_empty() || req.api_key.len() > 256 {
        return Ok(Json(serde_json::json!({ "success": false, "error": "API key must be 1–256 characters" })));
    }

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

/// Get the user's API key and optional model preference for a provider.
async fn get_user_api_key_and_model(
    state: &AppState,
    user_id: &str,
    provider: &str,
) -> Result<(String, Option<String>), (StatusCode, Json<serde_json::Value>)> {
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT api_key_enc, model_preference FROM ai_keys WHERE user_id = ? AND provider = ?",
    )
    .bind(user_id)
    .bind(provider)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "success": false, "error": "Database error while fetching API key" })),
    ))?;

    let (encrypted, model_preference) = row
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
        ))?;

    let api_key = manager_core::crypto::decrypt(&encrypted, &state.master_key)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "success": false, "error": "Failed to decrypt API key. The master key may have changed." })),
        ))?;

    Ok((api_key, model_preference))
}

async fn call_ai_provider(
    provider: &str,
    api_key: &str,
    model_preference: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();

    match provider {
        "openai" => call_openai(&client, api_key, model_preference, system_prompt, user_prompt).await,
        "anthropic" => call_anthropic(&client, api_key, model_preference, system_prompt, user_prompt).await,
        "gemini" => call_gemini(&client, api_key, model_preference, system_prompt, user_prompt).await,
        _ => Err(format!("Unknown provider: {provider}")),
    }
}

async fn call_openai(
    client: &reqwest::Client,
    api_key: &str,
    model_preference: Option<&str>,
    system: &str,
    prompt: &str,
) -> Result<String, String> {
    let model = model_preference.unwrap_or("gpt-4.1");
    let body = serde_json::json!({
        "model": model,
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
    model_preference: Option<&str>,
    system: &str,
    prompt: &str,
) -> Result<String, String> {
    let model = model_preference.unwrap_or("claude-sonnet-4-6-20260318");
    let body = serde_json::json!({
        "model": model,
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
    model_preference: Option<&str>,
    system: &str,
    prompt: &str,
) -> Result<String, String> {
    let model = model_preference.unwrap_or("gemini-2.0-flash");
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={api_key}",
        model
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
