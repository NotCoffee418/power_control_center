-- Add initial node configuration to settings table
-- This will store the node-based logic configuration as JSON
INSERT OR IGNORE INTO settings (setting_key, setting_value) 
VALUES ('node_configuration', '{"nodes": [], "edges": []}');
