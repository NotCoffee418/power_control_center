use serde::{Deserialize, Serialize};

pub type ApiError = ApiResponse<()>;

/// used by webserver::json_response()
#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}
