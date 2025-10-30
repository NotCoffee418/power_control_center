mod plan_helpers;
mod plan_types;

use crate::device_requests;
use std::time::Duration;
use tokio;

/// ----------------- DEBUG OLD VER TEST
/// Start the loop of controlling ACs
pub async fn start_ac_controller() {
    return; // Do nothing yet
    
    // Example usage of fetch_data_and_get_plan function:
    // use plan_types::{AcDevices, fetch_data_and_get_plan};
    // let living_room_plan = fetch_data_and_get_plan(&AcDevices::LivingRoom).await;
    // let veranda_plan = fetch_data_and_get_plan(&AcDevices::Veranda).await;
    // 
    // Or use get_plan directly with known data for testing:
    // use plan_types::get_plan;
    // let plan = get_plan(22.0, 1500, true);
    // 
    // Then use the RequestMode returned to decide what API calls to make
    
    // Read thing from thing debug
    match device_requests::meter::get_solar_production().await {
        Ok(data) => {
            println!("Solar production data: {:?}", data.current_production);
        }
        Err(e) => println!("Error fetching solar production data: {}", e),
    }

    // Read other thing
    match device_requests::meter::get_latest_reading().await {
        Ok(data) => {
            println!("Latest meter reading: {:?}", data);
        }
        Err(e) => println!("Error fetching latest meter reading: {}", e),
    }

    // // Debug fetch sensors
    match device_requests::ac::get_sensors("LivingRoom").await {
        Ok(data) => {
            println!("LivingRoom Sensor data: {:?}", data);
        }
        Err(e) => println!("Error fetching sensor data: {}", e),
    }
    println!("Done debg thing. wait 5s then turn on ac.");

    match device_requests::ac::get_sensors("Veranda").await {
        Ok(data) => {
            println!("Veranda Sensor data: {:?}", data);
        }
        Err(e) => println!("Error fetching sensor data: {}", e),
    }
    println!("Done debg thing. wait 5s then turn on ac.");

    // Debug turn on AC
    match device_requests::ac::turn_on_ac("LivingRoom", 4, 0, 21.0, 1).await {
        Ok(_) => {
            println!("AC turned on successfully");
        }
        Err(e) => println!("Error turning on AC: {}", e),
    }
    println!("Done debug thing. AC controller is running...");

    match device_requests::ac::turn_on_ac("Veranda", 4, 0, 19.0, 0).await {
        Ok(_) => {
            println!("AC turned on successfully");
        }
        Err(e) => println!("Error turning on AC: {}", e),
    }
    println!("Done debug thing. AC controller is running...");

    // match device_requests::ac::turn_off_ac("LivingRoom").await {
    //     Ok(_) => {
    //         println!("AC turned off successfully");
    //     }
    //     Err(e) => println!("Error turning off AC: {}", e),
    // }

    loop {
        println!("NIY: AC Controller is running...");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
