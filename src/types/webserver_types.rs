use serde::{Deserialize, Serialize};

/// used by webserver::json_response()
#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,

    #[serde(skip)]
    pub http_status: u16,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn success(data: T) -> Self {
        Self::success_with_status(data, 200)
    }

    pub fn success_with_status(data: T, status_code: u16) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
            http_status: status_code,
        }
    }

    pub fn error(message: String) -> Self {
        Self::error_with_status(message, 500)
    }

    pub fn error_with_status(message: String, status_code: u16) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message),
            http_status: status_code,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| "{\"success\":false,\"error\":\"Serialization error\"}".to_string())
    }
}
