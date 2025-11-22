-- Add PIR Detection cause ID documentation
-- This migration doesn't alter the schema, but documents the new cause ID
-- 0 = Undefined (default for existing records)
-- 1 = Ice Exception (outdoor temp < 5°C)
-- 2 = PIR Detection (motion sensor triggered AC off)
-- Future cause IDs can be added as needed

-- No schema changes needed, just documentation of new cause_id value
