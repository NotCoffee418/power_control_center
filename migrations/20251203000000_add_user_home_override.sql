-- Add user_is_home_override setting to allow manual override of is_user_home logic
-- Value is a unix timestamp (0 means no override, >0 means user is home until that time)
INSERT OR IGNORE INTO settings (setting_key, setting_value) VALUES ('user_is_home_override', '0');
