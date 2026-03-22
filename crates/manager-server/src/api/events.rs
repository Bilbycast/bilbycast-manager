use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::models::{Event, EventQuery};

pub async fn list_events(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(mut query): Query<EventQuery>,
) -> Result<Json<Vec<Event>>, StatusCode> {
    // Clamp query params
    if let Some(ref s) = query.search
        && s.len() > 256
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(pp) = query.per_page {
        query.per_page = Some(pp.min(200));
    }

    let events = manager_core::db::events::query_events(&state.db, &query)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(events))
}

pub async fn acknowledge_event(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    manager_core::db::events::acknowledge_event(&state.db, id, &auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

pub async fn unacknowledged_count(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let count = manager_core::db::events::count_unacknowledged(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"count": count})))
}
