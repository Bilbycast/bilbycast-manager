// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

use sqlx::SqlitePool;

/// Fetch all saved topology positions for a user and view.
pub async fn get_positions(
    pool: &SqlitePool,
    user_id: &str,
    view: &str,
) -> Result<Vec<(String, f64, f64)>, sqlx::Error> {
    let rows: Vec<(String, f64, f64)> = sqlx::query_as(
        "SELECT node_id, x, y FROM topology_positions WHERE user_id = ? AND view = ?",
    )
    .bind(user_id)
    .bind(view)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Save topology positions for a user and view (batch upsert).
/// Replaces all positions for the given user+view with the new set.
pub async fn save_positions(
    pool: &SqlitePool,
    user_id: &str,
    view: &str,
    positions: &[(String, f64, f64)],
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();

    // Delete existing positions for this user+view, then insert new ones.
    // Use a transaction for atomicity.
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM topology_positions WHERE user_id = ? AND view = ?")
        .bind(user_id)
        .bind(view)
        .execute(&mut *tx)
        .await?;

    for (node_id, x, y) in positions {
        sqlx::query(
            "INSERT INTO topology_positions (user_id, node_id, view, x, y, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(node_id)
        .bind(view)
        .bind(x)
        .bind(y)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Clear all saved positions for a user and view (for "Reset Layout").
pub async fn clear_positions(
    pool: &SqlitePool,
    user_id: &str,
    view: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM topology_positions WHERE user_id = ? AND view = ?")
        .bind(user_id)
        .bind(view)
        .execute(pool)
        .await?;
    Ok(())
}
