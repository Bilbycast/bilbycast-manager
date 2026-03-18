use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::models::UserRole;

pub async fn get_settings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    let settings = manager_core::db::settings::get_all_settings(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut map = serde_json::Map::new();
    for (key, value) in settings {
        // Try to parse as JSON value, fallback to string
        let parsed = serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value));
        map.insert(key, parsed);
    }

    Ok(Json(serde_json::Value::Object(map)))
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub settings: std::collections::HashMap<String, serde_json::Value>,
}

pub async fn update_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<StatusCode, StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    for (key, value) in &req.settings {
        let value_str = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        manager_core::db::settings::set_setting(&state.db, key, &value_str, Some(&auth.user_id))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let _ = manager_core::db::audit::log_audit(
        &state.db,
        Some(&auth.user_id),
        "settings.update",
        Some("settings"),
        None,
        Some(&serde_json::to_value(&req.settings).unwrap_or_default()),
        None,
    )
    .await;

    Ok(StatusCode::OK)
}
