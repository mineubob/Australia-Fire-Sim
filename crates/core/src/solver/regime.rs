//! Fire regime detection
//!
//! Detects whether fire is wind-driven or plume-dominated using the Byram number.
//!
//! # Scientific Background
//!
//! Fires operate in two fundamental regimes:
//! 1. **Wind-Driven:** Ambient wind controls fire behavior (predictable)
//! 2. **Plume-Dominated:** Fire's convection controls behavior (erratic)
//!
//! The transition between regimes is particularly dangerous because:
//! - Fire direction can change suddenly
//! - Spread rate can accelerate unpredictably
//! - Standard fire behavior predictions become unreliable
//!
//! The Byram number (`N_c`) discriminates between regimes:
//! - `N_c` < 1: Wind-driven regime
//! - `N_c` > 10: Plume-dominated regime
//! - 1 < `N_c` < 10: Transitional (most dangerous)
//!
//! # Scientific References
//!
//! - Byram, G.M. (1959). "Combustion of forest fuels." Forest Fires: Control and Use.
//! - Nelson, R.M. (2003). "Power of the fire—a thermodynamic analysis."
//!   Int. J. Wildland Fire 12:51-65.
//! - Finney, M.A. and McAllister, S.S. (2011). "A review of fire interactions and mass fires."
//!   J. Combustion.
#![expect(
    clippy::doc_markdown,
    reason = "McAllister is a scientific author name, not a code identifier"
)]

/// Fire behavior regime
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireRegime {
    /// Wind controls fire direction and spread (`N_c` < 1)
    WindDriven,
    /// Fire convection controls behavior (`N_c` > 10)
    PlumeDominated,
    /// Transitional regime - most dangerous (1 ≤ `N_c` ≤ 10)
    Transitional,
}

/// Calculate Byram number for regime detection
///
/// Byram Number formula:
/// `N_c` = (2 × g × I) / (ρ × `c_p` × T × U³)
///
/// Where:
/// - g: Gravity (9.81 m/s²)
/// - I: Fire intensity (W/m)
/// - ρ: Air density (kg/m³)
/// - `c_p`: Specific heat of air (J/kg·K)
/// - T: Ambient temperature (K)
/// - U: Wind speed (m/s)
///
/// # Arguments
///
/// * `fire_intensity` - Fire line intensity in W/m (Byram's intensity)
/// * `wind_speed` - Wind speed in m/s
/// * `ambient_temp` - Ambient temperature in °C
///
/// # Returns
///
/// Byram number (dimensionless)
pub fn byram_number(fire_intensity: f32, wind_speed: f32, ambient_temp: f32) -> f32 {
    const G: f32 = 9.81; // Gravity (m/s²)
    const RHO: f32 = 1.225; // Air density at sea level (kg/m³)
    const CP: f32 = 1005.0; // Specific heat of air (J/kg·K)

    let t_kelvin = ambient_temp + 273.15;

    // Explicitly handle zero or effectively zero wind speed:
    // In calm conditions (wind < 0.5 m/s), the fire is inherently plume-dominated
    // regardless of intensity, since the plume will dominate over negligible wind.
    // Artificial clamping to 0.5 m/s creates large errors: at 0.1 m/s actual wind,
    // clamping gives (0.5/0.1)³ = 125× error in N_c calculation.
    // Instead, return infinity to force PlumeDominated classification.
    if wind_speed < 0.5 {
        return f32::INFINITY;
    }

    let u_cubed = wind_speed.powi(3);

    (2.0 * G * fire_intensity) / (RHO * CP * t_kelvin * u_cubed)
}

/// Determine fire regime from conditions
///
/// # Arguments
///
/// * `fire_intensity` - Fire line intensity in W/m
/// * `wind_speed` - Wind speed in m/s
/// * `ambient_temp` - Ambient temperature in °C
///
/// # Returns
///
/// Fire regime classification
pub fn detect_regime(fire_intensity: f32, wind_speed: f32, ambient_temp: f32) -> FireRegime {
    let nc = byram_number(fire_intensity, wind_speed, ambient_temp);

    if nc < 1.0 {
        FireRegime::WindDriven
    } else if nc > 10.0 {
        FireRegime::PlumeDominated
    } else {
        FireRegime::Transitional
    }
}

/// Get spread direction uncertainty for regime
///
/// Returns the expected angular uncertainty in fire spread direction based on regime.
///
/// # Arguments
///
/// * `regime` - Fire regime classification
///
/// # Returns
///
/// Direction uncertainty in degrees (±)
pub fn direction_uncertainty(regime: FireRegime) -> f32 {
    match regime {
        FireRegime::WindDriven => 15.0,      // ±15° uncertainty
        FireRegime::Transitional => 60.0,    // ±60° uncertainty
        FireRegime::PlumeDominated => 180.0, // Can go any direction
    }
}

/// Get spread rate predictability factor
///
/// Returns a factor (0-1) indicating how predictable the spread rate is,
/// with 1.0 being highly predictable and 0.0 being highly unpredictable.
///
/// # Arguments
///
/// * `regime` - Fire regime classification
///
/// # Returns
///
/// Predictability factor (0.0 to 1.0)
pub fn predictability_factor(regime: FireRegime) -> f32 {
    match regime {
        FireRegime::WindDriven => 1.0,     // Highly predictable
        FireRegime::Transitional => 0.5,   // Moderate predictability
        FireRegime::PlumeDominated => 0.2, // Low predictability
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byram_number_low_for_strong_wind() {
        // Strong wind, low intensity = wind-driven
        let nc = byram_number(
            5000.0, // 5 kW/m intensity
            10.0,   // 10 m/s wind
            20.0,   // 20°C
        );

        assert!(nc < 1.0, "Strong wind should give low Byram number: {nc}");
    }

    #[test]
    fn test_byram_number_high_for_weak_wind() {
        // Weak wind, high intensity = plume-dominated
        let nc = byram_number(
            2000000.0, // 2000 kW/m intensity (extreme crown fire)
            1.0,       // 1 m/s wind (very light)
            20.0,      // 20°C
        );

        assert!(
            nc > 10.0,
            "Weak wind with high intensity should give high Byram number: {nc}"
        );
    }

    #[test]
    fn test_detect_regime_wind_driven() {
        let regime = detect_regime(
            3000.0, // 3 kW/m
            12.0,   // 12 m/s strong wind
            25.0,   // 25°C
        );

        assert_eq!(
            regime,
            FireRegime::WindDriven,
            "Strong wind should be wind-driven"
        );
    }

    #[test]
    fn test_detect_regime_plume_dominated() {
        let regime = detect_regime(
            2000000.0, // 2000 kW/m (extreme intensity - major crown fire)
            1.0,       // 1.0 m/s (very light wind, clamped to 0.5)
            30.0,      // 30°C
        );

        assert_eq!(
            regime,
            FireRegime::PlumeDominated,
            "Intense fire with light wind should be plume-dominated"
        );
    }

    #[test]
    fn test_detect_regime_transitional() {
        let intensity = 750000.0; // 750 kW/m (high intensity - major crown fire)
        let wind = 2.0; // 2.0 m/s (light wind)
        let temp = 25.0; // 25°C

        let regime = detect_regime(intensity, wind, temp);

        assert_eq!(
            regime,
            FireRegime::Transitional,
            "High intensity with light wind should be transitional"
        );
    }

    #[test]
    fn test_direction_uncertainty_increases_with_plume_dominance() {
        let wind_uncertainty = direction_uncertainty(FireRegime::WindDriven);
        let trans_uncertainty = direction_uncertainty(FireRegime::Transitional);
        let plume_uncertainty = direction_uncertainty(FireRegime::PlumeDominated);

        assert!(
            wind_uncertainty < trans_uncertainty,
            "Transitional should have more uncertainty than wind-driven"
        );
        assert!(
            trans_uncertainty < plume_uncertainty,
            "Plume-dominated should have most uncertainty"
        );
    }

    #[test]
    fn test_predictability_decreases_with_plume_dominance() {
        let wind_pred = predictability_factor(FireRegime::WindDriven);
        let trans_pred = predictability_factor(FireRegime::Transitional);
        let plume_pred = predictability_factor(FireRegime::PlumeDominated);

        assert!(
            wind_pred > trans_pred,
            "Wind-driven should be more predictable than transitional"
        );
        assert!(
            trans_pred > plume_pred,
            "Transitional should be more predictable than plume-dominated"
        );
        assert!((0.0..=1.0).contains(&plume_pred));
    }

    #[test]
    fn test_byram_number_prevents_division_by_zero() {
        // Zero or very low wind speed should return infinity (plume-dominated)
        // rather than causing division by zero or using artificial clamping
        let nc = byram_number(
            10000.0, // 10 kW/m
            0.0,     // Zero wind
            20.0,    // 20°C
        );

        assert!(nc.is_infinite(), "Zero wind should return infinity (plume-dominated)");
        assert!(nc > 0.0, "Should return positive value");
        
        // Very low wind (< 0.5 m/s) should also return infinity
        let nc_low = byram_number(
            10000.0, // 10 kW/m
            0.1,     // Very low wind
            20.0,    // 20°C
        );
        assert!(nc_low.is_infinite(), "Wind < 0.5 m/s should return infinity (plume-dominated)");
    }

    #[test]
    fn test_byram_number_scales_with_intensity() {
        let ambient_temp = 25.0;
        let wind_speed = 5.0;

        let nc_low = byram_number(5000.0, wind_speed, ambient_temp);
        let nc_high = byram_number(50000.0, wind_speed, ambient_temp);

        assert!(
            nc_high > nc_low,
            "Higher intensity should give higher Byram number"
        );
        assert!(
            nc_high / nc_low > 9.0,
            "10× intensity should give ~10× Byram number"
        );
    }

    #[test]
    fn test_byram_number_inversely_scales_with_wind_cubed() {
        let intensity = 10000.0;
        let ambient_temp = 25.0;

        let nc_low_wind = byram_number(intensity, 2.0, ambient_temp);
        let nc_high_wind = byram_number(intensity, 4.0, ambient_temp);

        // 2× wind speed → 8× wind^3 → 1/8 Byram number
        assert!(
            nc_low_wind / nc_high_wind > 7.0,
            "Doubling wind should reduce Byram number by ~8×"
        );
    }
}
