-- Per-leg push status tracking for tunnels.
-- Tracks whether each leg (ingress edge, egress edge, relay) has been
-- successfully configured, enabling retry and visibility.

ALTER TABLE tunnels ADD COLUMN ingress_push_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE tunnels ADD COLUMN egress_push_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE tunnels ADD COLUMN relay_push_status TEXT;
ALTER TABLE tunnels ADD COLUMN ingress_push_error TEXT;
ALTER TABLE tunnels ADD COLUMN egress_push_error TEXT;
ALTER TABLE tunnels ADD COLUMN relay_push_error TEXT;
