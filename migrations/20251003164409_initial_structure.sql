-- Add migration script here
CREATE TABLE ac_actions (
    id SERIAL PRIMARY KEY,
    action_timestamp INTEGER NOT NULL,
    device_identifier VARCHAR NOT NULL,
    action_type VARCHAR NOT NULL,
    -- on, off, toggle-powerful
    mode INT,
    fan_speed INT,
    request_temperature FLOAT,
    swing INT,
    measured_temperature FLOAT,
    -- Temperature measured by the device at the time of request
    measured_net_power_watt INT,
    -- Measured power usage (negative for production) at the time of request
    is_human_home BOOLEAN
);