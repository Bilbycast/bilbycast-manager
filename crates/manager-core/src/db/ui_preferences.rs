// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use sqlx::SqlitePool;

/// Fetch a UI preference value for a user by key.
pub async fn get_preference(
    pool: &SqlitePool,
    user_id: &str,
    pref_key: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT pref_value FROM ui_preferences WHERE user_id = ? AND pref_key = ?",
    )
    .bind(user_id)
    .bind(pref_key)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0))
}

/// Set a UI preference value for a user (upsert).
pub async fn set_preference(
    pool: &SqlitePool,
    user_id: &str,
    pref_key: &str,
    pref_value: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO ui_preferences (user_id, pref_key, pref_value, updated_at) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(user_id, pref_key) DO UPDATE SET pref_value = excluded.pref_value, updated_at = excluded.updated_at",
    )
    .bind(user_id)
    .bind(pref_key)
    .bind(pref_value)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete a UI preference for a user.
pub async fn delete_preference(
    pool: &SqlitePool,
    user_id: &str,
    pref_key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM ui_preferences WHERE user_id = ? AND pref_key = ?")
        .bind(user_id)
        .bind(pref_key)
        .execute(pool)
        .await?;
    Ok(())
}
