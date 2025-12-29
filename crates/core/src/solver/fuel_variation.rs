//! Sub-grid fuel variation for heterogeneous landscapes.
//!
//! Applies spatially correlated noise to fuel properties to create
//! realistic patchy fuel distributions following Finney (2003).
//!
//! # Scientific Background
//!
//! Real fuel loads are never uniform. Vegetation varies due to:
//! - Microclimate gradients (aspect, elevation, drainage)
//! - Soil variability and moisture availability
//! - Historical disturbance patterns (previous fires, logging)
//! - Species distribution and competition
//!
//! This heterogeneity significantly affects fire behavior:
//! - Creates gaps that can slow or stop fire spread
//! - Produces variable intensity along the fire front
//! - Influences spotfire ignition probability
//!
//! # Implementation
//!
//! Fuel variation is applied using the formula from Finney (2003):
//! ```text
//! F(x,y) = F_base × (1 + σ_fuel × η(x,y))
//! ```
//! Where:
//! - `F_base` is the original fuel load
//! - `σ_fuel` is the coefficient of variation (typically 0.2-0.5)
//! - `η(x,y)` ∈ [-1, 1] is spatially correlated noise
//!
//! Moisture variation incorporates terrain aspect effects following
//! Bradshaw (1984), with north-facing slopes being drier in the
//! Southern Hemisphere due to higher solar exposure.
//!
//! # References
//!
//! - Finney, M.A. (2003). Calculation of fire spread rates across random
//!   landscapes. International Journal of Wildland Fire, 12(2), 167-174.
//! - Bradshaw, L.S. et al. (1984). The 1978 National Fire-Danger Rating
//!   System: Technical Documentation. USDA Forest Service.

use super::noise::NoiseGenerator;

/// Configuration for fuel heterogeneity.
///
/// Controls how spatial variation is applied to fuel and moisture fields.
/// Disable individual variation types by setting their enabled flags to false.
#[derive(Clone, Debug)]
pub struct HeterogeneityConfig {
    /// Enable fuel load variation.
    pub fuel_variation_enabled: bool,

    /// Coefficient of variation for fuel load (0.0-1.0, typically 0.2-0.5).
    ///
    /// Higher values create more patchy fuel distributions:
    /// - 0.2: Relatively uniform fuel (grasslands)
    /// - 0.3: Moderate variation (open woodland)
    /// - 0.5: High variation (mixed forest with gaps)
    pub fuel_cv: f32,

    /// Enable moisture microclimate variation.
    pub moisture_variation_enabled: bool,

    /// Coefficient of variation for moisture (0.0-0.5).
    ///
    /// Moisture variation is typically smaller than fuel variation:
    /// - 0.1: Low variation (flat terrain, uniform canopy)
    /// - 0.2: Moderate variation (varied topography)
    /// - 0.3: High variation (complex terrain with gullies)
    pub moisture_cv: f32,

    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for HeterogeneityConfig {
    /// Create default configuration with moderate variation.
    ///
    /// Uses typical values for Australian eucalyptus forest:
    /// - Fuel CV: 0.3 (moderate patchiness)
    /// - Moisture CV: 0.15 (terrain-driven variation)
    fn default() -> Self {
        Self {
            fuel_variation_enabled: true,
            fuel_cv: 0.3,
            moisture_variation_enabled: true,
            moisture_cv: 0.15,
            seed: 0,
        }
    }
}

impl HeterogeneityConfig {
    /// Create a new configuration with specified parameters.
    ///
    /// # Arguments
    ///
    /// * `fuel_cv` - Coefficient of variation for fuel load (clamped to 0.0-1.0)
    /// * `moisture_cv` - Coefficient of variation for moisture (clamped to 0.0-0.5)
    /// * `seed` - Random seed for reproducibility
    #[must_use]
    pub fn new(fuel_cv: f32, moisture_cv: f32, seed: u64) -> Self {
        Self {
            fuel_variation_enabled: true,
            fuel_cv: fuel_cv.clamp(0.0, 1.0),
            moisture_variation_enabled: true,
            moisture_cv: moisture_cv.clamp(0.0, 0.5),
            seed,
        }
    }

    /// Create configuration with fuel variation only.
    #[must_use]
    pub fn fuel_only(fuel_cv: f32, seed: u64) -> Self {
        Self {
            fuel_variation_enabled: true,
            fuel_cv: fuel_cv.clamp(0.0, 1.0),
            moisture_variation_enabled: false,
            moisture_cv: 0.0,
            seed,
        }
    }

    /// Create configuration with moisture variation only.
    #[must_use]
    pub fn moisture_only(moisture_cv: f32, seed: u64) -> Self {
        Self {
            fuel_variation_enabled: false,
            fuel_cv: 0.0,
            moisture_variation_enabled: true,
            moisture_cv: moisture_cv.clamp(0.0, 0.5),
            seed,
        }
    }

    /// Create configuration with all variation disabled.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            fuel_variation_enabled: false,
            fuel_cv: 0.0,
            moisture_variation_enabled: false,
            moisture_cv: 0.0,
            seed: 0,
        }
    }
}

/// Apply sub-grid fuel variation to fuel and moisture fields.
///
/// Modifies fuel load and moisture content in-place using spatially
/// correlated noise to create realistic heterogeneous fuel distributions.
///
/// # Arguments
///
/// * `fuel_load` - Mutable slice of fuel load values (kg/m²)
/// * `moisture` - Mutable slice of moisture content values (fraction 0.0-1.0)
/// * `aspect` - Terrain aspect values for moisture variation (degrees, 0=North)
/// * `noise` - Noise generator instance
/// * `config` - Heterogeneity configuration
/// * `width` - Grid width in cells
/// * `height` - Grid height in cells
/// * `cell_size` - Cell size in meters
///
/// # Formulas Applied
///
/// Fuel load variation (Finney 2003):
/// ```text
/// F(x,y) = F_base × (1 + cv × noise(x,y))
/// ```
///
/// Moisture variation includes aspect factor (Bradshaw 1984):
/// ```text
/// M(x,y) = M_base × (1 + cv × noise(x,y)) + aspect_factor(aspect)
/// ```
///
/// # Panics
///
/// Panics if slice lengths don't match width × height.
#[expect(clippy::too_many_arguments)]
#[expect(clippy::cast_precision_loss)]
pub fn apply_fuel_heterogeneity(
    fuel_load: &mut [f32],
    moisture: &mut [f32],
    aspect: &[f32],
    noise: &NoiseGenerator,
    config: &HeterogeneityConfig,
    width: u32,
    height: u32,
    cell_size: f32,
) {
    let expected_size = (width as usize) * (height as usize);
    assert_eq!(
        fuel_load.len(),
        expected_size,
        "Fuel load slice length mismatch"
    );
    assert_eq!(
        moisture.len(),
        expected_size,
        "Moisture slice length mismatch"
    );
    assert_eq!(aspect.len(), expected_size, "Aspect slice length mismatch");

    // Early return if nothing to do
    if !config.fuel_variation_enabled && !config.moisture_variation_enabled {
        return;
    }

    for y in 0..height {
        for x in 0..width {
            let idx = (y as usize) * (width as usize) + (x as usize);
            let world_x = (x as f32) * cell_size;
            let world_y = (y as f32) * cell_size;

            // Sample noise at this location
            let noise_value = noise.sample(world_x, world_y);

            // Apply fuel variation: F(x,y) = F_base × (1 + cv × noise)
            if config.fuel_variation_enabled {
                let multiplier = 1.0 + config.fuel_cv * noise_value;
                // Ensure fuel load never goes negative
                fuel_load[idx] = (fuel_load[idx] * multiplier).max(0.0);
            }

            // Apply moisture variation with aspect factor
            if config.moisture_variation_enabled {
                let aspect_degrees = aspect[idx];
                let aspect_factor = calculate_aspect_moisture_factor(aspect_degrees);

                // Moisture variation: M = M_base × (1 + cv × noise) + aspect_factor
                let base_moisture = moisture[idx];
                let noise_multiplier = 1.0 + config.moisture_cv * noise_value;
                let new_moisture = base_moisture * noise_multiplier + aspect_factor;

                // Clamp moisture to valid range [0.0, 1.0]
                moisture[idx] = new_moisture.clamp(0.0, 1.0);
            }
        }
    }
}

/// Calculate aspect-based moisture factor (Bradshaw 1984).
///
/// In the Southern Hemisphere, north-facing slopes receive more direct
/// sunlight and are therefore drier. South-facing slopes are more shaded
/// and retain moisture better.
///
/// # Arguments
///
/// * `aspect_degrees` - Terrain aspect in degrees (0=North, 90=East, 180=South, 270=West)
///
/// # Returns
///
/// Moisture modification factor in range approximately [-0.3, +0.3]:
/// - Negative values (drier) for north-facing slopes
/// - Positive values (wetter) for south-facing slopes
/// - Near zero for east/west-facing slopes
///
/// # Formula
///
/// ```text
/// factor = -0.3 × cos(aspect_radians)
/// ```
///
/// Where aspect=0 (North) gives -0.3 (drier) and aspect=180 (South) gives +0.3 (wetter).
#[must_use]
pub fn calculate_aspect_moisture_factor(aspect_degrees: f32) -> f32 {
    let aspect_radians = aspect_degrees.to_radians();

    // In Southern Hemisphere:
    // - North (0°): cos(0) = 1.0 → factor = -0.3 (drier)
    // - South (180°): cos(π) = -1.0 → factor = +0.3 (wetter)
    // - East/West (90°/270°): cos(π/2) = 0 → factor = 0.0 (neutral)
    -0.3 * aspect_radians.cos()
}

/// Apply fuel heterogeneity to a single cell (convenience function).
///
/// Useful for on-demand heterogeneity calculation without pre-generating
/// the entire noise field.
///
/// # Arguments
///
/// * `base_fuel_load` - Original fuel load (kg/m²)
/// * `base_moisture` - Original moisture content (fraction)
/// * `aspect_degrees` - Terrain aspect at this cell (degrees)
/// * `noise` - Noise generator
/// * `config` - Heterogeneity configuration
/// * `world_x` - X position in world coordinates (meters)
/// * `world_y` - Y position in world coordinates (meters)
///
/// # Returns
///
/// Tuple of (modified fuel load, modified moisture)
#[must_use]
pub fn apply_heterogeneity_single(
    base_fuel_load: f32,
    base_moisture: f32,
    aspect_degrees: f32,
    noise: &NoiseGenerator,
    config: &HeterogeneityConfig,
    world_x: f32,
    world_y: f32,
) -> (f32, f32) {
    let noise_value = noise.sample(world_x, world_y);

    let fuel_load = if config.fuel_variation_enabled {
        let multiplier = 1.0 + config.fuel_cv * noise_value;
        (base_fuel_load * multiplier).max(0.0)
    } else {
        base_fuel_load
    };

    let moisture = if config.moisture_variation_enabled {
        let aspect_factor = calculate_aspect_moisture_factor(aspect_degrees);
        let noise_multiplier = 1.0 + config.moisture_cv * noise_value;
        (base_moisture * noise_multiplier + aspect_factor).clamp(0.0, 1.0)
    } else {
        base_moisture
    };

    (fuel_load, moisture)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that mean fuel load is approximately preserved after variation.
    #[test]
    #[expect(clippy::cast_precision_loss)]
    fn fuel_variation_preserves_mean() {
        let noise = NoiseGenerator::new(12345);
        let config = HeterogeneityConfig::new(0.3, 0.0, 12345);

        let width = 100;
        let height = 100;
        let cell_size = 10.0;
        let base_fuel = 2.5; // kg/m²

        let mut fuel_load = vec![base_fuel; width * height];
        let mut moisture = vec![0.1; width * height];
        let aspect = vec![0.0; width * height];

        let original_sum: f32 = fuel_load.iter().sum();

        apply_fuel_heterogeneity(
            &mut fuel_load,
            &mut moisture,
            &aspect,
            &noise,
            &config,
            width as u32,
            height as u32,
            cell_size,
        );

        let modified_sum: f32 = fuel_load.iter().sum();
        let original_mean = original_sum / (width * height) as f32;
        let modified_mean = modified_sum / (width * height) as f32;

        // Mean should be approximately preserved (within 10%)
        let relative_diff = (modified_mean - original_mean).abs() / original_mean;
        assert!(
            relative_diff < 0.10,
            "Mean fuel load changed too much: original {original_mean}, modified {modified_mean}, diff {relative_diff}"
        );
    }

    /// Test that north-facing slopes are drier in Southern Hemisphere.
    #[test]
    fn aspect_moisture_factor_north_dry() {
        // North aspect (0°) should give negative (drier) factor
        let factor = calculate_aspect_moisture_factor(0.0);
        assert!(
            factor < 0.0,
            "North-facing slopes should be drier (negative factor), got {factor}"
        );
        assert!(
            (factor + 0.3).abs() < 0.01,
            "North-facing factor should be approximately -0.3, got {factor}"
        );
    }

    /// Test that south-facing slopes are wetter in Southern Hemisphere.
    #[test]
    fn aspect_moisture_factor_south_wet() {
        // South aspect (180°) should give positive (wetter) factor
        let factor = calculate_aspect_moisture_factor(180.0);
        assert!(
            factor > 0.0,
            "South-facing slopes should be wetter (positive factor), got {factor}"
        );
        assert!(
            (factor - 0.3).abs() < 0.01,
            "South-facing factor should be approximately +0.3, got {factor}"
        );
    }

    /// Test that east/west slopes have neutral moisture factor.
    #[test]
    fn aspect_moisture_factor_east_west_neutral() {
        let east_factor = calculate_aspect_moisture_factor(90.0);
        let west_factor = calculate_aspect_moisture_factor(270.0);

        assert!(
            east_factor.abs() < 0.01,
            "East-facing factor should be near zero, got {east_factor}"
        );
        assert!(
            west_factor.abs() < 0.01,
            "West-facing factor should be near zero, got {west_factor}"
        );
    }

    /// Test that disabled heterogeneity leaves fuel unchanged.
    #[test]
    #[expect(clippy::cast_precision_loss)]
    fn heterogeneity_disabled_no_change() {
        let noise = NoiseGenerator::new(999);
        let config = HeterogeneityConfig::disabled();

        let width = 50;
        let height = 50;
        let cell_size = 5.0;

        let original_fuel: Vec<f32> = (0..width * height).map(|i| i as f32 * 0.01).collect();
        let original_moisture: Vec<f32> = (0..width * height)
            .map(|i| (i as f32 * 0.001) % 1.0)
            .collect();

        let mut fuel_load = original_fuel.clone();
        let mut moisture = original_moisture.clone();
        let aspect = vec![45.0; width * height];

        apply_fuel_heterogeneity(
            &mut fuel_load,
            &mut moisture,
            &aspect,
            &noise,
            &config,
            width as u32,
            height as u32,
            cell_size,
        );

        // Values should be unchanged
        for (i, (orig, modified)) in original_fuel.iter().zip(fuel_load.iter()).enumerate() {
            assert!(
                (orig - modified).abs() < f32::EPSILON,
                "Fuel at index {i} changed when heterogeneity disabled: {orig} → {modified}"
            );
        }
        for (i, (orig, modified)) in original_moisture.iter().zip(moisture.iter()).enumerate() {
            assert!(
                (orig - modified).abs() < f32::EPSILON,
                "Moisture at index {i} changed when heterogeneity disabled: {orig} → {modified}"
            );
        }
    }

    /// Test that fuel load never goes negative after variation.
    #[test]
    #[expect(clippy::cast_precision_loss)]
    fn fuel_variation_within_bounds() {
        let noise = NoiseGenerator::new(77777);
        // Use maximum CV for stress testing
        let config = HeterogeneityConfig::new(1.0, 0.5, 77777);

        let width = 100;
        let height = 100;
        let cell_size = 10.0;

        // Use low base fuel to stress test negative prevention
        let mut fuel_load = vec![0.5; width * height];
        let mut moisture = vec![0.5; width * height];
        let aspect: Vec<f32> = (0..width * height)
            .map(|i| (i as f32 * 3.6) % 360.0)
            .collect();

        apply_fuel_heterogeneity(
            &mut fuel_load,
            &mut moisture,
            &aspect,
            &noise,
            &config,
            width as u32,
            height as u32,
            cell_size,
        );

        // Check all values are within valid bounds
        for (i, &fuel) in fuel_load.iter().enumerate() {
            assert!(fuel >= 0.0, "Fuel load at index {i} went negative: {fuel}");
        }
        for (i, &moist) in moisture.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&moist),
                "Moisture at index {i} out of range: {moist}"
            );
        }
    }

    /// Test that variation creates actual differences in the field.
    #[test]
    #[expect(clippy::cast_precision_loss)]
    fn fuel_variation_creates_differences() {
        let noise = NoiseGenerator::new(11111);
        let config = HeterogeneityConfig::new(0.3, 0.15, 11111);

        let width = 50;
        let height = 50;
        let cell_size = 10.0;
        let base_fuel = 2.0;

        let mut fuel_load = vec![base_fuel; width * height];
        let mut moisture = vec![0.2; width * height];
        let aspect: Vec<f32> = (0..width * height)
            .map(|i| (i as f32 * 7.2) % 360.0)
            .collect();

        apply_fuel_heterogeneity(
            &mut fuel_load,
            &mut moisture,
            &aspect,
            &noise,
            &config,
            width as u32,
            height as u32,
            cell_size,
        );

        // Count cells that differ from base value
        let fuel_different = fuel_load
            .iter()
            .filter(|&&f| (f - base_fuel).abs() > 0.01)
            .count();
        let moisture_different = moisture.iter().filter(|&&m| (m - 0.2).abs() > 0.01).count();

        // Most cells should have been modified
        assert!(
            fuel_different > (width * height) / 2,
            "Not enough fuel cells were modified: {fuel_different} out of {}",
            width * height
        );
        assert!(
            moisture_different > (width * height) / 2,
            "Not enough moisture cells were modified: {moisture_different} out of {}",
            width * height
        );
    }

    /// Test the single-cell convenience function.
    #[test]
    #[expect(clippy::cast_precision_loss)]
    fn single_cell_heterogeneity() {
        let noise = NoiseGenerator::new(55555);
        let config = HeterogeneityConfig::new(0.3, 0.2, 55555);

        let base_fuel = 2.0;
        let base_moisture = 0.15;

        // Test multiple positions
        for i in 0..10 {
            let x = i as f32 * 25.0;
            let y = i as f32 * 25.0;
            let aspect = (i as f32 * 36.0) % 360.0;

            let (fuel, moisture) =
                apply_heterogeneity_single(base_fuel, base_moisture, aspect, &noise, &config, x, y);

            assert!(fuel >= 0.0, "Single cell fuel went negative: {fuel}");
            assert!(
                (0.0..=1.0).contains(&moisture),
                "Single cell moisture out of range: {moisture}"
            );
        }
    }

    /// Test configuration constructors.
    #[test]
    fn config_constructors() {
        let fuel_only = HeterogeneityConfig::fuel_only(0.4, 123);
        assert!(fuel_only.fuel_variation_enabled);
        assert!(!fuel_only.moisture_variation_enabled);
        assert!((fuel_only.fuel_cv - 0.4).abs() < f32::EPSILON);

        let moisture_only = HeterogeneityConfig::moisture_only(0.25, 456);
        assert!(!moisture_only.fuel_variation_enabled);
        assert!(moisture_only.moisture_variation_enabled);
        assert!((moisture_only.moisture_cv - 0.25).abs() < f32::EPSILON);

        let disabled = HeterogeneityConfig::disabled();
        assert!(!disabled.fuel_variation_enabled);
        assert!(!disabled.moisture_variation_enabled);
    }

    /// Test that CV values are clamped to valid ranges.
    #[test]
    fn config_cv_clamping() {
        // Fuel CV should clamp to [0, 1]
        let config1 = HeterogeneityConfig::new(-0.5, 0.1, 0);
        assert!((config1.fuel_cv - 0.0).abs() < f32::EPSILON);

        let config2 = HeterogeneityConfig::new(1.5, 0.1, 0);
        assert!((config2.fuel_cv - 1.0).abs() < f32::EPSILON);

        // Moisture CV should clamp to [0, 0.5]
        let config3 = HeterogeneityConfig::new(0.3, -0.1, 0);
        assert!((config3.moisture_cv - 0.0).abs() < f32::EPSILON);

        let config4 = HeterogeneityConfig::new(0.3, 0.8, 0);
        assert!((config4.moisture_cv - 0.5).abs() < f32::EPSILON);
    }
}
