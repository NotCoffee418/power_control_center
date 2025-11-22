-- Add cause_id column to ac_actions table
-- 0 = Undefined (default for existing records)
-- 1 = Ice Exception
-- Future cause IDs can be added as needed

ALTER TABLE ac_actions ADD COLUMN cause_id INTEGER NOT NULL DEFAULT 0;
