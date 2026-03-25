-- Topology node positions per user per view (graph/flow).
-- Cascade-deletes when the referenced user or node is removed.
CREATE TABLE IF NOT EXISTS topology_positions (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    node_id    TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    view       TEXT NOT NULL DEFAULT 'graph',
    x          REAL NOT NULL,
    y          REAL NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (user_id, node_id, view)
);
