-- Add is_editable column to cause_reasons table
-- System cause reasons (existing ones) are not editable
-- New cause reasons default to editable

ALTER TABLE cause_reasons ADD COLUMN is_editable BOOLEAN NOT NULL DEFAULT 1;

-- Set all existing cause reasons to non-editable (system cause reasons)
UPDATE cause_reasons SET is_editable = 0 WHERE id <= 7;
