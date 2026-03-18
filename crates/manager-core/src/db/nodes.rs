use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{CreateNodeRequest, Node, NodeStatus, UpdateNodeRequest};

/// Create a new node with a registration token.
pub async fn create_node(pool: &SqlitePool, req: &CreateNodeRequest) -> Result<Node, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let registration_token = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO nodes (id, name, description, registration_token, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, 'pending', ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&registration_token)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    get_node_by_id(pool, &id).await
}

/// Get a node by ID.
pub async fn get_node_by_id(pool: &SqlitePool, id: &str) -> Result<Node, sqlx::Error> {
    let row = sqlx::query_as::<_, NodeRow>("SELECT * FROM nodes WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(row.into_node())
}

/// Get a node by registration token.
pub async fn get_node_by_token(
    pool: &SqlitePool,
    token: &str,
) -> Result<Option<Node>, sqlx::Error> {
    let row = sqlx::query_as::<_, NodeRow>(
        "SELECT * FROM nodes WHERE registration_token = ? AND status = 'pending'",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.into_node()))
}

/// Get a node by node_id for authentication after registration.
pub async fn get_node_by_node_id(
    pool: &SqlitePool,
    node_id: &str,
) -> Result<Option<Node>, sqlx::Error> {
    let row = sqlx::query_as::<_, NodeRow>("SELECT * FROM nodes WHERE id = ?")
        .bind(node_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.into_node()))
}

/// List all nodes.
pub async fn list_nodes(pool: &SqlitePool) -> Result<Vec<Node>, sqlx::Error> {
    let rows = sqlx::query_as::<_, NodeRow>("SELECT * FROM nodes ORDER BY name ASC")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.into_node()).collect())
}

/// Update node status.
pub async fn update_node_status(
    pool: &SqlitePool,
    id: &str,
    status: NodeStatus,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE nodes SET status = ?, last_seen_at = ?, updated_at = ? WHERE id = ?")
        .bind(status.as_str())
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Mark node as registered (consume token, store auth credentials).
pub async fn complete_registration(
    pool: &SqlitePool,
    id: &str,
    node_secret_enc: &str,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        UPDATE nodes SET registration_token = NULL, auth_client_secret_enc = ?,
                        status = 'online', last_seen_at = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(node_secret_enc)
    .bind(&now)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Update a node.
pub async fn update_node(
    pool: &SqlitePool,
    id: &str,
    req: &UpdateNodeRequest,
) -> Result<Node, sqlx::Error> {
    let existing = get_node_by_id(pool, id).await?;
    let now = Utc::now().to_rfc3339();

    let name = req.name.as_deref().unwrap_or(&existing.name);
    let description = req.description.as_deref().or(existing.description.as_deref());

    sqlx::query("UPDATE nodes SET name = ?, description = ?, updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(description)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

    get_node_by_id(pool, id).await
}

/// Update node health and last seen.
pub async fn update_node_health(
    pool: &SqlitePool,
    id: &str,
    health: &serde_json::Value,
    version: Option<&str>,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let health_json = serde_json::to_string(health).unwrap_or_default();

    sqlx::query(
        r#"
        UPDATE nodes SET last_health = ?, software_version = COALESCE(?, software_version),
                        last_seen_at = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&health_json)
    .bind(version)
    .bind(&now)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete a node.
pub async fn delete_node(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM nodes WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Generate a new registration token for an existing node.
pub async fn regenerate_token(pool: &SqlitePool, id: &str) -> Result<String, sqlx::Error> {
    let token = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE nodes SET registration_token = ?, status = 'pending', updated_at = ? WHERE id = ?",
    )
    .bind(&token)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(token)
}

/// Get the encrypted node secret.
pub async fn get_node_secret_enc(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT auth_client_secret_enc FROM nodes WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|r| r.0))
}

#[derive(sqlx::FromRow)]
struct NodeRow {
    id: String,
    name: String,
    description: Option<String>,
    registration_token: Option<String>,
    #[allow(dead_code)]
    auth_client_id: Option<String>,
    #[allow(dead_code)]
    auth_client_secret_enc: Option<String>,
    status: String,
    last_seen_at: Option<String>,
    last_health: Option<String>,
    software_version: Option<String>,
    metadata: Option<String>,
    created_at: String,
    updated_at: String,
}

impl NodeRow {
    fn into_node(self) -> Node {
        Node {
            id: self.id,
            name: self.name,
            description: self.description,
            registration_token: self.registration_token,
            status: NodeStatus::from_str(&self.status).unwrap_or(NodeStatus::Pending),
            last_seen_at: self
                .last_seen_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            last_health: self
                .last_health
                .and_then(|s| serde_json::from_str(&s).ok()),
            software_version: self.software_version,
            metadata: self.metadata.and_then(|s| serde_json::from_str(&s).ok()),
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_default(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_default(),
        }
    }
}
