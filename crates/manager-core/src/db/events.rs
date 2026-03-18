use chrono::Utc;
use sqlx::SqlitePool;

use crate::models::{Event, EventQuery, EventSeverity};

/// Insert a new event.
pub async fn insert_event(
    pool: &SqlitePool,
    node_id: &str,
    severity: EventSeverity,
    category: &str,
    message: &str,
    details: Option<&serde_json::Value>,
    flow_id: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let details_json = details.map(|d| serde_json::to_string(d).unwrap_or_default());
    let severity_str = severity.as_str();

    let result = sqlx::query(
        r#"
        INSERT INTO events (node_id, severity, category, message, details, flow_id, created_at, acknowledged)
        VALUES (?, ?, ?, ?, ?, ?, ?, FALSE)
        "#,
    )
    .bind(node_id)
    .bind(severity_str)
    .bind(category)
    .bind(message)
    .bind(details_json.as_deref())
    .bind(flow_id)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Query events with filters. Uses a fixed query with optional params.
pub async fn query_events(
    pool: &SqlitePool,
    query: &EventQuery,
) -> Result<Vec<Event>, sqlx::Error> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(200);
    let offset = ((page - 1) * per_page) as i64;
    let limit = per_page as i64;

    let search_pattern = query.search.as_ref().map(|s| format!("%{s}%"));
    let from_str = query.from.map(|dt| dt.to_rfc3339());
    let to_str = query.to.map(|dt| dt.to_rfc3339());

    // Use a single query with COALESCE/NULL checks for optional filters
    let rows = sqlx::query_as::<_, EventRow>(
        r#"
        SELECT * FROM events
        WHERE (? IS NULL OR node_id = ?)
          AND (? IS NULL OR severity = ?)
          AND (? IS NULL OR category = ?)
          AND (? IS NULL OR flow_id = ?)
          AND (? IS NULL OR message LIKE ?)
          AND (? IS NULL OR created_at >= ?)
          AND (? IS NULL OR created_at <= ?)
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(query.node_id.as_deref())
    .bind(query.node_id.as_deref())
    .bind(query.severity.as_deref())
    .bind(query.severity.as_deref())
    .bind(query.category.as_deref())
    .bind(query.category.as_deref())
    .bind(query.flow_id.as_deref())
    .bind(query.flow_id.as_deref())
    .bind(search_pattern.as_deref())
    .bind(search_pattern.as_deref())
    .bind(from_str.as_deref())
    .bind(from_str.as_deref())
    .bind(to_str.as_deref())
    .bind(to_str.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into_event()).collect())
}

/// Acknowledge an event.
pub async fn acknowledge_event(
    pool: &SqlitePool,
    event_id: i64,
    user_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE events SET acknowledged = TRUE, acknowledged_by = ? WHERE id = ?")
        .bind(user_id)
        .bind(event_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Count unacknowledged events (for alarm badge).
pub async fn count_unacknowledged(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM events WHERE acknowledged = FALSE AND severity IN ('critical', 'warning')",
    )
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Delete events older than retention period.
pub async fn cleanup_old_events(
    pool: &SqlitePool,
    retention_days: u32,
) -> Result<u64, sqlx::Error> {
    let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
    let cutoff_str = cutoff.to_rfc3339();

    let result = sqlx::query("DELETE FROM events WHERE created_at < ?")
        .bind(&cutoff_str)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: i64,
    node_id: String,
    severity: String,
    category: String,
    message: String,
    details: Option<String>,
    flow_id: Option<String>,
    created_at: String,
    acknowledged: bool,
    acknowledged_by: Option<String>,
}

impl EventRow {
    fn into_event(self) -> Event {
        Event {
            id: self.id,
            node_id: self.node_id,
            severity: EventSeverity::from_str(&self.severity).unwrap_or(EventSeverity::Info),
            category: self.category,
            message: self.message,
            details: self.details.and_then(|s| serde_json::from_str(&s).ok()),
            flow_id: self.flow_id,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_default(),
            acknowledged: self.acknowledged,
            acknowledged_by: self.acknowledged_by,
        }
    }
}
