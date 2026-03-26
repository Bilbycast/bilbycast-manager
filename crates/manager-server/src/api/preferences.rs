// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::models::UserRole;

/// Maximum payload size for preference values.
const MAX_PREF_VALUE_SIZE: usize = 50 * 1024; // 50 KB

/// Allowed preference keys (whitelist).
const ALLOWED_PREF_KEYS: &[&str] = &[
    "flow_table_order",
    "tunnel_table_order",
];

fn is_valid_pref_key(key: &str) -> bool {
    ALLOWED_PREF_KEYS.contains(&key)
}

#[derive(Deserialize)]
pub struct SetPreferenceRequest {
    pub value: String,
}

/// GET /api/v1/preferences/:key
pub async fn get_preference(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::Viewer) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !is_valid_pref_key(&key) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let value = manager_core::db::ui_preferences::get_preference(&state.db, &auth.user_id, &key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match value {
        Some(v) => Ok(Json(serde_json::json!({ "value": v }))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// PUT /api/v1/preferences/:key
pub async fn set_preference(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(key): Path<String>,
    Json(body): Json<SetPreferenceRequest>,
) -> Result<StatusCode, StatusCode> {
    if !auth.role.has_permission(UserRole::Viewer) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !is_valid_pref_key(&key) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if body.value.len() > MAX_PREF_VALUE_SIZE {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    manager_core::db::ui_preferences::set_preference(&state.db, &auth.user_id, &key, &body.value)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/preferences/:key
pub async fn delete_preference(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(key): Path<String>,
) -> Result<StatusCode, StatusCode> {
    if !auth.role.has_permission(UserRole::Viewer) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !is_valid_pref_key(&key) {
        return Err(StatusCode::BAD_REQUEST);
    }

    manager_core::db::ui_preferences::delete_preference(&state.db, &auth.user_id, &key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
