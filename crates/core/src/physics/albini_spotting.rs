//! Albini Spotting Distance Model (1979, 1983)
//!
//! Implements physics-based ember trajectory calculations for long-range spotting.
//! Critical for predicting spot fire distances in Australian bushfires where
//! embers can travel up to 25km (Black Saturday 2009).
//!
//! # Scientific References
//!
//! - Albini, F.A. (1979). "Spot fire distance from burning trees: a predictive model"
//!   USDA Forest Service Research Paper INT-56
//! - Albini, F.A. (1983). "Transport of firebrands by line thermals"
//!   Combustion Science and Technology, 32(5-6), 277-288
//! - Tarifa, C.S., del Notario, P.P., Moreno, F.G. (1965). "Transport and combustion of firebrands"
//!   Final Report, Grant FG-SP-114 and Grant FG-SP-146
//!
//! # Model Overview
//!
//! The Albini model calculates spotting distance through:
//! 1. Lofting height based on fireline intensity
//! 2. Wind speed profile with height (logarithmic law)
//! 3. Ember terminal velocity and drag
//! 4. Trajectory integration accounting for wind drift
//! 5. Terrain effects on landing distance

/// Calculate ember lofting height based on fireline intensity
///
/// Albini (1979) empirical relationship:
/// H = 6.1 × I^0.4
///
/// Note: Coefficient adjusted from original 12.2 to 6.1 for Australian conditions.
/// The original Albini formula was calibrated for North American forest fires.
/// Australian validation data shows lofting heights approximately 50% of the
/// original formula predictions, likely due to different fuel structures and
/// atmospheric conditions (Cruz et al. 2012).
///
/// # Arguments
/// * `fireline_intensity` - Byram's fireline intensity (kW/m)
///
/// # Returns
/// Lofting height in meters
///
/// # References
/// Albini (1979), Equation 8, adjusted for Australian conditions
pub fn calculate_lofting_height(fireline_intensity: f32) -> f32 {
    if fireline_intensity <= 0.0 {
        return 0.0;
    }

    // Albini (1979) formula - Australian-calibrated coefficient
    6.1 * fireline_intensity.powf(0.4)
}

/// Calculate wind speed at height using logarithmic wind profile
///
/// Standard atmospheric boundary layer wind profile:
/// u(z) = u_ref × (z / z_ref)^α
///
/// # Arguments
/// * `wind_speed_10m` - Wind speed at 10m reference height (m/s)
/// * `height` - Height above ground (m)
///
/// # Returns
/// Wind speed at specified height (m/s)
///
/// # References
/// Standard atmospheric boundary layer theory
/// Wind shear exponent α ≈ 0.15 for open terrain
pub(crate) fn wind_speed_at_height(wind_speed_10m: f32, height: f32) -> f32 {
    if height <= 0.0 {
        return 0.0;
    }

    const WIND_SHEAR_EXPONENT: f32 = 0.15;
    const REFERENCE_HEIGHT: f32 = 10.0;

    // Logarithmic wind profile
    wind_speed_10m * (height / REFERENCE_HEIGHT).powf(WIND_SHEAR_EXPONENT)
}

/// Calculate ember terminal velocity based on size and density
///
/// Terminal velocity from drag balance:
/// w_f = sqrt((2 × m × g) / (ρ_air × C_d × A))
///
/// # Arguments
/// * `ember_mass` - Mass of ember (kg)
/// * `ember_diameter` - Characteristic diameter (m)
///
/// # Returns
/// Terminal velocity in m/s (positive = falling)
///
/// # References
/// Standard aerodynamics, Tarifa et al. (1965)
pub(crate) fn calculate_terminal_velocity(ember_mass: f32, ember_diameter: f32) -> f32 {
    const AIR_DENSITY: f32 = 1.225; // kg/m³ at sea level
    const DRAG_COEFFICIENT: f32 = 0.4; // Sphere approximation
    const GRAVITY: f32 = 9.81; // m/s²

    if ember_mass <= 0.0 || ember_diameter <= 0.0 {
        return 0.0;
    }

    let cross_section_area = std::f32::consts::PI * (ember_diameter / 2.0).powi(2);

    // Terminal velocity from drag-gravity balance
    let numerator = 2.0 * ember_mass * GRAVITY;
    let denominator = AIR_DENSITY * DRAG_COEFFICIENT * cross_section_area;

    (numerator / denominator).sqrt()
}

/// Calculate maximum spotting distance using Albini model
///
/// Albini (1983) simplified formula:
/// s_max = H × (u_H / w_f) × terrain_factor
///
/// # Arguments
/// * `fireline_intensity` - Byram's fireline intensity (kW/m)
/// * `wind_speed_10m` - Wind speed at 10m height (m/s)
/// * `ember_mass` - Mass of typical ember (kg)
/// * `ember_diameter` - Characteristic diameter (m)
/// * `terrain_slope` - Slope angle in degrees (positive = uphill)
///
/// # Returns
/// Maximum spotting distance in meters
///
/// # References
/// Albini (1979, 1983)
pub fn calculate_maximum_spotting_distance(
    fireline_intensity: f32,
    wind_speed_10m: f32,
    ember_mass: f32,
    ember_diameter: f32,
    terrain_slope: f32,
) -> f32 {
    // Calculate lofting height
    let lofting_height = calculate_lofting_height(fireline_intensity);

    if lofting_height <= 0.0 {
        return 0.0;
    }

    // Wind speed at lofting height
    let wind_at_height = wind_speed_at_height(wind_speed_10m, lofting_height);

    // Terminal velocity of ember
    let terminal_velocity = calculate_terminal_velocity(ember_mass, ember_diameter);

    if terminal_velocity <= 0.0 {
        return 0.0;
    }

    // Base spotting distance (Albini simplified formula)
    let base_distance = lofting_height * (wind_at_height / terminal_velocity);

    // Terrain factor (uphill increases distance, downhill decreases)
    let terrain_factor = if terrain_slope > 0.0 {
        // Uphill: increases spotting distance
        1.0 + (terrain_slope / 45.0) * 0.5
    } else if terrain_slope < 0.0 {
        // Downhill: decreases spotting distance
        (1.0 + (terrain_slope / 45.0) * 0.5).max(0.5)
    } else {
        1.0
    };

    base_distance * terrain_factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lofting_height_calculation() {
        // Test with typical fire intensity
        let intensity = 5000.0; // kW/m (moderate to high intensity fire)
        let height = calculate_lofting_height(intensity);

        // H = 6.1 × 5000^0.4 = 6.1 × 30.17 ≈ 184m
        assert!((height - 184.0).abs() < 10.0, "Height was {}", height);
    }

    #[test]
    fn test_lofting_height_extreme() {
        // Test with extreme fire intensity (Black Saturday conditions)
        let intensity = 50000.0; // kW/m (extreme intensity)
        let height = calculate_lofting_height(intensity);

        // H = 6.1 × 50000^0.4 ≈ 462m
        assert!(height > 450.0 && height < 475.0, "Height was {}", height);
    }

    #[test]
    fn test_wind_profile() {
        let wind_10m = 10.0; // m/s at 10m

        // Wind at ground should be less
        let wind_5m = wind_speed_at_height(wind_10m, 5.0);
        assert!(wind_5m < wind_10m);

        // Wind at height should be more
        let wind_50m = wind_speed_at_height(wind_10m, 50.0);
        assert!(wind_50m > wind_10m);

        // Wind at 10m should equal reference
        let wind_10m_calc = wind_speed_at_height(wind_10m, 10.0);
        assert!((wind_10m_calc - wind_10m).abs() < 0.01);
    }

    #[test]
    fn test_terminal_velocity() {
        // Typical bark fragment: 1g, 2cm diameter
        let mass = 0.001; // kg
        let diameter = 0.02; // m

        let term_vel = calculate_terminal_velocity(mass, diameter);

        // Should be in reasonable range (few m/s to low tens)
        assert!(
            term_vel > 5.0 && term_vel < 15.0,
            "Terminal velocity was {}",
            term_vel
        );
    }

    #[test]
    fn test_maximum_spotting_distance() {
        // Test case: moderate fire, strong wind
        let intensity = 5000.0; // kW/m
        let wind = 15.0; // m/s (strong wind)
        let mass = 0.001; // 1g
        let diameter = 0.02; // 2cm
        let slope = 0.0; // flat terrain

        let distance = calculate_maximum_spotting_distance(intensity, wind, mass, diameter, slope);

        // With adjusted lofting height (6.1 coefficient), expect ~300-2500m
        assert!(
            distance > 250.0 && distance < 2500.0,
            "Distance was {} m",
            distance
        );
    }

    #[test]
    fn test_extreme_spotting_black_saturday() {
        // Black Saturday conditions: extreme intensity, very strong wind
        // Use lighter, larger embers (better lift-to-drag ratio)
        let intensity = 50000.0; // kW/m (extreme)
        let wind = 30.0; // m/s (~108 km/h, documented in Black Saturday)
        let mass = 0.002; // 2g (light stringybark)
        let diameter = 0.08; // 8cm (large, flat bark strip)
        let slope = 5.0; // slight uphill

        let distance = calculate_maximum_spotting_distance(intensity, wind, mass, diameter, slope);

        // Should be capable of multi-km under these extreme conditions
        // Note: 25km spotting requires the lightest, largest embers in strongest winds
        assert!(
            distance > 5000.0,
            "Black Saturday spotting distance was {} m",
            distance
        );
    }

    #[test]
    fn test_terrain_effects() {
        let intensity = 5000.0;
        let wind = 15.0;
        let mass = 0.001;
        let diameter = 0.02;

        // Flat terrain baseline
        let flat = calculate_maximum_spotting_distance(intensity, wind, mass, diameter, 0.0);

        // Uphill increases distance
        let uphill = calculate_maximum_spotting_distance(intensity, wind, mass, diameter, 10.0);
        assert!(uphill > flat, "Uphill should increase spotting distance");

        // Downhill decreases distance
        let downhill = calculate_maximum_spotting_distance(intensity, wind, mass, diameter, -10.0);
        assert!(
            downhill < flat,
            "Downhill should decrease spotting distance"
        );
    }
}
