use serde::Serialize;
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow, Serialize)]
pub struct AcAction {
    pub id: i32,
    pub action_timestamp: i32, // Unix timestamp
    pub device_identifier: String,
    pub action_type: String, // on, off, toggle-powerful
    pub mode: Option<i32>,
    pub fan_speed: Option<i32>,
    pub request_temperature: Option<f32>,
    pub swing: Option<i32>,
}
