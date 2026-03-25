// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use std::collections::HashMap;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::models::UserRole;

/// Maximum payload size for position save requests.
const MAX_POSITIONS_PAYLOAD_SIZE: usize = 50 * 1024; // 50 KB

#[derive(Deserialize, Default)]
pub struct PositionsQuery {
    pub view: Option<String>,
}

#[derive(Deserialize)]
pub struct SavePositionsRequest {
    pub view: String,
    pub positions: HashMap<String, PositionXY>,
}

#[derive(Deserialize, Clone)]
pub struct PositionXY {
    pub x: f64,
    pub y: f64,
}

/// GET /api/v1/topology/positions?view=graph
/// Returns saved node positions for the current user and view.
pub async fn get_positions(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<PositionsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::Viewer) {
        return Err(StatusCode::FORBIDDEN);
    }

    let view = query.view.as_deref().unwrap_or("graph");
    if view != "graph" && view != "flow" {
        return Err(StatusCode::BAD_REQUEST);
    }

    let rows = manager_core::db::topology_positions::get_positions(&state.db, &auth.user_id, view)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut positions = serde_json::Map::new();
    for (node_id, x, y) in rows {
        positions.insert(
            node_id,
            serde_json::json!({"x": x, "y": y}),
        );
    }

    Ok(Json(serde_json::json!({ "positions": positions })))
}

/// PUT /api/v1/topology/positions
/// Save node positions for the current user and view.
pub async fn save_positions(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    if !auth.role.has_permission(UserRole::Viewer) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check payload size
    let size = serde_json::to_string(&body).map(|s| s.len()).unwrap_or(0);
    if size > MAX_POSITIONS_PAYLOAD_SIZE {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    // Parse the request
    let req: SavePositionsRequest = serde_json::from_value(body)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    if req.view != "graph" && req.view != "flow" {
        return Err(StatusCode::BAD_REQUEST);
    }

    let positions: Vec<(String, f64, f64)> = req
        .positions
        .into_iter()
        .map(|(node_id, pos)| (node_id, pos.x, pos.y))
        .collect();

    manager_core::db::topology_positions::save_positions(
        &state.db,
        &auth.user_id,
        &req.view,
        &positions,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/topology/positions?view=graph
/// Clear all saved positions for the current user and view.
pub async fn clear_positions(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<PositionsQuery>,
) -> Result<StatusCode, StatusCode> {
    if !auth.role.has_permission(UserRole::Viewer) {
        return Err(StatusCode::FORBIDDEN);
    }

    let view = query.view.as_deref().unwrap_or("graph");
    if view != "graph" && view != "flow" {
        return Err(StatusCode::BAD_REQUEST);
    }

    manager_core::db::topology_positions::clear_positions(&state.db, &auth.user_id, view)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
