-- IP Tunnels for NAT traversal between edge nodes
CREATE TABLE IF NOT EXISTS tunnels (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    -- Transport protocol tunneled: 'tcp' or 'udp'
    protocol TEXT NOT NULL CHECK (protocol IN ('tcp', 'udp')),
    -- Connection mode: 'relay' (via bilbycast-relay) or 'direct' (edge-to-edge)
    mode TEXT NOT NULL CHECK (mode IN ('relay', 'direct')),
    -- Ingress side: local device -> this edge -> tunnel
    ingress_node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    ingress_listen_port INTEGER NOT NULL,
    -- Egress side: tunnel -> this edge -> local device
    egress_node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    egress_forward_addr TEXT NOT NULL,
    -- Relay server address (required for relay mode)
    relay_addr TEXT,
    -- Pre-shared key for tunnel auth (encrypted)
    tunnel_psk_enc TEXT,
    -- Status
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'active', 'error', 'disabled')),
    -- Optional: associated flow IDs (JSON array) for UI linking
    associated_flow_ids TEXT,
    -- Metadata
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_tunnels_ingress ON tunnels(ingress_node_id);
CREATE INDEX IF NOT EXISTS idx_tunnels_egress ON tunnels(egress_node_id);
CREATE INDEX IF NOT EXISTS idx_tunnels_status ON tunnels(status);

-- Add network_type to nodes so the UI can detect NAT
ALTER TABLE nodes ADD COLUMN network_type TEXT DEFAULT 'nat'
    CHECK (network_type IN ('nat', 'public', 'unknown'));
