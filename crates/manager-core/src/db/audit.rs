// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use chrono::Utc;
use sqlx::SqlitePool;

use crate::models::AuditEntry;

/// Log an audit entry.
pub async fn log_audit(
    pool: &SqlitePool,
    user_id: Option<&str>,
    action: &str,
    target_type: Option<&str>,
    target_id: Option<&str>,
    details: Option<&serde_json::Value>,
    ip_address: Option<&str>,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let details_json = details.map(|d| serde_json::to_string(d).unwrap_or_default());

    sqlx::query(
        r#"
        INSERT INTO audit_log (user_id, action, target_type, target_id, details, ip_address, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(&details_json)
    .bind(ip_address)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Query audit log entries.
pub async fn query_audit_log(
    pool: &SqlitePool,
    limit: u32,
    offset: u32,
) -> Result<Vec<AuditEntry>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AuditRow>(
        "SELECT * FROM audit_log ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into_entry()).collect())
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: i64,
    user_id: Option<String>,
    action: String,
    target_type: Option<String>,
    target_id: Option<String>,
    details: Option<String>,
    ip_address: Option<String>,
    created_at: String,
}

impl AuditRow {
    fn into_entry(self) -> AuditEntry {
        AuditEntry {
            id: self.id,
            user_id: self.user_id,
            action: self.action,
            target_type: self.target_type,
            target_id: self.target_id,
            details: self.details.and_then(|s| serde_json::from_str(&s).ok()),
            ip_address: self.ip_address,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_default(),
        }
    }
}
