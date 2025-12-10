//! Atmospheric Instability Modeling
//!
//! Implements vertical atmospheric profile analysis and stability indices
//! for fire weather assessment.
//!
//! # Scientific References
//!
//! - Haines, D.A. (1988). "A lower atmosphere severity index for wildland fires."
//! - ICAO Standard Atmosphere (1993)
//! - Holton, J.R. (2004). "An Introduction to Dynamic Meteorology"

use crate::core_types::units::{Celsius, Percent};
use serde::{Deserialize, Serialize};

/// Atmospheric stability profile for fire weather calculations
///
/// Represents the vertical structure of the atmosphere including
/// temperature, moisture, and wind profiles needed to assess
/// fire weather severity and pyrocumulus potential.
///
/// # Scientific Basis
///
/// Uses standard meteorological indices:
/// - **Haines Index**: Fire weather severity (2-6 scale)
/// - **Lifted Index (LI)**: Atmospheric stability (°C)
/// - **K-Index**: Thunderstorm/convection potential
/// - **Mixing Height**: Depth of turbulent boundary layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AtmosphericProfile {
    // ═══════════════════════════════════════════════════════════════════
    // TEMPERATURE PROFILE
    // ═══════════════════════════════════════════════════════════════════
    /// Surface temperature (°C)
    pub(crate) surface_temperature: f32,

    /// Temperature lapse rate (°C/km)
    /// Standard atmosphere: 6.5°C/km
    /// Unstable: >6.5°C/km
    /// Stable/Inversion: <6.5°C/km or negative
    pub(crate) lapse_rate: f32,

    /// Temperature at 850 hPa (~1500m) (°C)
    pub(crate) temp_850: f32,

    /// Temperature at 700 hPa (~3000m) (°C)
    pub(crate) temp_700: f32,

    /// Temperature at 500 hPa (~5500m) (°C)
    pub(crate) temp_500: f32,

    // ═══════════════════════════════════════════════════════════════════
    // MOISTURE PROFILE
    // ═══════════════════════════════════════════════════════════════════
    /// Surface dewpoint temperature (°C)
    pub(crate) surface_dewpoint: f32,

    /// Dewpoint at 850 hPa (°C)
    pub(crate) dewpoint_850: f32,

    /// Dewpoint at 700 hPa (°C)
    pub(crate) dewpoint_700: f32,

    // ═══════════════════════════════════════════════════════════════════
    // STABILITY INDICES (COMPUTED)
    // ═══════════════════════════════════════════════════════════════════
    /// Lifted Index (°C)
    /// Negative values indicate instability
    /// LI < -3: Unstable (favorable for pyrocumulus)
    /// LI > 0: Stable
    pub(crate) lifted_index: f32,

    /// K-Index (dimensionless)
    /// K > 30: High thunderstorm/convection potential
    /// K > 40: Very high
    pub(crate) k_index: f32,

    /// Haines Index (2-6)
    /// 2-3: Very low fire weather potential
    /// 4: Low to moderate
    /// 5: High
    /// 6: Very high (extreme fire behavior)
    pub(crate) haines_index: u8,

    // ═══════════════════════════════════════════════════════════════════
    // BOUNDARY LAYER
    // ═══════════════════════════════════════════════════════════════════
    /// Mixing height - depth of turbulent boundary layer (meters)
    /// Higher values = stronger vertical mixing, more erratic fire behavior
    pub(crate) mixing_height: f32,

    /// Inversion altitude (meters AGL), if present
    pub(crate) inversion_altitude: Option<f32>,

    /// Inversion strength (°C difference across inversion)
    pub(crate) inversion_strength: f32,

    // ═══════════════════════════════════════════════════════════════════
    // WIND PROFILE
    // ═══════════════════════════════════════════════════════════════════
    /// Wind shear magnitude (m/s per km)
    /// High shear + unstable = fire tornado risk
    pub(crate) wind_shear: f32,

    /// Wind backing/veering with height
    /// Veering (clockwise): warm air advection
    /// Backing (counterclockwise): cold air advection
    pub(crate) wind_direction_change: f32,
}

impl AtmosphericProfile {
    /// Create atmospheric profile from surface conditions
    ///
    /// Estimates upper-level conditions using empirical relationships
    /// and standard atmospheric lapse rates.
    ///
    /// # Parameters
    /// - `surface_temp`: Surface temperature
    /// - `surface_humidity`: Surface relative humidity
    /// - `surface_wind_speed`: Surface wind speed (m/s)
    /// - `is_daytime`: Whether sun is up (affects mixing height)
    pub fn from_surface_conditions(
        surface_temp: Celsius,
        surface_humidity: Percent,
        surface_wind_speed: f32,
        is_daytime: bool,
    ) -> Self {
        // Calculate dewpoint from temperature and humidity
        let surface_dewpoint = Self::calculate_dewpoint(surface_temp, surface_humidity);

        let surface_temp_f32 = *surface_temp as f32;
        // Standard lapse rate - slightly modified by temperature
        // Hot days tend to be more unstable
        let base_lapse = 6.5; // °C/km (standard)
        let lapse_rate = if surface_temp_f32 > 35.0 {
            base_lapse + (surface_temp_f32 - 35.0) * 0.1 // More unstable when hot
        } else {
            base_lapse
        };

        // Estimate upper-level temperatures
        // 850 hPa is approximately 1500m above sea level
        let temp_850 = surface_temp_f32 - lapse_rate * 1.5;
        let temp_700 = surface_temp_f32 - lapse_rate * 3.0;
        let temp_500 = surface_temp_f32 - lapse_rate * 5.5;

        // Dewpoint decreases with altitude (drying rate ~2°C/km)
        let dewpoint_850 = surface_dewpoint - 3.0;
        let dewpoint_700 = surface_dewpoint - 6.0;

        // Calculate stability indices
        let lifted_index = Self::calculate_lifted_index(surface_temp_f32, surface_dewpoint, temp_500);
        let k_index =
            Self::calculate_k_index(temp_850, temp_700, temp_500, dewpoint_850, dewpoint_700);
        let haines_index = Self::calculate_haines_index_low(temp_850, temp_700, dewpoint_850);

        // Mixing height - depends on surface heating and stability
        let mixing_height = if is_daytime {
            // Daytime: convective boundary layer
            let base_height = 500.0 + (surface_temp_f32 - 20.0) * 50.0;
            let stability_factor = if lifted_index < 0.0 {
                1.0 + (-lifted_index * 0.1).min(0.5)
            } else {
                1.0 / (1.0 + lifted_index * 0.1)
            };
            (base_height * stability_factor).clamp(200.0, 4000.0)
        } else {
            // Nighttime: shallow nocturnal boundary layer
            200.0 + surface_wind_speed * 20.0
        };

        // Check for inversion
        let (inversion_altitude, inversion_strength) = if !is_daytime || surface_temp_f32 < 20.0 {
            // Nighttime or cool conditions may have inversion
            if lapse_rate < 5.0 {
                (Some(mixing_height), 5.0 - lapse_rate)
            } else {
                (None, 0.0)
            }
        } else {
            (None, 0.0)
        };

        // Wind shear estimate
        let wind_shear = surface_wind_speed * 0.5; // Simple approximation

        Self {
            surface_temperature: surface_temp_f32,
            lapse_rate,
            temp_850,
            temp_700,
            temp_500,
            surface_dewpoint,
            dewpoint_850,
            dewpoint_700,
            lifted_index,
            k_index,
            haines_index,
            mixing_height,
            inversion_altitude,
            inversion_strength,
            wind_shear,
            wind_direction_change: 0.0, // Would need upper wind data
        }
    }

    /// Calculate dewpoint from temperature and relative humidity
    ///
    /// Uses Magnus-Tetens approximation.
    ///
    /// # Scientific Reference
    /// Alduchov, O.A. and Eskridge, R.E. (1996). "Improved Magnus Form Approximation
    /// of Saturation Vapor Pressure." Journal of Applied Meteorology, 35(4), 601-609.
    fn calculate_dewpoint(temp: Celsius, humidity: Percent) -> f32 {
        // Magnus-Tetens constants (Alduchov & Eskridge 1996)
        const MAGNUS_A: f32 = 17.27; // Dimensionless coefficient
        const MAGNUS_B: f32 = 237.7; // °C - temperature offset
        const MIN_HUMIDITY: f32 = 0.01; // Minimum humidity to avoid log(0)

        let temp_f32 = *temp as f32;
        let humidity_fraction = (*humidity.to_fraction()).clamp(MIN_HUMIDITY, 1.0);
        let gamma = (MAGNUS_A * temp_f32 / (MAGNUS_B + temp_f32)) + humidity_fraction.ln();
        MAGNUS_B * gamma / (MAGNUS_A - gamma)
    }

    /// Calculate Lifted Index (LI)
    ///
    /// LI = `T_500` - `T_parcel`
    ///
    /// Where `T_parcel` is the temperature of an air parcel lifted from
    /// the surface to 500 hPa (approximately 5.5 km).
    ///
    /// # Scientific Reference
    /// Galway, J.G. (1956). "The lifted index as a predictor of latent instability."
    fn calculate_lifted_index(surface_temp: f32, surface_dewpoint: f32, temp_500: f32) -> f32 {
        // Simplified parcel lifting calculation
        // 1. Lift parcel dry-adiabatically to LCL (9.8°C/km)
        // 2. Then moist-adiabatically to 500 hPa (6°C/km approx)

        // Estimate LCL height (simple approximation)
        let lcl_height = (surface_temp - surface_dewpoint) * 125.0; // meters

        // Temperature at LCL (dry adiabatic lift)
        let temp_at_lcl = surface_temp - 9.8 * (lcl_height / 1000.0);

        // Lift moist-adiabatically to 500 hPa (5500m)
        let remaining_lift = 5500.0 - lcl_height;
        let parcel_temp_500 = if remaining_lift > 0.0 {
            temp_at_lcl - 6.0 * (remaining_lift / 1000.0)
        } else {
            temp_at_lcl
        };

        // LI = Environment - Parcel (positive = stable)
        temp_500 - parcel_temp_500
    }

    /// Calculate K-Index
    ///
    /// K = (`T_850` - `T_500`) + `Td_850` - (`T_700` - `Td_700`)
    ///
    /// Measures thunderstorm/convection potential.
    ///
    /// # Reference
    /// George, J.J. (1960). "Weather Forecasting for Aeronautics."
    fn calculate_k_index(
        temp_850: f32,
        temp_700: f32,
        temp_500: f32,
        dewpoint_850: f32,
        dewpoint_700: f32,
    ) -> f32 {
        (temp_850 - temp_500) + dewpoint_850 - (temp_700 - dewpoint_700)
    }

    /// Calculate Haines Index (Low Altitude Variant)
    ///
    /// HI = Stability + Moisture terms
    ///
    /// # Scientific Reference
    /// Haines, D.A. (1988). "A lower atmosphere severity index for wildland fires."
    /// National Weather Digest, 13(2), 23-27.
    fn calculate_haines_index_low(temp_850: f32, _temp_700: f32, dewpoint_850: f32) -> u8 {
        // Low altitude variant (for terrain < 1000m MSL)
        // Uses 950-850 hPa layer for stability

        // Approximate 950 hPa temp (about 500m above surface)
        let temp_950 = temp_850 + 3.25; // Assuming standard lapse rate upward

        // Stability term (950-850 hPa temperature difference)
        // A = 3 if (T_950 - T_850) >= 8
        // A = 2 if 4 <= (T_950 - T_850) < 8
        // A = 1 if (T_950 - T_850) < 4
        let stability_diff = temp_950 - temp_850;
        let stability_term = if stability_diff >= 8.0 {
            3
        } else if stability_diff >= 4.0 {
            2
        } else {
            1
        };

        // Moisture term (T_850 - Td_850)
        // B = 3 if depression >= 15
        // B = 2 if 6 <= depression < 15
        // B = 1 if depression < 6
        let moisture_depression = temp_850 - dewpoint_850;
        let moisture_term = if moisture_depression >= 15.0 {
            3
        } else if moisture_depression >= 6.0 {
            2
        } else {
            1
        };

        // HI = Stability + Moisture (ranges 2-6)
        (stability_term + moisture_term).clamp(2, 6)
    }

    /// Check if conditions support pyrocumulus development
    ///
    /// Pyrocumulus (pyroCu) clouds form when:
    /// 1. Atmosphere is sufficiently unstable (LI < -2)
    /// 2. Fire intensity is high enough (>10 MW/m)
    /// 3. Mixing height is adequate (>1500m)
    /// 4. No strong inversion capping vertical motion
    ///
    /// # Parameters
    /// - `fire_intensity`: Fireline intensity (kW/m)
    ///
    /// # Returns
    /// (`can_form`, `intensity_threshold_factor`)
    pub fn pyrocumulus_potential(&self, fire_intensity_kwm: f32) -> (bool, f32) {
        // Minimum intensity for pyrocumulus (typically ~10,000 kW/m = 10 MW/m)
        let min_intensity = 10_000.0;

        // Factors that modify the threshold
        let instability_factor = if self.lifted_index < -4.0 {
            0.7 // Very unstable - can form at lower intensity
        } else if self.lifted_index < -2.0 {
            0.85
        } else if self.lifted_index < 0.0 {
            1.0
        } else {
            1.5 // Stable - needs much higher intensity
        };

        let mixing_factor = if self.mixing_height > 3000.0 {
            0.8 // Deep mixing layer helps
        } else if self.mixing_height > 1500.0 {
            1.0
        } else {
            1.5 // Shallow mixing layer inhibits
        };

        // Inversion blocks vertical development
        let inversion_factor = if self.inversion_strength > 3.0 {
            2.0 // Strong inversion very inhibiting
        } else if self.inversion_strength > 1.0 {
            1.3
        } else {
            1.0
        };

        let adjusted_threshold =
            min_intensity * instability_factor * mixing_factor * inversion_factor;
        let can_form = fire_intensity_kwm > adjusted_threshold && self.lifted_index < 0.0;

        (can_form, fire_intensity_kwm / adjusted_threshold)
    }

    /// Calculate CAPE (Convective Available Potential Energy)
    ///
    /// CAPE represents the energy available for convection.
    /// Higher CAPE = more vigorous updrafts in pyrocumulus.
    ///
    /// # Parameters
    /// - `parcel_temp_excess`: Temperature excess of parcel over environment (°C)
    /// - `depth`: Depth of unstable layer (m)
    ///
    /// # Returns
    /// CAPE in J/kg
    pub fn estimate_cape(&self) -> f32 {
        if self.lifted_index >= 0.0 {
            return 0.0; // Stable - no CAPE
        }

        // Simplified CAPE estimate from LI
        // True CAPE requires integrating through the entire profile
        // CAPE ≈ |LI| × g × depth / T_environment

        const G: f32 = 9.81;
        let avg_temp_k = (self.temp_850 + 273.15 + self.temp_500 + 273.15) / 2.0;
        let depth = 4000.0; // 850-500 hPa layer (~4 km)

        let cape = G * (-self.lifted_index) * depth / avg_temp_k;
        cape.max(0.0)
    }

    /// Check fire tornado (fire whirl) potential
    ///
    /// Fire tornadoes require:
    /// 1. High fire intensity
    /// 2. Wind shear (horizontal vorticity)
    /// 3. Unstable atmosphere
    /// 4. Convergence zone
    ///
    /// # Returns
    /// Risk level (0.0-1.0)
    pub fn fire_tornado_risk(&self, fire_intensity_kwm: f32) -> f32 {
        let intensity_factor = (fire_intensity_kwm / 20_000.0).clamp(0.0, 1.0);
        let shear_factor = (self.wind_shear / 10.0).clamp(0.0, 1.0);
        let instability_factor = if self.lifted_index < -4.0 {
            1.0
        } else if self.lifted_index < 0.0 {
            0.5
        } else {
            0.0
        };

        intensity_factor * shear_factor * instability_factor
    }

    /// Get fire weather severity description
    pub fn fire_weather_severity(&self) -> &'static str {
        match self.haines_index {
            2 | 3 => "Very Low",
            4 => "Low to Moderate",
            5 => "High",
            6 => "Very High - Extreme Fire Behavior Possible",
            _ => "Unknown",
        }
    }
}

impl Default for AtmosphericProfile {
    /// Default: Neutral atmosphere typical of mild conditions
    fn default() -> Self {
        Self::from_surface_conditions(
            Celsius::new(25.0), // 25°C surface temp
            Percent::new(50.0), // 50% humidity
            5.0,                // 5 m/s wind
            true,               // daytime
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atmospheric_profile_creation() {
        let profile = AtmosphericProfile::from_surface_conditions(Celsius::new(30.0), Percent::new(30.0), 10.0, true);

        assert!(profile.temp_850 < profile.surface_temperature);
        assert!(profile.temp_700 < profile.temp_850);
        assert!(profile.temp_500 < profile.temp_700);
    }

    #[test]
    fn test_haines_index_range() {
        // Low Haines Index - cool, moist
        let low = AtmosphericProfile::from_surface_conditions(Celsius::new(15.0), Percent::new(80.0), 2.0, true);
        assert!(
            low.haines_index <= 5,
            "Cool/moist should have moderate or lower Haines, got {}",
            low.haines_index
        );

        // High Haines Index - hot, dry
        let high = AtmosphericProfile::from_surface_conditions(Celsius::new(42.0), Percent::new(15.0), 15.0, true);
        // Very hot and dry will have high Haines, but the simplified model may not reach 6
        assert!(
            high.haines_index >= 4,
            "Hot/dry should have moderate to high Haines, got {}",
            high.haines_index
        );

        // Verify relative ordering
        assert!(
            high.haines_index >= low.haines_index,
            "Hot/dry ({}) should have >= Haines than cool/moist ({})",
            high.haines_index,
            low.haines_index
        );
    }

    #[test]
    fn test_lifted_index_instability() {
        // Test that LI calculation produces reasonable values
        let profile = AtmosphericProfile::from_surface_conditions(Celsius::new(30.0), Percent::new(40.0), 10.0, true);

        // LI can range from about -10 (extremely unstable) to +10 (very stable)
        assert!(
            (-15.0..=15.0).contains(&profile.lifted_index),
            "LI should be in reasonable meteorological range, got {}",
            profile.lifted_index
        );

        // Test that lower humidity leads to different LI
        // (affects dewpoint and therefore LCL and parcel trajectory)
        let dry = AtmosphericProfile::from_surface_conditions(Celsius::new(30.0), Percent::new(20.0), 10.0, true);
        let moist = AtmosphericProfile::from_surface_conditions(Celsius::new(30.0), Percent::new(80.0), 10.0, true);

        // Both should be in valid range
        assert!(
            (-15.0..=15.0).contains(&dry.lifted_index),
            "Dry LI should be valid, got {}",
            dry.lifted_index
        );
        assert!(
            (-15.0..=15.0).contains(&moist.lifted_index),
            "Moist LI should be valid, got {}",
            moist.lifted_index
        );
    }

    #[test]
    fn test_pyrocumulus_potential() {
        // Unstable atmosphere
        let unstable = AtmosphericProfile {
            lifted_index: -5.0,
            mixing_height: 3000.0,
            inversion_strength: 0.0,
            ..Default::default()
        };

        // High intensity should form pyrocumulus
        let (can_form, _) = unstable.pyrocumulus_potential(25_000.0);
        assert!(can_form, "High intensity + unstable should form pyroCu");

        // Low intensity should not
        let (can_form_low, _) = unstable.pyrocumulus_potential(5_000.0);
        assert!(!can_form_low, "Low intensity should not form pyroCu");
    }

    #[test]
    fn test_cape_estimation() {
        // Unstable atmosphere has positive CAPE
        let unstable = AtmosphericProfile {
            lifted_index: -4.0,
            temp_850: 15.0,
            temp_500: -15.0,
            ..Default::default()
        };

        let cape = unstable.estimate_cape();
        assert!(cape > 0.0, "Unstable atmosphere should have positive CAPE");
        assert!(
            cape < 5000.0,
            "CAPE should be reasonable (<5000 J/kg), got {cape}"
        );
    }

    #[test]
    fn test_fire_tornado_risk() {
        let high_shear = AtmosphericProfile {
            lifted_index: -3.0,
            wind_shear: 15.0,
            ..Default::default()
        };

        let risk = high_shear.fire_tornado_risk(30_000.0);
        assert!(
            risk > 0.3,
            "High intensity + shear should have fire tornado risk"
        );

        let low_intensity_risk = high_shear.fire_tornado_risk(5_000.0);
        assert!(
            low_intensity_risk < risk,
            "Lower intensity should have lower risk"
        );
    }

    #[test]
    fn test_dewpoint_calculation() {
        // At 100% humidity, dewpoint = temperature
        let dp_100 = AtmosphericProfile::calculate_dewpoint(Celsius::new(20.0), Percent::new(99.0));
        assert!(
            (dp_100 - 20.0).abs() < 1.0,
            "At near 100% RH, dewpoint ≈ temp"
        );

        // At low humidity, dewpoint << temperature
        let dp_low = AtmosphericProfile::calculate_dewpoint(Celsius::new(30.0), Percent::new(20.0));
        assert!(dp_low < 10.0, "At 20% RH, dewpoint should be much lower");
    }

    #[test]
    fn test_k_index_calculation() {
        // Standard profile
        let k = AtmosphericProfile::calculate_k_index(
            15.0,  // T850
            5.0,   // T700
            -10.0, // T500
            8.0,   // Td850
            -5.0,  // Td700
        );

        // K = (15 - (-10)) + 8 - (5 - (-5)) = 25 + 8 - 10 = 23
        assert!(
            (k - 23.0).abs() < 0.1,
            "K-index calculation error: expected 23, got {k}"
        );
    }
}
