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
    
    // Dynamic weather state
    time_of_day: f32,          // Hours since midnight (0-24)
    day_of_year: u16,          // Day number (1-365)
    weather_front_progress: f32, // 0-1 for weather front passage
    target_temperature: f32,   // Target for smooth transitions
    target_humidity: f32,      // Target for smooth transitions
    target_wind_speed: f32,    // Target for smooth transitions
    target_wind_direction: f32, // Target for smooth transitions
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
            time_of_day: 12.0,
            day_of_year: 180,
            weather_front_progress: 0.0,
            target_temperature: temperature,
            target_humidity: humidity,
            target_wind_speed: wind_speed,
            target_wind_direction: wind_direction,
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
            time_of_day: 12.0,
            day_of_year: 180,
            weather_front_progress: 0.0,
            target_temperature: 25.0,
            target_humidity: 50.0,
            target_wind_speed: 15.0,
            target_wind_direction: 0.0,
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
            time_of_day: 14.0,
            day_of_year: 15,
            weather_front_progress: 0.0,
            target_temperature: 45.0,
            target_humidity: 10.0,
            target_wind_speed: 60.0,
            target_wind_direction: 0.0,
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
    pub fn update(&mut self, dt: f32) {
        // Update time of day
        self.time_of_day += dt / 3600.0; // dt is in seconds
        if self.time_of_day >= 24.0 {
            self.time_of_day -= 24.0;
            self.day_of_year += 1;
            if self.day_of_year > 365 {
                self.day_of_year = 1;
            }
        }
        
        // Diurnal (daily) temperature cycle
        // Temperature peaks around 14:00 (2 PM) and minimum around 5:00 AM
        let hour_offset = (self.time_of_day - 14.0) * std::f32::consts::PI / 12.0;
        let diurnal_variation = -8.0 * hour_offset.cos(); // ±8°C variation
        
        // Smooth transition toward target temperature with diurnal cycle
        let target_with_diurnal = self.target_temperature + diurnal_variation;
        let temp_diff = target_with_diurnal - self.temperature;
        self.temperature += temp_diff * (dt / 3600.0).min(0.1); // Smooth transition
        
        // Humidity varies inversely with temperature (simplified)
        // Higher temp = lower humidity during the day
        let humidity_variation = 15.0 * hour_offset.cos(); // ±15% variation
        let target_with_variation = self.target_humidity + humidity_variation;
        let humidity_diff = target_with_variation - self.humidity;
        self.humidity = (self.humidity + humidity_diff * (dt / 1800.0).min(0.1)).clamp(5.0, 95.0);
        
        // Wind speed variations (wind typically picks up during day, calms at night)
        let wind_hour_offset = (self.time_of_day - 15.0) * std::f32::consts::PI / 12.0;
        let wind_variation = 5.0 * wind_hour_offset.cos(); // ±5 km/h variation
        let target_wind_with_variation = self.target_wind_speed - wind_variation;
        let wind_diff = target_wind_with_variation - self.wind_speed;
        self.wind_speed = (self.wind_speed + wind_diff * (dt / 1800.0).min(0.1)).max(0.0);
        
        // Wind direction shifts gradually
        let dir_diff = self.target_wind_direction - self.wind_direction;
        // Handle wraparound (e.g., 350° to 10°)
        let dir_diff = if dir_diff > 180.0 {
            dir_diff - 360.0
        } else if dir_diff < -180.0 {
            dir_diff + 360.0
        } else {
            dir_diff
        };
        self.wind_direction += dir_diff * (dt / 3600.0).min(0.05);
        if self.wind_direction < 0.0 {
            self.wind_direction += 360.0;
        }
        if self.wind_direction >= 360.0 {
            self.wind_direction -= 360.0;
        }
        
        // Weather front progression (simulate frontal passages)
        if self.weather_front_progress > 0.0 {
            self.weather_front_progress -= dt / 7200.0; // 2 hour front passage
            if self.weather_front_progress <= 0.0 {
                self.weather_front_progress = 0.0;
            }
        }
        
        // Drought factor slowly increases without rain (very slow change)
        // Increases by about 0.1 per day without rain in extreme heat
        if self.temperature > 35.0 && self.humidity < 20.0 {
            self.drought_factor = (self.drought_factor + dt / 864000.0).min(10.0); // ~10 days to max
        }
    }
    
    // Setter methods for updating weather without replacement
    
    /// Set temperature target (°C) - will smoothly transition
    pub fn set_temperature(&mut self, temperature: f32) {
        self.target_temperature = temperature;
    }
    
    /// Set humidity target (%) - will smoothly transition
    pub fn set_humidity(&mut self, humidity: f32) {
        self.target_humidity = humidity.clamp(0.0, 100.0);
    }
    
    /// Set wind speed target (km/h) - will smoothly transition
    pub fn set_wind_speed(&mut self, wind_speed: f32) {
        self.target_wind_speed = wind_speed.max(0.0);
    }
    
    /// Set wind direction target (degrees) - will smoothly transition
    pub fn set_wind_direction(&mut self, direction: f32) {
        self.target_wind_direction = direction % 360.0;
    }
    
    /// Set drought factor directly (0-10)
    pub fn set_drought_factor(&mut self, drought: f32) {
        self.drought_factor = drought.clamp(0.0, 10.0);
    }
    
    /// Set time of day (hours since midnight)
    pub fn set_time_of_day(&mut self, hours: f32) {
        self.time_of_day = hours % 24.0;
    }
    
    /// Set day of year (1-365)
    pub fn set_day_of_year(&mut self, day: u16) {
        self.day_of_year = day.clamp(1, 365);
    }
    
    /// Trigger a weather front passage (causes rapid changes)
    pub fn trigger_weather_front(&mut self, new_temp: f32, new_humidity: f32, new_wind_speed: f32, new_wind_dir: f32) {
        self.target_temperature = new_temp;
        self.target_humidity = new_humidity;
        self.target_wind_speed = new_wind_speed;
        self.target_wind_direction = new_wind_dir;
        self.weather_front_progress = 1.0;
    }
    
    /// Get current time of day
    pub fn time_of_day(&self) -> f32 {
        self.time_of_day
    }
    
    /// Get current day of year
    pub fn day_of_year(&self) -> u16 {
        self.day_of_year
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
