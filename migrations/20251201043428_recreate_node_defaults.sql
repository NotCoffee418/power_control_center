-- Clear system cause_reasons to allow fresh initialization from code
-- System cause_reasons (IDs 0-99) are managed by db::defaults module
-- User-created cause_reasons start at ID 100 (enforced in code)
DELETE FROM cause_reasons WHERE id < 100;
