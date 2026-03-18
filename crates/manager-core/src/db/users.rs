use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::auth::hash_password;
use crate::models::{CreateUserRequest, UpdateUserRequest, User, UserRole};

/// Create a new user in the database.
pub async fn create_user(pool: &SqlitePool, req: &CreateUserRequest) -> Result<User, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let password_hash =
        hash_password(&req.password).map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
    let now = Utc::now().to_rfc3339();
    let role_str = req.role.as_str();
    let expires_at = req.expires_at.map(|dt| dt.to_rfc3339());
    let allowed_node_ids = req
        .allowed_node_ids
        .as_ref()
        .map(|ids| serde_json::to_string(ids).unwrap_or_default());

    sqlx::query(
        r#"
        INSERT INTO users (id, username, password_hash, display_name, email, role,
                          is_temporary, expires_at, allowed_node_ids, is_active, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, TRUE, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.username)
    .bind(&password_hash)
    .bind(&req.display_name)
    .bind(&req.email)
    .bind(role_str)
    .bind(req.is_temporary)
    .bind(&expires_at)
    .bind(&allowed_node_ids)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    get_user_by_id(pool, &id).await
}

/// Get a user by ID.
pub async fn get_user_by_id(pool: &SqlitePool, id: &str) -> Result<User, sqlx::Error> {
    let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(row.into_user())
}

/// Get a user by username.
pub async fn get_user_by_username(
    pool: &SqlitePool,
    username: &str,
) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.into_user()))
}

/// List all users.
pub async fn list_users(pool: &SqlitePool) -> Result<Vec<User>, sqlx::Error> {
    let rows = sqlx::query_as::<_, UserRow>("SELECT * FROM users ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.into_user()).collect())
}

/// Update a user.
pub async fn update_user(
    pool: &SqlitePool,
    id: &str,
    req: &UpdateUserRequest,
) -> Result<User, sqlx::Error> {
    let existing = get_user_by_id(pool, id).await?;
    let now = Utc::now().to_rfc3339();

    let display_name = req.display_name.as_deref().unwrap_or(&existing.display_name);
    let email = req.email.as_deref().or(existing.email.as_deref());
    let role = req.role.unwrap_or(existing.role);
    let is_temporary = req.is_temporary.unwrap_or(existing.is_temporary);
    let expires_at = match &req.expires_at {
        Some(opt) => opt.map(|dt| dt.to_rfc3339()),
        None => existing.expires_at.map(|dt| dt.to_rfc3339()),
    };
    let allowed_node_ids = match &req.allowed_node_ids {
        Some(opt) => opt
            .as_ref()
            .map(|ids| serde_json::to_string(ids).unwrap_or_default()),
        None => existing
            .allowed_node_ids
            .as_ref()
            .map(|ids| serde_json::to_string(ids).unwrap_or_default()),
    };
    let is_active = req.is_active.unwrap_or(existing.is_active);

    let password_hash = if let Some(ref pwd) = req.password {
        hash_password(pwd).map_err(|e| sqlx::Error::Protocol(e.to_string()))?
    } else {
        existing.password_hash.clone()
    };

    sqlx::query(
        r#"
        UPDATE users SET display_name = ?, email = ?, role = ?, is_temporary = ?,
                        expires_at = ?, allowed_node_ids = ?, is_active = ?,
                        password_hash = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(display_name)
    .bind(email)
    .bind(role.as_str())
    .bind(is_temporary)
    .bind(&expires_at)
    .bind(&allowed_node_ids)
    .bind(is_active)
    .bind(&password_hash)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get_user_by_id(pool, id).await
}

/// Delete a user.
pub async fn delete_user(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update last login timestamp.
pub async fn update_last_login(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Count users (for checking if setup is needed).
pub async fn count_users(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

// Internal row type for sqlx mapping
#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    username: String,
    password_hash: String,
    display_name: String,
    email: Option<String>,
    role: String,
    is_temporary: bool,
    expires_at: Option<String>,
    allowed_node_ids: Option<String>,
    is_active: bool,
    created_at: String,
    updated_at: String,
    last_login_at: Option<String>,
}

impl UserRow {
    fn into_user(self) -> User {
        User {
            id: self.id,
            username: self.username,
            password_hash: self.password_hash,
            display_name: self.display_name,
            email: self.email,
            role: UserRole::from_str(&self.role).unwrap_or(UserRole::Viewer),
            is_temporary: self.is_temporary,
            expires_at: self
                .expires_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            allowed_node_ids: self
                .allowed_node_ids
                .and_then(|s| serde_json::from_str(&s).ok()),
            is_active: self.is_active,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_default(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_default(),
            last_login_at: self
                .last_login_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
        }
    }
}
