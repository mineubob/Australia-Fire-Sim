//! Weather simulation module for realistic fire behavior modeling
//!
//! This module implements dynamic weather conditions that directly affect fire spread and behavior.
//! Weather parameters are based on real meteorological data and fire science principles.

use crate::core_types::element::Vec3;
use crate::core_types::units::{Celsius, Degrees, Hours, KilometersPerHour, Percent};
use serde::{Deserialize, Serialize};

/// Climate pattern types affecting weather
///
/// These represent major climate phenomena that influence fire weather across seasons:
/// - **Neutral**: Normal conditions with average temperatures and rainfall
/// - **El Niño**: Warm phase of ENSO, typically causes hotter/drier conditions in Australia
/// - **La Niña**: Cool phase of ENSO, typically causes cooler/wetter conditions in Australia
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ClimatePattern {
    /// Normal atmospheric conditions
    Neutral,
    /// El Niño Southern Oscillation warm phase (hotter, drier)
    ElNino,
    /// El Niño Southern Oscillation cool phase (cooler, wetter)
    LaNina,
}

/// Weather condition preset defining base temperatures, wind, and modifiers for regional climates
///
/// Supports dynamic weather simulation with:
/// - Monthly temperature variations (seasonal cycles)
/// - Diurnal temperature changes (coldest at 6am, hottest at 2pm)
/// - Climate pattern effects (El Niño/La Niña)
/// - Seasonal humidity, wind, and solar radiation patterns
/// - Drought progression based on season and climate
/// - Fuel curing (dryness) percentages affecting ignition and spread
///
/// # Example
/// ```
/// use fire_sim_core::WeatherPreset;
///
/// // Create Perth Metro weather preset
/// let weather = WeatherPreset::perth_metro();
///
/// // Hot, dry summer conditions perfect for fire spread
/// assert!(weather.summer_humidity < 45.0);
/// assert!(weather.summer_curing > 90.0);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeatherPreset {
    /// Region name (e.g., "Perth Metro", "Wheatbelt")
    pub name: String,

    /// Monthly base temperatures as (min, max) pairs in °C
    ///
    /// Array indexed by month: [Jan=0, Feb=1, ..., Dec=11]
    /// - Min: Overnight/early morning temperature (around 6am)
    /// - Max: Afternoon temperature (around 2-3pm)
    ///
    /// Used to calculate diurnal temperature cycles:
    /// - Coldest at 6am (min temp)
    /// - Hottest at 2pm (max temp)  
    /// - Smooth sinusoidal transition between
    pub monthly_temps: [(f32, f32); 12],

    /// Temperature modification during El Niño events (°C)
    ///
    /// El Niño typically adds 1.5-3.0°C to Australian temperatures
    /// Applied additively to monthly base temperatures
    pub el_nino_temp_mod: f32,

    /// Temperature modification during La Niña events (°C)
    ///
    /// La Niña typically reduces temperatures by 0.5-1.5°C
    /// Applied additively to monthly base temperatures (negative value)
    pub la_nina_temp_mod: f32,

    /// Base relative humidity for summer (%)
    ///
    /// Summer (Dec-Feb in Southern Hemisphere)
    /// Lower humidity increases fire danger significantly
    /// Typical range: 20-50% for Australian regions
    pub summer_humidity: f32,

    /// Base relative humidity for autumn (%)
    ///
    /// Autumn (Mar-May)
    /// Transitional season with moderate humidity
    pub autumn_humidity: f32,

    /// Base relative humidity for winter (%)
    ///
    /// Winter (Jun-Aug)\
    /// Highest humidity season, reduced fire risk
    /// Typical range: 45-75% for Australian regions
    pub winter_humidity: f32,

    /// Base relative humidity for spring (%)
    ///
    /// Spring (Sep-Nov)
    /// Fire season begins, humidity decreases
    pub spring_humidity: f32,

    /// Humidity modification during El Niño (% points)
    ///
    /// El Niño reduces humidity by 8-15% points
    /// Dramatically increases fire danger
    pub el_nino_humidity_mod: f32,

    /// Humidity modification during La Niña (% points)
    ///
    /// La Niña increases humidity by 3-8% points
    /// Reduces fire danger
    pub la_nina_humidity_mod: f32,

    /// Base wind speed for summer (km/h)
    ///
    /// Higher wind speeds increase fire spread rate exponentially
    /// Wind affects: rate of spread, spotting distance, ember transport
    pub summer_wind: f32,

    /// Base wind speed for autumn (km/h)
    pub autumn_wind: f32,

    /// Base wind speed for winter (km/h)
    pub winter_wind: f32,

    /// Base wind speed for spring (km/h)
    pub spring_wind: f32,

    /// Temperature increase during heatwave events (°C)
    ///
    /// Heatwaves add to base temperature, creating extreme fire conditions
    /// Typical values: 6-12°C above normal
    /// Combined with low pressure and humidity for catastrophic fire danger
    pub heatwave_temp_bonus: f32,

    /// Base atmospheric pressure (hPa or millibars)
    ///
    /// Standard: 1013 hPa at sea level
    /// Varies 1008-1018 hPa regionally
    /// Affects oxygen availability and combustion
    pub base_pressure: f32,

    /// Pressure drop during heatwave (hPa)
    ///
    /// Low pressure systems bring hot, dry conditions
    /// Typical drop: 6-12 hPa during extreme heat
    pub heatwave_pressure_drop: f32,

    /// Summer pressure modification from base (hPa)
    ///
    /// Usually negative (lower pressure in summer)
    pub summer_pressure_mod: f32,

    /// Winter pressure modification from base (hPa)
    ///
    /// Usually positive (higher pressure in winter)
    pub winter_pressure_mod: f32,

    /// Maximum solar radiation in summer (W/m²)
    ///
    /// Peak intensity affects fuel heating and drying
    /// Typical Australian values: 950-1200 W/m² at solar noon
    /// Influences ignition probability and fire intensity
    pub summer_solar_max: f32,

    /// Maximum solar radiation in autumn (W/m²)
    pub autumn_solar_max: f32,

    /// Maximum solar radiation in winter (W/m²)
    pub winter_solar_max: f32,

    /// Maximum solar radiation in spring (W/m²)
    pub spring_solar_max: f32,

    /// Drought factor progression rate in summer (per day)
    ///
    /// Positive values: drought intensifies (no rainfall)
    /// Negative values: moisture recovery (rainfall period)
    /// Used in Keetch-Byram Drought Index calculation
    /// Typical range: 0.1-0.25 per day in dry periods
    pub summer_drought_rate: f32,

    /// Drought factor progression rate in autumn (per day)
    pub autumn_drought_rate: f32,

    /// Drought factor progression rate in winter (per day)
    ///
    /// Often negative (moisture recovery during rainy season)
    pub winter_drought_rate: f32,

    /// Drought factor progression rate in spring (per day)
    pub spring_drought_rate: f32,

    /// Drought progression modifier during El Niño (per day)
    ///
    /// Positive: accelerates drought during El Niño
    /// Typical: +0.08 to +0.20 per day
    pub el_nino_drought_mod: f32,

    /// Drought progression modifier during La Niña (per day)
    ///
    /// Negative: slows or reverses drought during La Niña
    /// Typical: -0.05 to -0.15 per day
    pub la_nina_drought_mod: f32,

    /// Fuel curing percentage in summer (0-100%)
    ///
    /// Curing = dryness/dead fuel content
    /// - 0%: All green, living fuel (will not burn)
    /// - 50%: Mix of green and dry (slow burning)
    /// - 80%+: Mostly cured (readily combustible)
    /// - 95%+: Fully cured (explosive fire spread)
    ///
    /// Summer typically 90-100% cured in fire-prone regions
    pub summer_curing: f32,

    /// Fuel curing percentage in autumn (%)
    pub autumn_curing: f32,

    /// Fuel curing percentage in winter (%)
    ///
    /// Lowest curing due to rainfall and growth
    /// Typical: 40-75% depending on rainfall
    pub winter_curing: f32,

    /// Fuel curing percentage in spring (%)
    pub spring_curing: f32,
}

impl WeatherPreset {
    /// Catastrophic preset - Extreme fire weather (Code Red)
    #[must_use]
    pub fn catastrophic() -> Self {
        WeatherPreset {
            name: "Catastrophic".to_string(),
            monthly_temps: [
                (42.0, 46.0), // Jan
                (42.0, 46.0), // Feb
                (41.0, 45.0), // Mar
                (40.0, 44.0), // Apr
                (40.0, 42.0), // May
                (40.0, 41.0), // Jun
                (40.0, 41.0), // Jul
                (40.0, 41.0), // Aug
                (40.0, 42.0), // Sep
                (41.0, 44.0), // Oct
                (41.0, 45.0), // Nov
                (42.0, 46.0), // Dec
            ],
            el_nino_temp_mod: 0.0,
            la_nina_temp_mod: 0.0,
            summer_humidity: 10.0,
            autumn_humidity: 10.0,
            winter_humidity: 10.0,
            spring_humidity: 10.0,
            el_nino_humidity_mod: 0.0,
            la_nina_humidity_mod: 0.0,
            summer_wind: 60.0,
            autumn_wind: 60.0,
            winter_wind: 60.0,
            spring_wind: 60.0,
            heatwave_temp_bonus: 0.0,
            base_pressure: 1005.0,
            heatwave_pressure_drop: 0.0,
            summer_pressure_mod: 0.0,
            winter_pressure_mod: 0.0,
            summer_solar_max: 1200.0,
            autumn_solar_max: 1200.0,
            winter_solar_max: 1200.0,
            spring_solar_max: 1200.0,
            summer_drought_rate: 0.0,
            autumn_drought_rate: 0.0,
            winter_drought_rate: 0.0,
            spring_drought_rate: 0.0,
            el_nino_drought_mod: 0.0,
            la_nina_drought_mod: 0.0,
            summer_curing: 100.0,
            autumn_curing: 100.0,
            winter_curing: 100.0,
            spring_curing: 100.0,
        }
    }
    /// Perth Metro preset - Mediterranean climate with hot dry summers
    #[must_use]
    pub fn perth_metro() -> Self {
        WeatherPreset {
            name: "Perth Metro".to_string(),
            // Perth temperatures: hot summer (Dec-Feb), mild winter (Jun-Aug)
            monthly_temps: [
                (18.0, 31.0), // Jan
                (18.0, 31.0), // Feb
                (16.0, 28.0), // Mar
                (13.0, 24.0), // Apr
                (10.0, 20.0), // May
                (8.0, 17.0),  // Jun
                (7.0, 17.0),  // Jul
                (8.0, 18.0),  // Aug
                (9.0, 20.0),  // Sep
                (11.0, 23.0), // Oct
                (14.0, 26.0), // Nov
                (16.0, 29.0), // Dec
            ],
            el_nino_temp_mod: 2.0,
            la_nina_temp_mod: -1.5,
            summer_humidity: 40.0,
            autumn_humidity: 50.0,
            winter_humidity: 65.0,
            spring_humidity: 50.0,
            el_nino_humidity_mod: -10.0,
            la_nina_humidity_mod: 5.0,
            summer_wind: 25.0,
            autumn_wind: 20.0,
            winter_wind: 20.0,
            spring_wind: 22.0,
            heatwave_temp_bonus: 8.0,
            base_pressure: 1013.0,
            heatwave_pressure_drop: 8.0,
            summer_pressure_mod: -2.0,
            winter_pressure_mod: 3.0,
            summer_solar_max: 1000.0,
            autumn_solar_max: 800.0,
            winter_solar_max: 550.0,
            spring_solar_max: 850.0,
            summer_drought_rate: 0.15,
            autumn_drought_rate: 0.05,
            winter_drought_rate: -0.2,
            spring_drought_rate: 0.0,
            el_nino_drought_mod: 0.1,
            la_nina_drought_mod: -0.1,
            summer_curing: 95.0,
            autumn_curing: 80.0,
            winter_curing: 50.0,
            spring_curing: 70.0,
        }
    }

    /// South West preset - Higher rainfall, cooler summers
    #[must_use]
    pub fn south_west() -> Self {
        WeatherPreset {
            name: "South West".to_string(),
            monthly_temps: [
                (16.0, 28.0), // Jan
                (16.0, 28.0), // Feb
                (14.0, 25.0), // Mar
                (11.0, 21.0), // Apr
                (9.0, 18.0),  // May
                (7.0, 15.0),  // Jun
                (6.0, 14.0),  // Jul
                (7.0, 15.0),  // Aug
                (8.0, 17.0),  // Sep
                (10.0, 20.0), // Oct
                (12.0, 23.0), // Nov
                (14.0, 26.0), // Dec
            ],
            el_nino_temp_mod: 1.5,
            la_nina_temp_mod: -1.0,
            summer_humidity: 50.0,
            autumn_humidity: 60.0,
            winter_humidity: 75.0,
            spring_humidity: 60.0,
            el_nino_humidity_mod: -8.0,
            la_nina_humidity_mod: 8.0,
            summer_wind: 22.0,
            autumn_wind: 18.0,
            winter_wind: 20.0,
            spring_wind: 20.0,
            heatwave_temp_bonus: 6.0,
            base_pressure: 1015.0,
            heatwave_pressure_drop: 6.0,
            summer_pressure_mod: -1.5,
            winter_pressure_mod: 2.5,
            summer_solar_max: 950.0,
            autumn_solar_max: 750.0,
            winter_solar_max: 500.0,
            spring_solar_max: 800.0,
            summer_drought_rate: 0.1,
            autumn_drought_rate: 0.0,
            winter_drought_rate: -0.25,
            spring_drought_rate: -0.05,
            el_nino_drought_mod: 0.08,
            la_nina_drought_mod: -0.15,
            summer_curing: 90.0,
            autumn_curing: 70.0,
            winter_curing: 40.0,
            spring_curing: 65.0,
        }
    }

    /// Wheatbelt preset - Hot dry interior
    #[must_use]
    pub fn wheatbelt() -> Self {
        WeatherPreset {
            name: "Wheatbelt".to_string(),
            monthly_temps: [
                (18.0, 33.0), // Jan
                (18.0, 33.0), // Feb
                (15.0, 29.0), // Mar
                (12.0, 24.0), // Apr
                (9.0, 19.0),  // May
                (7.0, 16.0),  // Jun
                (6.0, 15.0),  // Jul
                (7.0, 17.0),  // Aug
                (8.0, 20.0),  // Sep
                (11.0, 24.0), // Oct
                (14.0, 28.0), // Nov
                (16.0, 31.0), // Dec
            ],
            el_nino_temp_mod: 2.5,
            la_nina_temp_mod: -1.0,
            summer_humidity: 30.0,
            autumn_humidity: 40.0,
            winter_humidity: 60.0,
            spring_humidity: 40.0,
            el_nino_humidity_mod: -12.0,
            la_nina_humidity_mod: 5.0,
            summer_wind: 28.0,
            autumn_wind: 22.0,
            winter_wind: 18.0,
            spring_wind: 24.0,
            heatwave_temp_bonus: 10.0,
            base_pressure: 1011.0,
            heatwave_pressure_drop: 10.0,
            summer_pressure_mod: -3.0,
            winter_pressure_mod: 4.0,
            summer_solar_max: 1050.0,
            autumn_solar_max: 850.0,
            winter_solar_max: 600.0,
            spring_solar_max: 900.0,
            summer_drought_rate: 0.2,
            autumn_drought_rate: 0.08,
            winter_drought_rate: -0.15,
            spring_drought_rate: 0.02,
            el_nino_drought_mod: 0.15,
            la_nina_drought_mod: -0.08,
            summer_curing: 98.0,
            autumn_curing: 85.0,
            winter_curing: 60.0,
            spring_curing: 75.0,
        }
    }

    /// Goldfields preset - Very hot, arid
    #[must_use]
    pub fn goldfields() -> Self {
        WeatherPreset {
            name: "Goldfields".to_string(),
            monthly_temps: [
                (20.0, 36.0), // Jan
                (20.0, 35.0), // Feb
                (17.0, 31.0), // Mar
                (13.0, 26.0), // Apr
                (10.0, 21.0), // May
                (7.0, 17.0),  // Jun
                (6.0, 16.0),  // Jul
                (7.0, 18.0),  // Aug
                (9.0, 22.0),  // Sep
                (12.0, 27.0), // Oct
                (16.0, 31.0), // Nov
                (18.0, 34.0), // Dec
            ],
            el_nino_temp_mod: 3.0,
            la_nina_temp_mod: -0.5,
            summer_humidity: 20.0,
            autumn_humidity: 30.0,
            winter_humidity: 45.0,
            spring_humidity: 28.0,
            el_nino_humidity_mod: -15.0,
            la_nina_humidity_mod: 3.0,
            summer_wind: 30.0,
            autumn_wind: 25.0,
            winter_wind: 20.0,
            spring_wind: 28.0,
            heatwave_temp_bonus: 12.0,
            base_pressure: 1010.0,
            heatwave_pressure_drop: 12.0,
            summer_pressure_mod: -4.0,
            winter_pressure_mod: 5.0,
            summer_solar_max: 1100.0,
            autumn_solar_max: 900.0,
            winter_solar_max: 650.0,
            spring_solar_max: 950.0,
            summer_drought_rate: 0.25,
            autumn_drought_rate: 0.12,
            winter_drought_rate: -0.05,
            spring_drought_rate: 0.08,
            el_nino_drought_mod: 0.2,
            la_nina_drought_mod: -0.05,
            summer_curing: 100.0,
            autumn_curing: 95.0,
            winter_curing: 75.0,
            spring_curing: 85.0,
        }
    }

    /// Kimberley preset - Tropical, wet season
    #[must_use]
    pub fn kimberley() -> Self {
        WeatherPreset {
            name: "Kimberley".to_string(),
            monthly_temps: [
                (26.0, 38.0), // Jan - Wet season
                (26.0, 37.0), // Feb - Wet season
                (25.0, 36.0), // Mar
                (22.0, 34.0), // Apr
                (18.0, 31.0), // May
                (15.0, 29.0), // Jun - Dry season
                (14.0, 29.0), // Jul - Dry season
                (16.0, 31.0), // Aug
                (20.0, 34.0), // Sep
                (23.0, 36.0), // Oct
                (25.0, 37.0), // Nov
                (26.0, 38.0), // Dec
            ],
            el_nino_temp_mod: 1.5,
            la_nina_temp_mod: -1.0,
            summer_humidity: 70.0, // High during wet season
            autumn_humidity: 50.0,
            winter_humidity: 30.0, // Low during dry season
            spring_humidity: 45.0,
            el_nino_humidity_mod: -15.0,
            la_nina_humidity_mod: 10.0,
            summer_wind: 18.0,
            autumn_wind: 20.0,
            winter_wind: 25.0,
            spring_wind: 22.0,
            heatwave_temp_bonus: 5.0,
            base_pressure: 1008.0,
            heatwave_pressure_drop: 5.0,
            summer_pressure_mod: -5.0, // Monsoon lows
            winter_pressure_mod: 4.0,
            summer_solar_max: 1150.0,
            autumn_solar_max: 1000.0,
            winter_solar_max: 900.0,
            spring_solar_max: 1050.0,
            summer_drought_rate: -0.3, // Wet season resets drought
            autumn_drought_rate: 0.05,
            winter_drought_rate: 0.2, // Rapid drying
            spring_drought_rate: 0.15,
            el_nino_drought_mod: 0.15,
            la_nina_drought_mod: -0.2,
            summer_curing: 30.0, // Green during wet season
            autumn_curing: 60.0,
            winter_curing: 95.0, // Very dry
            spring_curing: 90.0,
        }
    }

    /// Pilbara preset - Extremely hot, cyclone prone
    #[must_use]
    pub fn pilbara() -> Self {
        WeatherPreset {
            name: "Pilbara".to_string(),
            monthly_temps: [
                (27.0, 39.0), // Jan
                (27.0, 38.0), // Feb
                (25.0, 37.0), // Mar
                (21.0, 33.0), // Apr
                (17.0, 28.0), // May
                (14.0, 25.0), // Jun
                (13.0, 25.0), // Jul
                (14.0, 27.0), // Aug
                (18.0, 31.0), // Sep
                (21.0, 34.0), // Oct
                (24.0, 37.0), // Nov
                (26.0, 39.0), // Dec
            ],
            el_nino_temp_mod: 2.0,
            la_nina_temp_mod: -1.0,
            summer_humidity: 45.0, // Cyclone season
            autumn_humidity: 35.0,
            winter_humidity: 25.0,
            spring_humidity: 30.0,
            el_nino_humidity_mod: -12.0,
            la_nina_humidity_mod: 8.0,
            summer_wind: 22.0,
            autumn_wind: 20.0,
            winter_wind: 25.0,
            spring_wind: 24.0,
            heatwave_temp_bonus: 8.0,
            base_pressure: 1009.0,
            heatwave_pressure_drop: 8.0,
            summer_pressure_mod: -4.0,
            winter_pressure_mod: 3.0,
            summer_solar_max: 1200.0,
            autumn_solar_max: 1000.0,
            winter_solar_max: 850.0,
            spring_solar_max: 1050.0,
            summer_drought_rate: 0.0, // Cyclone rains
            autumn_drought_rate: 0.15,
            winter_drought_rate: 0.2,
            spring_drought_rate: 0.18,
            el_nino_drought_mod: 0.12,
            la_nina_drought_mod: -0.1,
            summer_curing: 70.0,
            autumn_curing: 85.0,
            winter_curing: 95.0,
            spring_curing: 90.0,
        }
    }

    /// Get temperature for specific day and time with modifiers
    #[must_use]
    pub fn get_temperature(
        &self,
        day_of_year: u16,
        time_of_day: f32,
        climate: ClimatePattern,
        is_heatwave: bool,
    ) -> f32 {
        let month = ((day_of_year - 1) / 30).min(11) as usize;
        let (min_temp, max_temp) = self.monthly_temps[month];

        // Apply climate pattern modifier
        let climate_mod = match climate {
            ClimatePattern::ElNino => self.el_nino_temp_mod,
            ClimatePattern::LaNina => self.la_nina_temp_mod,
            ClimatePattern::Neutral => 0.0,
        };

        // Apply heatwave bonus
        let heatwave_mod = if is_heatwave {
            self.heatwave_temp_bonus
        } else {
            0.0
        };

        // Diurnal cycle: coldest at 6am, hottest at 2pm (8 hour half-period)
        // Using π/16 factor so sin reaches 1.0 at 14:00 (2pm)
        // At 6am: sin(0 * π/16) = 0 (min temp)
        // At 2pm: sin(8 * π/16) = sin(π/2) = 1.0 (max temp)
        let hour_factor = ((time_of_day - 6.0) * std::f32::consts::PI / 16.0)
            .sin()
            .max(0.0);

        let base_temp = min_temp + (max_temp - min_temp) * hour_factor;
        base_temp + climate_mod + heatwave_mod
    }

    /// Get humidity for specific season with modifiers
    #[must_use]
    pub fn get_humidity(&self, day_of_year: u16, temperature: f32, climate: ClimatePattern) -> f32 {
        let season_humidity = match (day_of_year - 1) / 91 {
            0 => self.summer_humidity, // Dec-Feb
            1 => self.autumn_humidity, // Mar-May
            2 => self.winter_humidity, // Jun-Aug
            _ => self.spring_humidity, // Sep-Nov
        };

        // Apply climate pattern modifier
        let climate_mod = match climate {
            ClimatePattern::ElNino => self.el_nino_humidity_mod,
            ClimatePattern::LaNina => self.la_nina_humidity_mod,
            ClimatePattern::Neutral => 0.0,
        };

        // Temperature affects humidity (inverse relationship)
        let temp_adjustment = -(temperature - 25.0) * 0.5;

        (season_humidity + climate_mod + temp_adjustment).clamp(5.0, 95.0)
    }

    /// Get wind speed for specific season
    #[must_use]
    pub fn get_wind_speed(&self, day_of_year: u16) -> f32 {
        match (day_of_year - 1) / 91 {
            0 => self.summer_wind,
            1 => self.autumn_wind,
            2 => self.winter_wind,
            _ => self.spring_wind,
        }
    }

    /// Get drought rate for specific season with climate modifier
    #[must_use]
    pub fn get_drought_rate(&self, day_of_year: u16, climate: ClimatePattern) -> f32 {
        let season_rate = match (day_of_year - 1) / 91 {
            0 => self.summer_drought_rate,
            1 => self.autumn_drought_rate,
            2 => self.winter_drought_rate,
            _ => self.spring_drought_rate,
        };

        let climate_mod = match climate {
            ClimatePattern::ElNino => self.el_nino_drought_mod,
            ClimatePattern::LaNina => self.la_nina_drought_mod,
            ClimatePattern::Neutral => 0.0,
        };

        season_rate + climate_mod
    }

    /// Get fuel curing percentage (dryness) for specific season
    #[must_use]
    pub fn get_curing(&self, day_of_year: u16) -> f32 {
        match (day_of_year - 1) / 91 {
            0 => self.summer_curing,
            1 => self.autumn_curing,
            2 => self.winter_curing,
            _ => self.spring_curing,
        }
    }

    /// Get solar radiation for specific season and time
    #[must_use]
    pub fn get_solar_radiation(&self, day_of_year: u16, time_of_day: f32) -> f32 {
        let season_max = match (day_of_year - 1) / 91 {
            0 => self.summer_solar_max,
            1 => self.autumn_solar_max,
            2 => self.winter_solar_max,
            _ => self.spring_solar_max,
        };

        // Solar radiation follows sine curve from sunrise (6am) to sunset (6pm)
        if (6.0..=18.0).contains(&time_of_day) {
            let hour_factor = ((time_of_day - 6.0) * std::f32::consts::PI / 12.0).sin();
            season_max * hour_factor
        } else {
            0.0
        }
    }
}

/// Weather system with `McArthur` Forest Fire Danger Index (FFDI)
///
/// Provides dynamic weather conditions with diurnal cycles, seasonal variations,
/// and scientifically accurate fire danger calculations.
///
/// # `McArthur` Forest Fire Danger Index
///
/// The FFDI is Australia's standard metric for assessing wildfire danger.
/// Formula (Mark 5): `FFDI = 2.11 × exp(-0.45 + 0.987×ln(D) - 0.0345×H + 0.0338×T + 0.0234×V)`
///
/// Where:
/// - **D** = Drought Factor (0-10, from Keetch-Byram Drought Index)
/// - **H** = Relative Humidity (%)
/// - **T** = Air Temperature (°C)
/// - **V** = Wind Speed (km/h)
/// - **2.11** = Calibration constant (matches WA Fire Behaviour Calculator)
///
/// Reference: <https://aurora.landgate.wa.gov.au/fbc/#!/mmk5-forest>
///
/// # Fire Danger Ratings
///
/// - **0-5**: Low (controlled burning possible)
/// - **5-12**: Moderate (heightened awareness)
/// - **12-24**: High (avoid fire-prone activities)
/// - **24-50**: Very High (prepare to evacuate)
/// - **50-100**: Severe (serious fire danger)
/// - **100-150**: Extreme (catastrophic conditions likely)
/// - **150+**: Catastrophic (Code Red - leave high-risk areas)
///
/// # Example
///
/// ```
/// use fire_sim_core::{WeatherSystem, WeatherPreset, ClimatePattern};
///
/// // Create from Perth Metro preset
/// let weather = WeatherSystem::from_preset(
///     WeatherPreset::perth_metro(),
///     15,    // Day 15 (mid-January, peak summer)
///     14.0,  // 2pm (hottest time)
///     ClimatePattern::ElNino,
/// );
///
/// let ffdi = weather.calculate_ffdi();
/// // Expect FFDI 50-100+ on hot summer day with El Niño
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherSystem {
    /// Current air temperature
    pub(crate) temperature: Celsius,

    /// Current relative humidity
    pub(crate) humidity: Percent,

    /// Current wind speed
    pub(crate) wind_speed: KilometersPerHour,

    /// Wind direction (0=North, 90=East, 180=South, 270=West)
    pub(crate) wind_direction: Degrees,

    /// Drought factor (0-10)
    ///
    /// Based on Keetch-Byram Drought Index:
    /// - 0-2: Soil moist, fuels damp
    /// - 2-4: Moderate drying
    /// - 4-6: Significant drying, fire spread increases
    /// - 6-8: Severe drought, rapid fire spread
    /// - 8-10: Extreme drought, explosive fire behavior
    pub(crate) drought_factor: f32,

    /// Time of day in hours (0-24)
    ///
    /// Used for diurnal temperature/humidity cycles:
    /// - 6am: Coldest, highest humidity
    /// - 2pm: Hottest, lowest humidity
    /// - Smooth sinusoidal transitions between
    pub(crate) time_of_day: Hours,

    /// Day of year (1-365)
    ///
    /// Used for seasonal variations in temperature, humidity, wind, etc.
    pub(crate) day_of_year: u16,

    /// Weather front progression (0-1)
    ///
    /// Tracks passage of weather fronts:
    /// - 0.0: No front, stable conditions
    /// - 0.5: Front passing, rapid changes
    /// - 1.0: Front passed, new stable conditions
    pub(crate) weather_front_progress: f32,

    // Target values for smooth transitions
    pub(crate) target_temperature: Celsius,
    pub(crate) target_humidity: Percent,
    pub(crate) target_wind_speed: KilometersPerHour,
    pub(crate) target_wind_direction: Degrees,

    /// Regional weather preset with seasonal patterns
    pub(crate) preset: Option<WeatherPreset>,

    /// Active climate pattern (El Niño, La Niña, or Neutral)
    pub(crate) climate_pattern: ClimatePattern,

    /// Whether a heatwave is occurring
    pub(crate) is_heatwave: bool,

    /// Days remaining in heatwave (if active)
    pub(crate) heatwave_days_remaining: u8,
}

impl WeatherSystem {
    /// Create extreme weather (catastrophic conditions) using the catastrophic preset
    #[must_use]
    pub fn catastrophic() -> Self {
        let mut system = WeatherSystem::from_preset(
            WeatherPreset::catastrophic(),
            15,   // day_of_year (mid-January)
            14.0, // time_of_day (2pm)
            ClimatePattern::ElNino,
        );
        system.drought_factor = 10.0;
        system.temperature = Celsius::new(45.0);
        system.humidity = Percent::new(10.0);
        system.wind_speed = KilometersPerHour::new(60.0);
        system.wind_direction = Degrees::new(0.0);
        system.target_temperature = Celsius::new(45.0);
        system.target_humidity = Percent::new(10.0);
        system.target_wind_speed = KilometersPerHour::new(60.0);
        system.target_wind_direction = Degrees::new(0.0);
        system.is_heatwave = true;
        system.heatwave_days_remaining = 5;
        system
    }
    /// Create a new weather system
    ///
    /// # Arguments
    /// * `temperature` - Air temperature in °C
    /// * `humidity` - Relative humidity in %
    /// * `wind_speed` - Wind speed in km/h
    /// * `wind_direction` - Wind direction in degrees
    /// * `drought_factor` - Drought factor (0-10)
    #[must_use]
    pub fn new(
        temperature: f32,
        humidity: f32,
        wind_speed: f32,
        wind_direction: f32,
        drought_factor: f32,
    ) -> Self {
        WeatherSystem {
            temperature: Celsius::from(temperature),
            humidity: Percent::new(humidity),
            wind_speed: KilometersPerHour::new(wind_speed),
            wind_direction: Degrees::new(wind_direction),
            drought_factor,
            time_of_day: Hours::new(12.0),
            day_of_year: 180,
            weather_front_progress: 0.0,
            target_temperature: Celsius::from(temperature),
            target_humidity: Percent::new(humidity),
            target_wind_speed: KilometersPerHour::new(wind_speed),
            target_wind_direction: Degrees::new(wind_direction),
            preset: None,
            climate_pattern: ClimatePattern::Neutral,
            is_heatwave: false,
            heatwave_days_remaining: 0,
        }
    }

    /// Create weather system from a regional preset
    #[must_use]
    pub fn from_preset(
        preset: WeatherPreset,
        day_of_year: u16,
        time_of_day: f32,
        climate_pattern: ClimatePattern,
    ) -> Self {
        let temperature = preset.get_temperature(day_of_year, time_of_day, climate_pattern, false);
        let humidity = preset.get_humidity(day_of_year, temperature, climate_pattern);
        let wind_speed = preset.get_wind_speed(day_of_year);

        // Initial drought based on season and climate
        let drought_rate = preset.get_drought_rate(day_of_year, climate_pattern);
        let base_drought = if drought_rate > 0.0 { 6.0 } else { 3.0 };

        WeatherSystem {
            temperature: Celsius::from(temperature),
            humidity: Percent::new(humidity),
            wind_speed: KilometersPerHour::new(wind_speed),
            wind_direction: Degrees::new(0.0),
            drought_factor: base_drought,
            time_of_day: Hours::new(time_of_day),
            day_of_year,
            weather_front_progress: 0.0,
            target_temperature: Celsius::from(temperature),
            target_humidity: Percent::new(humidity),
            target_wind_speed: KilometersPerHour::new(wind_speed),
            target_wind_direction: Degrees::new(0.0),
            preset: Some(preset),
            climate_pattern,
            is_heatwave: false,
            heatwave_days_remaining: 0,
        }
    }
}

impl Default for WeatherSystem {
    fn default() -> Self {
        WeatherSystem {
            temperature: Celsius::new(25.0),
            humidity: Percent::new(50.0),
            wind_speed: KilometersPerHour::new(15.0),
            wind_direction: Degrees::new(0.0),
            drought_factor: 5.0,
            time_of_day: Hours::new(12.0),
            day_of_year: 180,
            weather_front_progress: 0.0,
            target_temperature: Celsius::new(25.0),
            target_humidity: Percent::new(50.0),
            target_wind_speed: KilometersPerHour::new(15.0),
            target_wind_direction: Degrees::new(0.0),
            preset: None,
            climate_pattern: ClimatePattern::Neutral,
            is_heatwave: false,
            heatwave_days_remaining: 0,
        }
    }
}

impl WeatherSystem {
    /// Calculate `McArthur` Forest Fire Danger Index (Mark 5)
    ///
    /// The FFDI is the primary fire danger metric used in Australia.
    ///
    /// # Formula
    ///
    /// ```text
    /// FFDI = C × exp(-0.45 + 0.987×ln(D) - 0.0345×H + 0.0338×T + 0.0234×V)
    /// ```
    ///
    /// Where:
    /// - **C** = 2.11 (calibration constant matching WA Fire Behaviour Calculator)
    /// - **D** = Drought Factor (0-10, Keetch-Byram Drought Index)
    /// - **H** = Relative Humidity (%)
    /// - **T** = Air Temperature (°C)
    /// - **V** = Wind Speed (km/h)
    ///
    /// # Physical Meaning
    ///
    /// The FFDI exponentially increases with:
    /// - Higher temperatures (0.0338 coefficient)
    /// - Lower humidity (-0.0345 coefficient, negative because more humidity reduces fire)
    /// - Higher wind speeds (0.0234 coefficient)
    /// - Higher drought factor (0.987 coefficient on logarithm)
    ///
    /// # Returns
    ///
    /// Fire danger index value (typically 0-150+):
    /// - 0-5: Low
    /// - 5-12: Moderate
    /// - 12-24: High
    /// - 24-50: Very High
    /// - 50-75: Severe
    /// - 75-100: Extreme
    /// - 100+: Catastrophic (Code Red)
    ///
    /// # Reference
    ///
    /// Based on `McArthur` (1967) and calibrated to Western Australian data:
    /// <https://aurora.landgate.wa.gov.au/fbc/#!/mmk5-forest>
    ///
    /// # Example
    ///
    /// ```
    /// use fire_sim_core::WeatherSystem;
    ///
    /// // Extreme conditions
    /// let weather = WeatherSystem::new(42.0, 12.0, 55.0, 0.0, 9.5);
    /// let ffdi = weather.calculate_ffdi();
    /// assert!(ffdi > 100.0); // Catastrophic
    /// ```
    #[must_use]
    pub fn calculate_ffdi(&self) -> f32 {
        // Drought Factor must be at least 1.0 for ln() to work
        let df = self.drought_factor.max(1.0);

        // McArthur Mark 5 FFDI formula (official)
        // Reference: Noble et al. (1980) - "McArthur's fire-danger meters expressed as equations"
        // Australian Journal of Ecology, 5(2), 201-203
        // Calibration constant 2.11 provides best match to WA Fire Behaviour Calculator:
        // - T=30°C, H=30%, V=30km/h, D=5 → FFDI=13.0 (reference: 12.7)
        // - T=45°C, H=10%, V=60km/h, D=10 → FFDI=172.3 (reference: 173.5)
        // https://aurora.landgate.wa.gov.au/fbc/#!/mmk5-forest
        let exponent = -0.45 + 0.987 * df.ln() - 0.0345 * *self.humidity
            + 0.0338 * (*self.temperature as f32)
            + 0.0234 * *self.wind_speed;

        let ffdi = 2.11 * exponent.exp();
        ffdi.max(0.0)
    }

    /// Get fire danger rating string
    #[must_use]
    pub fn fire_danger_rating(&self) -> &str {
        match self.calculate_ffdi() {
            f if f < 5.0 => "Low",
            f if f < 12.0 => "Moderate",
            f if f < 24.0 => "High",
            f if f < 50.0 => "Very High",
            f if f < 100.0 => "Severe",
            f if f < 150.0 => "Extreme",
            _ => "CATASTROPHIC", // Code Red
        }
    }

    /// Get spread rate multiplier based on FFDI
    ///
    /// Capped at 3.5 to achieve realistic spread rates:
    ///   - Moderate (FFDI ~11): 1.0x → 1-10 ha/hr
    ///   - Catastrophic (FFDI ~172): 3.5x → 100-300 ha/hr
    ///
    /// Real fire spread still takes time even in extreme FFDI 100+ conditions.
    #[must_use]
    pub fn spread_rate_multiplier(&self) -> f32 {
        // FFDI scales spread rate, but cap at 3.5x to achieve target rates
        // This ensures spread is faster in extreme conditions while remaining realistic
        (self.calculate_ffdi() / 20.0).clamp(1.0, 3.5)
    }

    /// Check if it's currently daytime (6am to 6pm)
    ///
    /// Daytime affects atmospheric stability and convection strength.
    /// During daytime, solar heating creates unstable conditions that
    /// enhance fire behavior through stronger convection.
    #[must_use]
    pub fn is_daytime(&self) -> bool {
        *self.time_of_day >= 6.0 && *self.time_of_day < 18.0
    }

    /// Get current air temperature in Celsius
    #[must_use]
    pub fn temperature(&self) -> Celsius {
        self.temperature
    }

    /// Get wind vector in m/s
    #[must_use]
    pub fn wind_vector(&self) -> Vec3 {
        let wind_ms = *self.wind_speed / 3.6; // Convert km/h to m/s
        let angle_rad = *self.wind_direction.to_radians();

        Vec3::new(angle_rad.sin() * wind_ms, angle_rad.cos() * wind_ms, 0.0)
    }

    /// Get wind speed in m/s
    #[must_use]
    pub fn wind_speed_ms(&self) -> f32 {
        *self.wind_speed / 3.6
    }

    /// Update weather (for dynamic simulations)
    pub fn update(&mut self, dt: f32) {
        // Update time of day
        *self.time_of_day += dt / 3600.0; // dt is in seconds
        if *self.time_of_day >= 24.0 {
            *self.time_of_day -= 24.0;
            self.day_of_year += 1;
            if self.day_of_year > 365 {
                self.day_of_year = 1;
            }

            // Update heatwave status
            if self.heatwave_days_remaining > 0 {
                self.heatwave_days_remaining -= 1;
                if self.heatwave_days_remaining == 0 {
                    self.is_heatwave = false;
                }
            }
        }

        // If using a preset, calculate weather from preset
        if let Some(preset) = &self.preset {
            // Update targets from preset
            let preset_temp = preset.get_temperature(
                self.day_of_year,
                12.0, // Base temperature at noon
                self.climate_pattern,
                self.is_heatwave,
            );
            self.target_temperature = Celsius::new(f64::from(preset_temp));
            *self.target_humidity = preset.get_humidity(
                self.day_of_year,
                *self.target_temperature as f32,
                self.climate_pattern,
            );
            *self.target_wind_speed = preset.get_wind_speed(self.day_of_year);

            // Diurnal temperature cycle from preset
            let temperature = preset.get_temperature(
                self.day_of_year,
                *self.time_of_day,
                self.climate_pattern,
                self.is_heatwave,
            );
            let temperature_f64 = f64::from(temperature);
            let temp_diff = temperature_f64 - *self.temperature;
            *self.temperature += temp_diff * (f64::from(dt) / 3600.0).min(0.1);

            // Humidity varies with temperature
            let humidity =
                preset.get_humidity(self.day_of_year, *self.temperature as f32, self.climate_pattern);
            let humidity_diff = humidity - *self.humidity;
            *self.humidity =
                (*self.humidity + humidity_diff * (dt / 1800.0).min(0.1)).clamp(5.0, 95.0);

            // Wind speed variations (wind typically picks up during day)
            let wind_hour_offset = (*self.time_of_day - 15.0) * std::f32::consts::PI / 12.0;
            let wind_variation = 5.0 * wind_hour_offset.cos(); // ±5 km/h variation
            let target_wind = preset.get_wind_speed(self.day_of_year) - wind_variation;
            let wind_diff = target_wind - *self.wind_speed;
            *self.wind_speed = (*self.wind_speed + wind_diff * (dt / 1800.0).min(0.1)).max(0.0);

            // Update drought factor based on season and climate
            let drought_rate = preset.get_drought_rate(self.day_of_year, self.climate_pattern);
            self.drought_factor =
                (self.drought_factor + drought_rate * dt / 86400.0).clamp(0.0, 10.0);
        } else {
            // Original update logic for non-preset weather
            // Diurnal (daily) temperature cycle
            let hour_offset = (*self.time_of_day - 14.0) * std::f32::consts::PI / 12.0;
            let diurnal_variation = -8.0 * hour_offset.cos();

            let target_with_diurnal = *self.target_temperature + f64::from(diurnal_variation);
            let temp_diff = target_with_diurnal - *self.temperature;
            *self.temperature += temp_diff * (f64::from(dt) / 3600.0).min(0.1);

            // Humidity varies inversely with temperature
            let humidity_variation = 15.0 * hour_offset.cos();
            let target_with_variation = *self.target_humidity + humidity_variation;
            let humidity_diff = target_with_variation - *self.humidity;
            *self.humidity =
                (*self.humidity + humidity_diff * (dt / 1800.0).min(0.1)).clamp(5.0, 95.0);

            // Wind speed variations
            let wind_hour_offset = (*self.time_of_day - 15.0) * std::f32::consts::PI / 12.0;
            let wind_variation = 5.0 * wind_hour_offset.cos();
            let target_wind_with_variation = *self.target_wind_speed - wind_variation;
            let wind_diff = target_wind_with_variation - *self.wind_speed;
            *self.wind_speed = (*self.wind_speed + wind_diff * (dt / 1800.0).min(0.1)).max(0.0);

            // Drought factor slowly increases without rain
            if *self.temperature > 35.0 && *self.humidity < 20.0 {
                self.drought_factor = (self.drought_factor + dt / 864000.0).min(10.0);
            }
        }

        // Wind direction shifts gradually (common to both modes)
        let dir_diff = *self.target_wind_direction - *self.wind_direction;
        let dir_diff = if dir_diff > 180.0 {
            dir_diff - 360.0
        } else if dir_diff < -180.0 {
            dir_diff + 360.0
        } else {
            dir_diff
        };
        *self.wind_direction += dir_diff * (dt / 3600.0).min(0.05);
        if *self.wind_direction < 0.0 {
            *self.wind_direction += 360.0;
        }
        if *self.wind_direction >= 360.0 {
            *self.wind_direction -= 360.0;
        }

        // Weather front progression
        if self.weather_front_progress > 0.0 {
            self.weather_front_progress -= dt / 7200.0;
            if self.weather_front_progress <= 0.0 {
                self.weather_front_progress = 0.0;
            }
        }
    }

    // Setter methods for updating weather without replacement

    /// Set temperature target (°C) - will smoothly transition
    pub fn set_temperature(&mut self, temperature: f32) {
        self.target_temperature = Celsius::from(temperature);
    }

    /// Set humidity target (%) - will smoothly transition
    pub fn set_humidity(&mut self, humidity: f32) {
        self.target_humidity = Percent::new(humidity.clamp(0.0, 100.0));
    }

    /// Set wind speed target (km/h) - will smoothly transition
    pub fn set_wind_speed(&mut self, wind_speed: f32) {
        self.target_wind_speed = KilometersPerHour::new(wind_speed.max(0.0));
    }

    /// Set wind direction target (degrees) - will smoothly transition
    pub fn set_wind_direction(&mut self, direction: f32) {
        self.target_wind_direction = Degrees::new(direction % 360.0);
    }

    /// Set drought factor directly (0-10)
    pub fn set_drought_factor(&mut self, drought: f32) {
        self.drought_factor = drought.clamp(0.0, 10.0);
    }

    /// Set time of day (hours since midnight)
    pub fn set_time_of_day(&mut self, hours: f32) {
        self.time_of_day = Hours::new(hours % 24.0);
    }

    /// Set day of year (1-365)
    pub fn set_day_of_year(&mut self, day: u16) {
        self.day_of_year = day.clamp(1, 365);
    }

    /// Trigger a weather front passage (causes rapid changes)
    pub fn trigger_weather_front(
        &mut self,
        new_temp: f32,
        new_humidity: f32,
        new_wind_speed: f32,
        new_wind_dir: f32,
    ) {
        self.target_temperature = Celsius::from(new_temp);
        self.target_humidity = Percent::new(new_humidity);
        self.target_wind_speed = KilometersPerHour::new(new_wind_speed);
        self.target_wind_direction = Degrees::new(new_wind_dir);
        self.weather_front_progress = 1.0;
    }

    /// Set climate pattern (El Niño, La Niña, or Neutral)
    pub fn set_climate_pattern(&mut self, pattern: ClimatePattern) {
        self.climate_pattern = pattern;
    }

    /// Get current climate pattern
    #[must_use]
    pub fn climate_pattern(&self) -> ClimatePattern {
        self.climate_pattern
    }

    /// Trigger a heatwave event (lasts for specified days)
    pub fn trigger_heatwave(&mut self, days: u8) {
        self.is_heatwave = true;
        self.heatwave_days_remaining = days;
    }

    /// Check if currently in a heatwave
    #[must_use]
    pub fn is_heatwave(&self) -> bool {
        self.is_heatwave
    }

    /// Set the weather preset for regional simulation
    pub fn set_preset(&mut self, preset: WeatherPreset) {
        self.preset = Some(preset);
    }

    /// Get current preset name if any
    #[must_use]
    pub fn preset_name(&self) -> Option<String> {
        self.preset.as_ref().map(|p| p.name.clone())
    }

    /// Get current solar radiation (W/m²) based on preset and time
    #[must_use]
    pub fn solar_radiation(&self) -> f32 {
        if let Some(preset) = &self.preset {
            preset.get_solar_radiation(self.day_of_year, *self.time_of_day)
        } else {
            // Fallback calculation
            if *self.time_of_day < 6.0 || *self.time_of_day > 18.0 {
                0.0
            } else {
                let hour_factor = ((*self.time_of_day - 6.0) * std::f32::consts::PI / 12.0).sin();
                1000.0 * hour_factor
            }
        }
    }

    /// Get fuel curing percentage (0-100%) based on preset and season
    #[must_use]
    pub fn fuel_curing(&self) -> f32 {
        if let Some(preset) = &self.preset {
            preset.get_curing(self.day_of_year)
        } else {
            // Fallback: higher in summer, lower in winter
            let season = (self.day_of_year - 1) / 91;
            match season {
                0 => 95.0, // Summer
                1 => 80.0, // Autumn
                2 => 50.0, // Winter
                _ => 70.0, // Spring
            }
        }
    }

    /// Get current time of day (hours since midnight)
    #[must_use]
    pub fn time_of_day(&self) -> Hours {
        self.time_of_day
    }

    /// Get current day of year
    #[must_use]
    pub fn day_of_year(&self) -> u16 {
        self.day_of_year
    }

    /// Calculate fuel moisture based on weather
    #[must_use]
    pub fn calculate_fuel_moisture(&self, base_moisture: f32) -> f32 {
        // Simplified fuel moisture calculation
        // Higher humidity increases moisture, higher temperature decreases it
        let humidity_factor = *self.humidity / 100.0;
        let temp_factor = (30.0 / (*self.temperature as f32).max(10.0)).min(2.0);

        (base_moisture * humidity_factor * temp_factor).clamp(0.0, 1.0)
    }

    /// Get comprehensive statistics about current weather conditions
    #[must_use]
    pub fn get_stats(&self) -> WeatherStats {
        WeatherStats {
            temperature: self.temperature,
            humidity: self.humidity,
            wind_speed: self.wind_speed,
            wind_direction: self.wind_direction,
            wind_speed_ms: self.wind_speed_ms(),
            drought_factor: self.drought_factor,
            time_of_day: self.time_of_day,
            day_of_year: self.day_of_year,
            ffdi: self.calculate_ffdi(),
            fire_danger_rating: self.fire_danger_rating().to_string(),
            spread_rate_multiplier: self.spread_rate_multiplier(),
            solar_radiation: self.solar_radiation(),
            fuel_curing: self.fuel_curing(),
            climate_pattern: self.climate_pattern,
            is_heatwave: self.is_heatwave,
            heatwave_days_remaining: self.heatwave_days_remaining,
            preset_name: self.preset_name(),
            weather_front_progress: self.weather_front_progress,
        }
    }
}

/// Statistics snapshot of weather system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherStats {
    /// Current air temperature
    pub temperature: Celsius,
    /// Current relative humidity
    pub humidity: Percent,
    /// Current wind speed
    pub wind_speed: KilometersPerHour,
    /// Wind direction
    pub wind_direction: Degrees,
    /// Wind speed in m/s
    pub wind_speed_ms: f32,
    /// Drought factor (0-10)
    pub drought_factor: f32,
    /// Time of day
    pub time_of_day: Hours,
    /// Day of year
    pub day_of_year: u16,
    /// `McArthur` FFDI
    pub ffdi: f32,
    /// Fire danger rating string
    pub fire_danger_rating: String,
    /// Spread rate multiplier
    pub spread_rate_multiplier: f32,
    /// Solar radiation (W/m²)
    pub solar_radiation: f32,
    /// Fuel curing percentage
    pub fuel_curing: f32,
    /// Climate pattern
    pub climate_pattern: ClimatePattern,
    /// Whether in heatwave
    pub is_heatwave: bool,
    /// Days remaining in heatwave
    pub heatwave_days_remaining: u8,
    /// Weather preset name
    pub preset_name: Option<String>,
    /// Weather front progress (0-1)
    pub weather_front_progress: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffdi_calculation() {
        let weather = WeatherSystem::new(30.0, 20.0, 40.0, 0.0, 8.0);
        let ffdi = weather.calculate_ffdi();

        // FFDI should be calculated using McArthur Mark 5 formula:
        // FFDI = 2.11 * exp(-0.45 + 0.987*ln(8) - 0.0345*20 + 0.0338*30 + 0.0234*40)
        // = 2.11 * exp(-0.45 + 2.054 - 0.69 + 1.014 + 0.936)
        // = 2.11 * exp(2.864) = 2.11 * 17.53 = 37.0
        assert!(ffdi > 35.0 && ffdi < 39.0, "FFDI was {ffdi}");
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
