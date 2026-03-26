-- Per-user UI preferences (e.g., flow table row order, view selection).
-- Cascade-deletes when the referenced user is removed.
CREATE TABLE IF NOT EXISTS ui_preferences (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    pref_key   TEXT NOT NULL,
    pref_value TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (user_id, pref_key)
);
