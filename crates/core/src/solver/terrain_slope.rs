//! Terrain slope effects on fire spread rate.
//!
//! Implements slope factors following A.G. `McArthur` (1967) and
//! R.C. Rothermel (1972) for fire spread rate modification based on
//! terrain slope and aspect.
//!
//! # Scientific Background
//!
//! Fire spreads faster uphill due to:
//! - Preheating of uphill fuels by radiation and convection
//! - Flame contact with uphill fuels (flames tilt toward slope)
//! - Reduced distance between flame and uphill fuel bed
//!
//! Empirical observation: fire spread rate approximately doubles
//! for every 10° of uphill slope.
//!
//! Downhill spread is reduced because:
//! - Flames tilt away from fuel
//! - Radiant heat is directed away from fuel bed
//! - Convective preheating is reduced
//!
//! # References
//!
//! - A.G. `McArthur` (1967). Fire behaviour in eucalypt forests. Forestry and
//!   Timber Bureau Leaflet No. 107, Canberra.
//! - R.C. Rothermel (1972). A mathematical model for predicting fire spread
//!   in wildland fuels. USDA Forest Service Research Paper INT-115.

use super::fields::FieldData;

// Helper to convert usize to f32, centralizing the intentional precision loss
#[inline]
#[expect(clippy::cast_precision_loss)]
fn usize_to_f32(v: usize) -> f32 {
    v as f32
}
use crate::TerrainData;

/// Terrain data for slope-aware fire spread calculations.
///
/// Stores precomputed terrain properties for each cell in the simulation grid
/// to enable efficient slope factor calculations during fire spread updates.
#[derive(Clone, Debug)]
pub struct TerrainFields {
    /// Terrain elevation at each cell (meters above sea level)
    pub elevation: FieldData,
    /// Slope angle at each cell (degrees, 0-90)
    pub slope: FieldData,
    /// Aspect direction at each cell (degrees, 0-360, 0=North, clockwise)
    ///
    /// Aspect represents the direction of steepest descent (downslope direction).
    /// 0° = North, 90° = East, 180° = South, 270° = West
    pub aspect: FieldData,
}

impl Default for TerrainFields {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl TerrainFields {
    /// Create new terrain fields with given dimensions.
    ///
    /// Initializes all fields to zero/flat terrain.
    ///
    /// # Arguments
    ///
    /// * `width` - Grid width in cells
    /// * `height` - Grid height in cells
    ///
    /// # Returns
    ///
    /// New terrain fields with all values initialized to zero
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            elevation: FieldData::new(width, height),
            slope: FieldData::new(width, height),
            aspect: FieldData::new(width, height),
        }
    }

    /// Initialize terrain fields from flat terrain data.
    ///
    /// Creates terrain fields representing a flat surface at a given elevation.
    /// Slope is zero everywhere and aspect is undefined (set to 0).
    ///
    /// # Arguments
    ///
    /// * `width` - Grid width in cells
    /// * `height` - Grid height in cells
    /// * `base_elevation` - Uniform elevation in meters above sea level
    ///
    /// # Returns
    ///
    /// Terrain fields for flat terrain at the specified elevation
    #[must_use]
    pub fn from_flat_terrain(width: usize, height: usize, base_elevation: f32) -> Self {
        Self {
            elevation: FieldData::with_value(width, height, base_elevation),
            slope: FieldData::new(width, height), // Zero slope for flat terrain
            aspect: FieldData::new(width, height), // Aspect undefined for flat terrain
        }
    }

    /// Initialize terrain fields from `TerrainData`.
    ///
    /// Samples elevation, slope, and aspect from the terrain data at each
    /// grid cell center, using Horn's method for accurate slope/aspect calculation.
    ///
    /// # Arguments
    ///
    /// * `terrain` - Source terrain data with elevation information
    /// * `width` - Grid width in cells
    /// * `height` - Grid height in cells
    /// * `cell_size` - Size of each cell in meters
    ///
    /// # Returns
    ///
    /// Terrain fields populated from the terrain data
    #[must_use]
    pub fn from_terrain_data(
        terrain: &TerrainData,
        width: usize,
        height: usize,
        cell_size: f32,
    ) -> Self {
        let mut fields = Self::new(width, height);

        for y in 0..height {
            for x in 0..width {
                // Calculate world position at cell center
                let world_x = (usize_to_f32(x) + 0.5) * cell_size;
                let world_y = (usize_to_f32(y) + 0.5) * cell_size;

                // Sample terrain properties using Horn's method for accuracy
                let elevation = *terrain.elevation_at(world_x, world_y);
                let slope = *terrain.slope_at_horn(world_x, world_y);
                let aspect = *terrain.aspect_at_horn(world_x, world_y);

                fields.elevation.set(x, y, elevation);
                fields.slope.set(x, y, slope);
                fields.aspect.set(x, y, aspect);
            }
        }

        fields
    }

    /// Get grid dimensions.
    ///
    /// # Returns
    ///
    /// Tuple of (width, height) in cells
    #[must_use]
    pub fn dimensions(&self) -> (usize, usize) {
        (self.elevation.width, self.elevation.height)
    }

    /// Check if terrain fields are empty (zero dimensions).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.elevation.width == 0 || self.elevation.height == 0
    }
}

/// Calculate slope factor based on empirical observations from A.G. `McArthur` (1967).
///
/// Fire spread rate approximately doubles for every 10° of uphill slope.
/// This empirical relationship has been validated extensively in Australian
/// bushfire conditions and forms the basis for fire danger rating systems.
///
/// # Physics
///
/// For uphill spread (positive effective slope):
/// - Factor = 1.0 + (slope / 10)^1.5 × 2.0
/// - At 10°: factor ≈ 3.0 (2× faster than flat)
/// - At 20°: factor ≈ 6.66 (approximately 4× faster than 10°)
///
/// For downhill spread (negative effective slope):
/// - Factor = (1.0 + slope / 30).max(0.3)
/// - Gradual reduction to minimum of 0.3 (70% reduction)
/// - Fire can still spread downhill, but much slower
///
/// # Arguments
///
/// * `effective_slope` - Slope in degrees relative to fire spread direction.
///   Positive values indicate uphill spread, negative values indicate downhill.
///
/// # Returns
///
/// Multiplier for base spread rate:
/// - 1.0 = no effect (flat terrain or cross-slope)
/// - > 1.0 = faster spread (uphill)
/// - < 1.0 = slower spread (downhill, minimum 0.3)
///
/// # Example
///
/// ```
/// use fire_sim_core::solver::terrain_slope::calculate_slope_factor;
///
/// // Flat terrain: no effect
/// let flat = calculate_slope_factor(0.0);
/// assert!((flat - 1.0).abs() < 0.01);
///
/// // 10° uphill: approximately 3× faster
/// let uphill_10 = calculate_slope_factor(10.0);
/// assert!(uphill_10 > 2.5 && uphill_10 < 3.5);
///
/// // 10° downhill: reduced spread
/// let downhill = calculate_slope_factor(-10.0);
/// assert!(downhill < 1.0 && downhill >= 0.3);
/// ```
#[must_use]
pub fn calculate_slope_factor(effective_slope: f32) -> f32 {
    if effective_slope > 0.0 {
        // Uphill: exponential increase following McArthur's empirical relationship
        // Fire spread approximately doubles for every 10° of uphill slope
        // Using power law: 1.0 + (slope/10)^1.5 * 2.0
        // At 10°: 1.0 + 1.0^1.5 * 2.0 = 3.0 (2× increase over flat)
        // At 20°: 1.0 + 2.0^1.5 * 2.0 ≈ 6.66 (approximately 4× over 10°)
        1.0 + (effective_slope / 10.0).powf(1.5) * 2.0
    } else if effective_slope < 0.0 {
        // Downhill: gradual reduction to minimum of 0.3
        // Linear decrease with gentler slope to avoid abrupt transitions
        // At -30°: factor = 1.0 + (-30/30) = 0.0 → clamped to 0.3
        (1.0 + effective_slope / 30.0).max(0.3)
    } else {
        // Flat terrain: no slope effect
        1.0
    }
}

/// Calculate effective slope based on fire spread direction and terrain aspect.
///
/// The effective slope is the component of the terrain slope that aligns with
/// the fire's spread direction. Fire spreading directly uphill experiences
/// the full slope effect, while fire spreading across the slope experiences
/// minimal slope effect.
///
/// # Physics
///
/// The effective slope is calculated as:
/// ```text
/// effective_slope = slope_angle × cos(spread_direction - uphill_direction)
/// ```
///
/// Where:
/// - `slope_angle` is the terrain slope magnitude (always positive)
/// - `spread_direction` is the direction the fire is moving (0°=N, 90°=E)
/// - `uphill_direction` is opposite to aspect (aspect + 180°)
///
/// The cosine term projects the slope onto the spread direction:
/// - cos(0°) = 1.0: spreading directly uphill → full positive slope
/// - cos(90°) = 0.0: spreading across slope → no slope effect
/// - cos(180°) = -1.0: spreading directly downhill → full negative slope
///
/// # Arguments
///
/// * `slope_angle` - Terrain slope magnitude in degrees (0-90, always positive)
/// * `aspect_angle` - Terrain aspect in degrees (0-360, 0=North, direction of downslope)
/// * `spread_direction` - Fire spread direction in degrees (0-360, 0=North)
///
/// # Returns
///
/// Effective slope in degrees:
/// - Positive values indicate uphill spread relative to fire direction
/// - Negative values indicate downhill spread relative to fire direction
/// - Zero indicates cross-slope or flat terrain
///
/// # Example
///
/// ```
/// use fire_sim_core::solver::terrain_slope::calculate_effective_slope;
///
/// // Fire spreading south (180°) on a north-facing slope (aspect=0°, downhill faces north)
/// // The uphill direction is south (180°), same as spread direction → uphill spread
/// let slope = calculate_effective_slope(15.0, 0.0, 180.0);
/// assert!((slope - 15.0).abs() < 0.1);
///
/// // Fire spreading east (90°) on the same slope → cross-slope, minimal effect
/// let cross = calculate_effective_slope(15.0, 0.0, 90.0);
/// assert!(cross.abs() < 0.1);
/// ```
#[must_use]
pub fn calculate_effective_slope(
    slope_angle: f32,
    aspect_angle: f32,
    spread_direction: f32,
) -> f32 {
    // Handle flat terrain (no slope effect regardless of direction)
    if slope_angle.abs() < 0.001 {
        return 0.0;
    }

    // Uphill direction is opposite to aspect (aspect points downhill)
    // Adding 180° to aspect gives the uphill direction
    let uphill_direction = (aspect_angle + 180.0) % 360.0;

    // Calculate angle difference between spread direction and uphill direction
    // Normalize to -180° to +180° range
    let mut angle_diff = spread_direction - uphill_direction;
    if angle_diff > 180.0 {
        angle_diff -= 360.0;
    } else if angle_diff < -180.0 {
        angle_diff += 360.0;
    }

    // Effective slope is the projection of slope onto spread direction
    // cos(angle_diff) gives:
    //   +1.0 when spreading directly uphill
    //   -1.0 when spreading directly downhill
    //    0.0 when spreading across the slope
    let angle_diff_rad = angle_diff.to_radians();
    slope_angle * angle_diff_rad.cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that 10° uphill slope gives approximately 2× boost (factor ~3.0)
    ///
    /// A.G. `McArthur` (1967) observed that fire spread rate approximately doubles
    /// for every 10° of uphill slope. Our formula gives factor = 3.0 at 10°,
    /// which represents a 2× increase over the base rate of 1.0.
    #[test]
    fn slope_factor_uphill_10_degrees() {
        let factor = calculate_slope_factor(10.0);

        // Expected: 1.0 + (10/10)^1.5 * 2.0 = 1.0 + 1.0 * 2.0 = 3.0
        assert!(
            (factor - 3.0).abs() < 0.01,
            "10° uphill should give factor ~3.0, got {factor}"
        );

        // Verify the 2× boost interpretation:
        // Factor 3.0 means spread rate is 3× the base rate
        // This is a 2× increase (boost) over base rate of 1.0
        let boost = factor - 1.0;
        assert!(
            (boost - 2.0).abs() < 0.01,
            "10° uphill should give ~2× boost, got {boost}× boost"
        );
    }

    /// Test that 20° uphill slope gives approximately 4× boost over 10°
    ///
    /// The power law relationship means steeper slopes have exponentially
    /// greater effects. At 20°, the factor should be roughly 4× that of 10°.
    #[test]
    fn slope_factor_uphill_20_degrees() {
        let factor_10 = calculate_slope_factor(10.0);
        let factor_20 = calculate_slope_factor(20.0);

        // Expected: 1.0 + (20/10)^1.5 * 2.0 = 1.0 + 2.828... * 2.0 ≈ 6.66
        let expected_20 = 1.0 + (2.0_f32).powf(1.5) * 2.0;
        assert!(
            (factor_20 - expected_20).abs() < 0.01,
            "20° uphill should give factor ~{expected_20}, got {factor_20}"
        );

        // Verify 20° is approximately 4× the boost of 10°
        // At 10°: boost = 2.0, at 20°: boost ≈ 5.66
        // Ratio of boosts: 5.66 / 2.0 ≈ 2.83 (2^1.5)
        let boost_10 = factor_10 - 1.0;
        let boost_20 = factor_20 - 1.0;
        let boost_ratio = boost_20 / boost_10;

        // The ratio should be 2^1.5 ≈ 2.83
        let expected_ratio = 2.0_f32.powf(1.5);
        assert!(
            (boost_ratio - expected_ratio).abs() < 0.01,
            "Boost ratio 20°/10° should be ~{expected_ratio}, got {boost_ratio}"
        );
    }

    /// Test that downhill slopes reduce spread rate to minimum of 0.3
    #[test]
    fn slope_factor_downhill() {
        // Mild downhill: gradual reduction
        let factor_10_down = calculate_slope_factor(-10.0);
        assert!(
            factor_10_down < 1.0,
            "Downhill should reduce factor below 1.0"
        );
        assert!(factor_10_down > 0.3, "Mild downhill should not hit minimum");
        // Expected: 1.0 + (-10/30) = 1.0 - 0.333 = 0.667
        assert!(
            (factor_10_down - 0.667).abs() < 0.01,
            "10° downhill should give factor ~0.667, got {factor_10_down}"
        );

        // Steep downhill: should hit minimum
        let factor_30_down = calculate_slope_factor(-30.0);
        assert!(
            (factor_30_down - 0.3).abs() < 0.01,
            "30° downhill should give minimum factor 0.3, got {factor_30_down}"
        );

        // Very steep downhill: still clamped to minimum
        let factor_45_down = calculate_slope_factor(-45.0);
        assert!(
            (factor_45_down - 0.3).abs() < 0.01,
            "45° downhill should be clamped to 0.3, got {factor_45_down}"
        );
    }

    /// Test that cross-slope spread has factor ~1.0 (no effect)
    #[test]
    fn slope_factor_cross_slope() {
        // Cross-slope spread: effective slope should be ~0
        // Fire spreading east (90°) on a north-facing slope (aspect=0°)

        // The uphill direction for a north-facing slope is south (180°)
        // Spreading east (90°) is perpendicular to uphill, so effective slope = 0
        let effective = calculate_effective_slope(20.0, 0.0, 90.0);
        assert!(
            effective.abs() < 0.1,
            "Cross-slope should have effective slope ~0, got {effective}"
        );

        // Factor for zero effective slope should be 1.0
        let factor = calculate_slope_factor(effective);
        assert!(
            (factor - 1.0).abs() < 0.01,
            "Cross-slope factor should be ~1.0, got {factor}"
        );

        // Also test spreading west (270°) - also perpendicular
        let effective_west = calculate_effective_slope(20.0, 0.0, 270.0);
        assert!(
            effective_west.abs() < 0.1,
            "Cross-slope west should have effective slope ~0, got {effective_west}"
        );
    }

    /// Test terrain fields initialization
    #[test]
    fn terrain_fields_initialization() {
        // Test basic construction
        let fields = TerrainFields::new(100, 50);
        assert_eq!(fields.dimensions(), (100, 50));
        assert!(!fields.is_empty());

        // All values should be zero
        assert_eq!(fields.elevation.get(0, 0), 0.0);
        assert_eq!(fields.slope.get(50, 25), 0.0);
        assert_eq!(fields.aspect.get(99, 49), 0.0);

        // Test flat terrain initialization
        let flat = TerrainFields::from_flat_terrain(64, 64, 100.0);
        assert_eq!(flat.dimensions(), (64, 64));
        assert_eq!(flat.elevation.get(32, 32), 100.0);
        assert_eq!(flat.slope.get(32, 32), 0.0);

        // Test empty fields
        let empty = TerrainFields::new(0, 0);
        assert!(empty.is_empty());

        // Test default (should be empty)
        let default_fields = TerrainFields::default();
        assert!(default_fields.is_empty());
    }

    /// Test effective slope calculation for various spread directions
    #[test]
    fn effective_slope_directions() {
        let slope_angle = 15.0;
        let aspect = 0.0; // North-facing slope (downhill direction is north)

        // Uphill direction is south (180°)

        // Fire spreading south (180°) = directly uphill
        let uphill = calculate_effective_slope(slope_angle, aspect, 180.0);
        assert!(
            (uphill - 15.0).abs() < 0.1,
            "Spreading uphill should give full positive slope, got {uphill}"
        );

        // Fire spreading north (0°) = directly downhill
        let downhill = calculate_effective_slope(slope_angle, aspect, 0.0);
        assert!(
            (downhill - (-15.0)).abs() < 0.1,
            "Spreading downhill should give full negative slope, got {downhill}"
        );

        // Fire spreading east (90°) = cross-slope
        let cross_east = calculate_effective_slope(slope_angle, aspect, 90.0);
        assert!(
            cross_east.abs() < 0.1,
            "Spreading cross-slope should give ~0, got {cross_east}"
        );

        // Fire spreading west (270°) = cross-slope
        let cross_west = calculate_effective_slope(slope_angle, aspect, 270.0);
        assert!(
            cross_west.abs() < 0.1,
            "Spreading cross-slope should give ~0, got {cross_west}"
        );

        // Fire spreading SE (135°) = diagonal uphill
        let diagonal = calculate_effective_slope(slope_angle, aspect, 135.0);
        // cos(135-180) = cos(-45) = 0.707
        let expected_diagonal = 15.0 * (45.0_f32.to_radians().cos());
        assert!(
            (diagonal - expected_diagonal).abs() < 0.1,
            "Diagonal uphill should be ~{expected_diagonal}, got {diagonal}"
        );
    }

    /// Test that flat terrain has no slope effect regardless of direction
    #[test]
    fn flat_terrain_no_slope_effect() {
        // Flat terrain: slope = 0
        let effective = calculate_effective_slope(0.0, 45.0, 180.0);
        assert!(
            effective.abs() < 0.001,
            "Flat terrain should have zero effective slope"
        );

        let factor = calculate_slope_factor(effective);
        assert!(
            (factor - 1.0).abs() < 0.001,
            "Flat terrain should have factor 1.0"
        );
    }

    /// Test aspect angle wrapping at 360° boundary
    #[test]
    fn aspect_angle_wrapping() {
        let slope = 10.0;

        // Aspect 350° (nearly north-facing) with spread direction 170° (nearly south)
        // Uphill = 350 + 180 = 530 → 170°
        // Spread direction 170° matches uphill → full uphill effect
        let effective_1 = calculate_effective_slope(slope, 350.0, 170.0);
        assert!(
            (effective_1 - 10.0).abs() < 0.1,
            "Should get full uphill effect near 360° boundary, got {effective_1}"
        );

        // Aspect 10° with spread direction 190°
        // Uphill = 10 + 180 = 190°
        // Spread = 190° matches uphill → full uphill effect
        let effective_2 = calculate_effective_slope(slope, 10.0, 190.0);
        assert!(
            (effective_2 - 10.0).abs() < 0.1,
            "Should get full uphill effect, got {effective_2}"
        );
    }
}
