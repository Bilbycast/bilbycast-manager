-- Add relay_node_id column to tunnels table.
-- For relay-mode tunnels, this references the managed relay node used.
ALTER TABLE tunnels ADD COLUMN relay_node_id TEXT;
