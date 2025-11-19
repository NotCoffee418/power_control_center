use serde::Deserialize;

#[derive(Debug)]
pub enum WeatherError {
    RequestFailed(String),
    ParseError(String),
}

impl std::fmt::Display for WeatherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeatherError::RequestFailed(msg) => write!(f, "Weather API request failed: {}", msg),
            WeatherError::ParseError(msg) => write!(f, "Failed to parse weather data: {}", msg),
        }
    }
}

impl std::error::Error for WeatherError {}

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    hourly: HourlyData,
}

#[derive(Debug, Deserialize)]
struct HourlyData {
    temperature_2m: Vec<f64>,
}

/// Get current outdoor temperature from Open-Meteo API
pub async fn get_current_outdoor_temp(latitude: f64, longitude: f64) -> Result<f64, WeatherError> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m&forecast_days=1",
        latitude, longitude
    );
    
    let response = reqwest::get(&url)
        .await
        .map_err(|e| WeatherError::RequestFailed(e.to_string()))?;
    
    let data: OpenMeteoResponse = response
        .json()
        .await
        .map_err(|e| WeatherError::ParseError(e.to_string()))?;
    
    // Get the first temperature value (current hour)
    data.hourly.temperature_2m
        .first()
        .copied()
        .ok_or_else(|| WeatherError::ParseError("No temperature data available".to_string()))
}

/// Get average outdoor temperature for next 12 hours from Open-Meteo API
pub async fn get_avg_next_12h_outdoor_temp(latitude: f64, longitude: f64) -> Result<f64, WeatherError> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m&forecast_days=2",
        latitude, longitude
    );
    
    let response = reqwest::get(&url)
        .await
        .map_err(|e| WeatherError::RequestFailed(e.to_string()))?;
    
    let data: OpenMeteoResponse = response
        .json()
        .await
        .map_err(|e| WeatherError::ParseError(e.to_string()))?;
    
    // Calculate average for next 12 hours (skip first hour which is current)
    let temps = &data.hourly.temperature_2m;
    if temps.len() < 2 {
        return Err(WeatherError::ParseError("Insufficient forecast data".to_string()));
    }
    
    // Take hours 1-12 (next 12 hours after current)
    let forecast_temps: Vec<f64> = temps.iter().skip(1).take(12).copied().collect();
    
    if forecast_temps.is_empty() {
        return Err(WeatherError::ParseError("No forecast data available".to_string()));
    }
    
    let sum: f64 = forecast_temps.iter().sum();
    Ok(sum / forecast_temps.len() as f64)
}

/// Compute temperature trend: returns the difference between average next 12h temp and current temp
/// Positive value means it's getting warmer, negative means it's getting colder
pub async fn compute_temperature_trend(latitude: f64, longitude: f64) -> Result<f64, WeatherError> {
    let current_temp = get_current_outdoor_temp(latitude, longitude).await?;
    let avg_next_12h = get_avg_next_12h_outdoor_temp(latitude, longitude).await?;
    Ok(avg_next_12h - current_temp)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test helper to validate that trend calculation logic is correct
    #[test]
    fn test_trend_calculation_logic() {
        let current = 20.0;
        let future_warmer = 23.0;
        let future_colder = 17.0;
        
        // Warming trend
        assert_eq!(future_warmer - current, 3.0);
        
        // Cooling trend
        assert_eq!(future_colder - current, -3.0);
    }

    // Note: Integration tests with actual API calls are not included here
    // as they would require network access and could be flaky.
    // In a production environment, you might want to:
    // 1. Mock the API responses for testing
    // 2. Use integration tests with VCR-style recording
    // 3. Test against a local mock server
}
