-- Add device_type column to nodes table for driver-based architecture.
-- All existing nodes default to 'edge' (bilbycast-edge transport nodes).
ALTER TABLE nodes ADD COLUMN device_type TEXT NOT NULL DEFAULT 'edge';
