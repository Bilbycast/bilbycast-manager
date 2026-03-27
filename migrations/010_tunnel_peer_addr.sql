-- Add egress_peer_addr for direct mode tunnels (reachable address of egress node's QUIC listener)
ALTER TABLE tunnels ADD COLUMN egress_peer_addr TEXT;
