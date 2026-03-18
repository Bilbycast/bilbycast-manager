-- bilbycast-manager initial database schema

-- Users
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    display_name TEXT NOT NULL,
    email TEXT,
    role TEXT NOT NULL CHECK (role IN ('super_admin', 'admin', 'operator', 'viewer')),
    is_temporary BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at TEXT,
    allowed_node_ids TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_login_at TEXT
);

-- User sessions
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    ip_address TEXT,
    user_agent TEXT
);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);

-- Edge nodes
CREATE TABLE IF NOT EXISTS nodes (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    api_url TEXT,
    registration_token TEXT UNIQUE,
    auth_client_id TEXT,
    auth_client_secret_enc TEXT,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'online', 'offline', 'degraded', 'error')),
    last_seen_at TEXT,
    last_health TEXT,
    software_version TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Node WebSocket connections
CREATE TABLE IF NOT EXISTS node_connections (
    node_id TEXT PRIMARY KEY NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    connected_at TEXT NOT NULL,
    remote_addr TEXT,
    ws_session_id TEXT NOT NULL
);

-- Events & Alarms
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    severity TEXT NOT NULL CHECK (severity IN ('critical', 'warning', 'info')),
    category TEXT NOT NULL,
    message TEXT NOT NULL,
    details TEXT,
    flow_id TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    acknowledged BOOLEAN NOT NULL DEFAULT FALSE,
    acknowledged_by TEXT REFERENCES users(id)
);
CREATE INDEX IF NOT EXISTS idx_events_node_time ON events(node_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_events_severity ON events(severity, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_events_category ON events(category);

-- Audit log
CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT REFERENCES users(id),
    action TEXT NOT NULL,
    target_type TEXT,
    target_id TEXT,
    details TEXT,
    ip_address TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_audit_time ON audit_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_log(user_id);

-- System settings (key-value store)
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_by TEXT REFERENCES users(id)
);

-- AI provider API keys (encrypted)
CREATE TABLE IF NOT EXISTS ai_keys (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('openai', 'anthropic', 'gemini')),
    api_key_enc TEXT NOT NULL,
    model_preference TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(user_id, provider)
);

-- Configuration templates/presets
CREATE TABLE IF NOT EXISTS config_templates (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    category TEXT,
    template TEXT NOT NULL,
    created_by TEXT REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Insert default settings
INSERT OR IGNORE INTO settings (key, value) VALUES ('events_retention_days', '30');
INSERT OR IGNORE INTO settings (key, value) VALUES ('ws_keepalive_interval_secs', '15');
INSERT OR IGNORE INTO settings (key, value) VALUES ('session_lifetime_hours', '24');
INSERT OR IGNORE INTO settings (key, value) VALUES ('max_login_attempts', '5');
INSERT OR IGNORE INTO settings (key, value) VALUES ('node_offline_threshold_secs', '30');
INSERT OR IGNORE INTO settings (key, value) VALUES ('stats_broadcast_interval_ms', '1000');
