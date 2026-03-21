use sqlx::SqlitePool;

/// Revoke a session by its JWT ID (jti).
pub async fn revoke_session(
    pool: &SqlitePool,
    jti: &str,
    expires_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR IGNORE INTO revoked_sessions (jti, expires_at) VALUES (?, ?)")
        .bind(jti)
        .bind(expires_at)
        .execute(pool)
        .await?;
    Ok(())
}

/// Check if a session has been revoked.
pub async fn is_session_revoked(pool: &SqlitePool, jti: &str) -> Result<bool, sqlx::Error> {
    let row: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM revoked_sessions WHERE jti = ?")
            .bind(jti)
            .fetch_optional(pool)
            .await?;
    Ok(row.is_some())
}

/// Clean up expired revoked sessions.
pub async fn cleanup_expired_sessions(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM revoked_sessions WHERE expires_at < datetime('now')")
            .execute(pool)
            .await?;
    Ok(result.rows_affected())
}
