-- Create nodesets table for storing multiple node logic configurations
CREATE TABLE nodesets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR NOT NULL,
    node_json VARCHAR NOT NULL
);

-- Move existing node_configuration from settings to nodesets with id 0 (default)
INSERT INTO nodesets (id, name, node_json) 
SELECT 0, 'Default', setting_value 
FROM settings 
WHERE setting_key = 'node_configuration';

-- If no node_configuration exists, create a default empty nodeset
INSERT OR IGNORE INTO nodesets (id, name, node_json) 
VALUES (0, 'Default', '{"nodes": [], "edges": []}');

-- Remove the old node_configuration from settings
DELETE FROM settings WHERE setting_key = 'node_configuration';

-- Add active_nodeset setting (default to 0)
INSERT OR REPLACE INTO settings (setting_key, setting_value) 
VALUES ('active_nodeset', '0');
