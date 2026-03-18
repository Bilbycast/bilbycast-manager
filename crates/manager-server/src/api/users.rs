use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::models::{CreateUserRequest, UpdateUserRequest, UserInfo, UserRole};

pub async fn list_users(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<UserInfo>>, StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    let users = manager_core::db::users::list_users(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(users.into_iter().map(UserInfo::from).collect()))
}

pub async fn create_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserInfo>), StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Only super_admin can create super_admin users
    if req.role == UserRole::SuperAdmin && auth.role != UserRole::SuperAdmin {
        return Err(StatusCode::FORBIDDEN);
    }

    let user = manager_core::db::users::create_user(&state.db, &req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = manager_core::db::audit::log_audit(
        &state.db,
        Some(&auth.user_id),
        "user.create",
        Some("user"),
        Some(&user.id),
        None,
        None,
    )
    .await;

    Ok((StatusCode::CREATED, Json(UserInfo::from(user))))
}

pub async fn get_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<UserInfo>, StatusCode> {
    // Users can view themselves, admins can view anyone
    if auth.user_id != id && !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    let user = manager_core::db::users::get_user_by_id(&state.db, &id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(UserInfo::from(user)))
}

pub async fn update_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<UserInfo>, StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    let target = manager_core::db::users::get_user_by_id(&state.db, &id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Non-super_admin cannot modify super_admin users
    if target.role == UserRole::SuperAdmin && auth.role != UserRole::SuperAdmin {
        return Err(StatusCode::FORBIDDEN);
    }

    let user = manager_core::db::users::update_user(&state.db, &id, &req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = manager_core::db::audit::log_audit(
        &state.db,
        Some(&auth.user_id),
        "user.update",
        Some("user"),
        Some(&id),
        None,
        None,
    )
    .await;

    Ok(Json(UserInfo::from(user)))
}

pub async fn delete_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Cannot delete yourself
    if auth.user_id == id {
        return Err(StatusCode::BAD_REQUEST);
    }

    let target = manager_core::db::users::get_user_by_id(&state.db, &id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Non-super_admin cannot delete super_admin
    if target.role == UserRole::SuperAdmin && auth.role != UserRole::SuperAdmin {
        return Err(StatusCode::FORBIDDEN);
    }

    manager_core::db::users::delete_user(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = manager_core::db::audit::log_audit(
        &state.db,
        Some(&auth.user_id),
        "user.delete",
        Some("user"),
        Some(&id),
        None,
        None,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}
