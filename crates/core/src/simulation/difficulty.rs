//! Difficulty mode physics scaling for gameplay balance
//!
//! This module provides physics parameter scaling for different difficulty levels
//! to create appropriate challenge levels for players.

use crate::core_types::units::{Celsius, Percent};
use crate::core_types::weather::WeatherSystem;

/// Difficulty mode for gameplay scaling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifficultyMode {
    /// Trainee mode: Easier conditions for learning
    /// - +20% fuel moisture
    /// - -15% wind speed  
    /// - +30% suppression effectiveness
    Trainee,

    /// Veteran mode: Realistic conditions (no scaling)
    Veteran,

    /// Black Saturday mode: Extreme conditions based on 2009 Black Saturday fires
    /// - -20% fuel moisture
    /// - +25% wind speed
    /// - FFDI forced to 150+
    /// - +50% ember spotting distance
    BlackSaturday,
}

impl DifficultyMode {
    /// Get fuel moisture multiplier
    pub fn fuel_moisture_multiplier(&self) -> f32 {
        match self {
            DifficultyMode::Trainee => 1.20,
            DifficultyMode::Veteran => 1.00,
            DifficultyMode::BlackSaturday => 0.80,
        }
    }

    /// Get wind speed multiplier
    pub fn wind_speed_multiplier(&self) -> f32 {
        match self {
            DifficultyMode::Trainee => 0.85,
            DifficultyMode::Veteran => 1.00,
            DifficultyMode::BlackSaturday => 1.25,
        }
    }

    /// Get suppression effectiveness multiplier
    pub fn suppression_effectiveness_multiplier(&self) -> f32 {
        match self {
            DifficultyMode::Trainee => 1.30,
            DifficultyMode::Veteran => 1.00,
            DifficultyMode::BlackSaturday => 1.00,
        }
    }

    /// Get ember spotting distance multiplier
    pub fn ember_spotting_multiplier(&self) -> f32 {
        match self {
            DifficultyMode::Trainee => 1.00,
            DifficultyMode::Veteran => 1.00,
            DifficultyMode::BlackSaturday => 1.50,
        }
    }

    /// Get minimum FFDI for this mode (None if no minimum)
    pub fn min_ffdi(&self) -> Option<f32> {
        match self {
            DifficultyMode::Trainee => None,
            DifficultyMode::Veteran => None,
            DifficultyMode::BlackSaturday => Some(150.0),
        }
    }

    /// Apply difficulty scaling to weather conditions
    pub fn apply_to_weather(&self, weather: &mut WeatherSystem) {
        // Scale wind speed
        let wind_speed_km_h = *weather.wind_speed * self.wind_speed_multiplier();
        weather.wind_speed = crate::core_types::units::KilometersPerHour::new(wind_speed_km_h);

        // Force FFDI for Black Saturday mode
        if let Some(min_ffdi) = self.min_ffdi() {
            // Recalculate FFDI with modified conditions
            // Note: This is simplified; full implementation would modify
            // temperature, humidity, etc. to achieve target FFDI
            let current_ffdi = weather.calculate_ffdi();
            if current_ffdi < min_ffdi {
                // Scale temperature and reduce humidity to increase FFDI
                let scale_factor = min_ffdi / current_ffdi.max(1.0);
                weather.temperature =
                    Celsius::new(*weather.temperature * (scale_factor as f64).min(1.5));
                weather.humidity = Percent::new(*weather.humidity * 0.5_f32.max(0.05));
            }
        }
    }

    /// Apply difficulty scaling to fuel moisture
    pub fn apply_to_fuel_moisture(&self, moisture: f32) -> f32 {
        moisture * self.fuel_moisture_multiplier()
    }

    /// Apply difficulty scaling to suppression effectiveness
    pub fn apply_to_suppression(&self, effectiveness: f32) -> f32 {
        (effectiveness * self.suppression_effectiveness_multiplier()).min(1.0)
    }
}

impl Default for DifficultyMode {
    fn default() -> Self {
        DifficultyMode::Veteran
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::weather::WeatherSystem;

    #[test]
    fn test_trainee_mode_multipliers() {
        let mode = DifficultyMode::Trainee;
        assert_eq!(mode.fuel_moisture_multiplier(), 1.20);
        assert_eq!(mode.wind_speed_multiplier(), 0.85);
        assert_eq!(mode.suppression_effectiveness_multiplier(), 1.30);
        assert_eq!(mode.min_ffdi(), None);
    }

    #[test]
    fn test_veteran_mode_multipliers() {
        let mode = DifficultyMode::Veteran;
        assert_eq!(mode.fuel_moisture_multiplier(), 1.00);
        assert_eq!(mode.wind_speed_multiplier(), 1.00);
        assert_eq!(mode.suppression_effectiveness_multiplier(), 1.00);
        assert_eq!(mode.min_ffdi(), None);
    }

    #[test]
    fn test_black_saturday_mode_multipliers() {
        let mode = DifficultyMode::BlackSaturday;
        assert_eq!(mode.fuel_moisture_multiplier(), 0.80);
        assert_eq!(mode.wind_speed_multiplier(), 1.25);
        assert_eq!(mode.ember_spotting_multiplier(), 1.50);
        assert_eq!(mode.min_ffdi(), Some(150.0));
    }

    #[test]
    fn test_apply_to_fuel_moisture() {
        let trainee = DifficultyMode::Trainee;
        let veteran = DifficultyMode::Veteran;
        let black_saturday = DifficultyMode::BlackSaturday;

        let base_moisture = 0.10;
        assert_eq!(trainee.apply_to_fuel_moisture(base_moisture), 0.12);
        assert_eq!(veteran.apply_to_fuel_moisture(base_moisture), 0.10);
        assert_eq!(black_saturday.apply_to_fuel_moisture(base_moisture), 0.08);
    }

    #[test]
    fn test_apply_to_suppression() {
        let trainee = DifficultyMode::Trainee;
        let base_eff = 0.70;

        let scaled = trainee.apply_to_suppression(base_eff);
        assert_eq!(scaled, 0.91);

        // Should cap at 1.0
        let high_eff = 0.90;
        let scaled_high = trainee.apply_to_suppression(high_eff);
        assert_eq!(scaled_high, 1.0);
    }

    #[test]
    fn test_apply_to_weather() {
        let mut weather = WeatherSystem::new(35.0, 0.15, 50.0, 90.0, 8.0);
        let original_wind = *weather.wind_speed;

        let mode = DifficultyMode::BlackSaturday;
        mode.apply_to_weather(&mut weather);

        // Wind should be scaled up
        assert!(*weather.wind_speed > original_wind);
        assert!((*weather.wind_speed - (original_wind * 1.25)).abs() < 0.01);
    }

    #[test]
    fn test_default_is_veteran() {
        assert_eq!(DifficultyMode::default(), DifficultyMode::Veteran);
    }
}
