use std::time::Duration;
use tokio;

pub async fn start_ac_controller() {
    loop {
        println!("NIY: AC Controller is running...");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
