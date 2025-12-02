use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::OnceCell;

// Internal response types (not exposed)
#[derive(Debug, Deserialize)]
pub(super) struct ApiResponse<T> {
    pub(super) success: bool,
    pub(super) error: String,
    pub(super) data: Option<T>,
}

static CLIENT: OnceCell<Client> = OnceCell::const_new();

pub(super) async fn get_client() -> &'static Client {
    CLIENT
        .get_or_init(|| async {
            Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap()
        })
        .await
}
