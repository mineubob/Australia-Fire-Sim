use crate::element::Vec3;
use serde::{Deserialize, Serialize};

/// Weather system with McArthur Forest Fire Danger Index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherSystem {
    pub temperature: f32,      // °C
    pub humidity: f32,         // %
    pub wind_speed: f32,       // km/h
    pub wind_direction: f32,   // degrees (0 = North, 90 = East)
    pub drought_factor: f32,   // 0-10 (Keetch-Byram Drought Index scaled)
}

impl WeatherSystem {
    /// Create a new weather system
    pub fn new(temperature: f32, humidity: f32, wind_speed: f32, wind_direction: f32, drought_factor: f32) -> Self {
        WeatherSystem {
            temperature,
            humidity,
            wind_speed,
            wind_direction,
            drought_factor,
        }
    }
    
    /// Create default weather (moderate conditions)
    pub fn default() -> Self {
        WeatherSystem {
            temperature: 25.0,
            humidity: 50.0,
            wind_speed: 15.0,
            wind_direction: 0.0,
            drought_factor: 5.0,
        }
    }
    
    /// Create extreme weather (catastrophic conditions)
    pub fn catastrophic() -> Self {
        WeatherSystem {
            temperature: 45.0,
            humidity: 10.0,
            wind_speed: 60.0,
            wind_direction: 0.0,
            drought_factor: 10.0,
        }
    }
    
    /// Calculate McArthur Forest Fire Danger Index
    pub fn calculate_ffdi(&self) -> f32 {
        // McArthur FFDI formula
        let ffdi = self.drought_factor 
                   * (self.temperature / 30.0) 
                   * ((100.0 - self.humidity) / 80.0) 
                   * (self.wind_speed / 20.0) 
                   * 10.0;
        ffdi.max(0.0)
    }
    
    /// Get fire danger rating string
    pub fn fire_danger_rating(&self) -> &str {
        match self.calculate_ffdi() {
            f if f < 5.0 => "Low",
            f if f < 12.0 => "Moderate",
            f if f < 24.0 => "High",
            f if f < 50.0 => "Very High",
            f if f < 75.0 => "Severe",
            f if f < 100.0 => "Extreme",
            _ => "CATASTROPHIC", // Code Red
        }
    }
    
    /// Get spread rate multiplier based on FFDI
    pub fn spread_rate_multiplier(&self) -> f32 {
        // FFDI directly scales spread rate
        (self.calculate_ffdi() / 10.0).max(1.0)
    }
    
    /// Get wind vector in m/s
    pub fn wind_vector(&self) -> Vec3 {
        let wind_ms = self.wind_speed / 3.6; // Convert km/h to m/s
        let angle_rad = self.wind_direction.to_radians();
        
        Vec3::new(
            angle_rad.sin() * wind_ms,
            angle_rad.cos() * wind_ms,
            0.0,
        )
    }
    
    /// Get wind speed in m/s
    pub fn wind_speed_ms(&self) -> f32 {
        self.wind_speed / 3.6
    }
    
    /// Update weather (for dynamic simulations)
    pub fn update(&mut self, _dt: f32) {
        // Placeholder for dynamic weather changes
        // Could add time-of-day effects, weather fronts, etc.
    }
    
    /// Calculate fuel moisture based on weather
    pub fn calculate_fuel_moisture(&self, base_moisture: f32) -> f32 {
        // Simplified fuel moisture calculation
        // Higher humidity increases moisture, higher temperature decreases it
        let humidity_factor = self.humidity / 100.0;
        let temp_factor = (30.0 / self.temperature.max(10.0)).min(2.0);
        
        (base_moisture * humidity_factor * temp_factor).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ffdi_calculation() {
        let weather = WeatherSystem::new(30.0, 20.0, 40.0, 0.0, 8.0);
        let ffdi = weather.calculate_ffdi();
        
        // FFDI should be calculated correctly
        // Expected: 8.0 * (30.0/30.0) * (80.0/80.0) * (40.0/20.0) * 10.0 = 160
        assert!((ffdi - 160.0).abs() < 1.0);
    }
    
    #[test]
    fn test_fire_danger_ratings() {
        let low = WeatherSystem::new(15.0, 80.0, 5.0, 0.0, 2.0);
        assert_eq!(low.fire_danger_rating(), "Low");
        
        let catastrophic = WeatherSystem::catastrophic();
        assert_eq!(catastrophic.fire_danger_rating(), "CATASTROPHIC");
    }
    
    #[test]
    fn test_wind_vector() {
        let weather = WeatherSystem::new(25.0, 50.0, 36.0, 90.0, 5.0); // 36 km/h = 10 m/s, East
        let wind = weather.wind_vector();
        
        // East direction (90°) should be primarily +X
        assert!((wind.x - 10.0).abs() < 0.1);
        assert!(wind.y.abs() < 0.1);
    }
    
    #[test]
    fn test_ffdi_scaling() {
        let weather1 = WeatherSystem::new(25.0, 50.0, 20.0, 0.0, 5.0);
        let weather2 = WeatherSystem::new(35.0, 30.0, 40.0, 0.0, 7.0);
        
        let ffdi1 = weather1.calculate_ffdi();
        let ffdi2 = weather2.calculate_ffdi();
        
        // Higher values should give higher FFDI
        assert!(ffdi2 > ffdi1);
    }
}
