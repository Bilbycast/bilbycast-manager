-- Add optional expiry time to nodes
ALTER TABLE nodes ADD COLUMN expires_at TEXT;
