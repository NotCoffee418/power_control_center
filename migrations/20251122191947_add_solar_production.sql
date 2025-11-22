-- Add measured_solar_production_watt column to ac_actions table
-- Stores the raw solar production (in watts) at the time of AC action decision/execution

ALTER TABLE ac_actions ADD COLUMN measured_solar_production_watt INTEGER;
