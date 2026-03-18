use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::models::UserRole;

#[derive(Deserialize)]
pub struct ExportQuery {
    pub include_events: Option<bool>,
    pub events_days: Option<u32>,
    pub include_audit: Option<bool>,
}

pub async fn export_data(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ExportQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::SuperAdmin) {
        return Err(StatusCode::FORBIDDEN);
    }

    let export = manager_core::export::export_all(
        &state.db,
        &auth.user_id,
        query.include_events.unwrap_or(false),
        query.events_days,
        query.include_audit.unwrap_or(false),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let json = serde_json::to_value(export).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json))
}

pub async fn import_data(
    State(_state): State<AppState>,
    auth: AuthUser,
    Json(_body): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    if !auth.role.has_permission(UserRole::SuperAdmin) {
        return Err(StatusCode::FORBIDDEN);
    }

    // TODO: implement import logic
    let _ = auth;
    Ok(StatusCode::NOT_IMPLEMENTED)
}
