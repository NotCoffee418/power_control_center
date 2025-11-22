-- Add migration script here
-- Add fields to support comprehensive AC command logging
ALTER TABLE ac_actions ADD COLUMN measured_temperature FLOAT;
ALTER TABLE ac_actions ADD COLUMN measured_net_power_watt INT;
ALTER TABLE ac_actions ADD COLUMN is_human_home BOOLEAN;
