-- Add migration script here
CREATE TABLE settings (
    setting_key VARCHAR NOT NULL UNIQUE,
    setting_value VARCHAR NOT NULL
)