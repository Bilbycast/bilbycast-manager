-- Revoked session tokens (for logout invalidation).
-- Entries are cleaned up after their original JWT expiry passes.
CREATE TABLE IF NOT EXISTS revoked_sessions (
    jti TEXT PRIMARY KEY NOT NULL,
    revoked_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_revoked_sessions_expires ON revoked_sessions(expires_at);
