//! Valley channeling and chimney effect
//!
//! Detects valley geometry and applies fire behavior modifications including:
//! - Wind acceleration through narrow valleys
//! - Cross-valley radiant heat transfer
//! - Chimney effect updrafts at valley heads
//!
//! # Scientific Background
//!
//! Valleys create dangerous fire conditions through multiple mechanisms:
//! - Wind is funneled and accelerated through narrow valleys (1.5-2.5× acceleration)
//! - Heat radiating from both valley walls preheats fuel in the center
//! - Strong updrafts at valley head create "chimney effect"
//! - Fire can race up valleys at extreme speeds
//!
//! # Scientific References
//!
//! - Butler, B.W. et al. (1998). "Fire behavior associated with the 1994 South Canyon Fire."
//!   USDA Forest Service Research Paper RMRS-RP-9.
//! - Sharples, J.J. (2009). "An overview of mountain meteorological effects relevant to fire
//!   behaviour and bushfire risk." Int. J. Wildland Fire 18:737-754.

use crate::TerrainData;

/// Characteristic updraft velocity for valley chimney effect (m/s)
/// Empirical constant from Butler et al. (1998) representing the typical
/// updraft velocity scale observed in valley fires. This is used to normalize
/// actual updraft velocities for calculating spread rate enhancement.
pub const VALLEY_UPDRAFT_CHARACTERISTIC_VELOCITY: f32 = 50.0;

/// Ratio of valley depth to distance from valley head
/// Empirical constant derived from typical valley morphology ratios observed
/// in field studies by Butler et al. (1998). Typical valleys have head distance
/// approximately 10× their depth.
/// TODO: PHASE 10 - Valley Head Detection & Upstream Tracing
/// A full implementation would trace upstream from the valley point to find
/// the actual valley head via geomorphological analysis (watershed delineation).
const VALLEY_DEPTH_TO_HEAD_DISTANCE_RATIO: f32 = 10.0;

/// Valley geometry at a position
#[derive(Debug, Clone, Copy)]
pub struct ValleyGeometry {
    /// Valley width (m)
    pub width: f32,
    /// Valley depth (m)
    pub depth: f32,
    /// Valley orientation (radians)
    pub orientation: f32,
    /// Distance from valley head (m)
    pub distance_from_head: f32,
    /// Is this position in a valley?
    pub in_valley: bool,
}

impl Default for ValleyGeometry {
    fn default() -> Self {
        Self {
            width: 0.0,
            depth: 0.0,
            orientation: 0.0,
            distance_from_head: 0.0,
            in_valley: false,
        }
    }
}

/// Detect valley geometry from terrain
///
/// Uses radial sampling to determine if a position is surrounded by higher terrain
/// (indicating a valley). Calculates valley properties including width, depth, and orientation.
///
/// # Arguments
///
/// * `terrain` - Terrain data with elevation information
/// * `x` - X position in world coordinates (m)
/// * `y` - Y position in world coordinates (m)
/// * `sample_radius` - Radius to sample for valley detection (m)
///
/// # Returns
///
/// Valley geometry information for the position
pub fn detect_valley_geometry(
    terrain: &TerrainData,
    x: f32,
    y: f32,
    sample_radius: f32,
) -> ValleyGeometry {
    let center_elevation = *terrain.elevation_at(x, y);

    // Sample elevations in 8 directions
    let num_samples = 8;
    let mut elevations = Vec::with_capacity(num_samples);
    let mut directions = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        #[expect(clippy::cast_precision_loss)]
        let angle = (i as f32) * std::f32::consts::TAU / (num_samples as f32);
        let dx = angle.cos() * sample_radius;
        let dy = angle.sin() * sample_radius;

        let sample_x = x + dx;
        let sample_y = y + dy;
        let sample_elev = *terrain.elevation_at(sample_x, sample_y);

        elevations.push(sample_elev);
        directions.push(angle);
    }

    // Check if surrounded by higher terrain (valley condition)
    let num_higher = elevations
        .iter()
        .filter(|&&e| e > center_elevation + 5.0)
        .count();

    // Need at least 3 directions with higher terrain to consider it a valley
    let in_valley = num_higher >= 3;

    if !in_valley {
        return ValleyGeometry {
            in_valley: false,
            ..Default::default()
        };
    }

    // Find valley orientation (direction of lowest path out)
    let mut min_elevation = f32::MAX;
    let mut valley_direction = 0.0;

    for (angle, &elev) in directions.iter().zip(&elevations) {
        if elev < min_elevation {
            min_elevation = elev;
            valley_direction = *angle;
        }
    }

    // Estimate valley width (distance to ridges perpendicular to valley axis)
    let perpendicular = valley_direction + std::f32::consts::FRAC_PI_2;
    let mut width_samples = Vec::new();

    for offset in [-1.0, 1.0] {
        let dx = perpendicular.cos() * offset;
        let dy = perpendicular.sin() * offset;

        // Sample progressively further until we find higher terrain
        let mut distance = 0.0;
        let step = 10.0; // 10m steps
        while distance < sample_radius {
            distance += step;
            let sample_x = x + dx * distance;
            let sample_y = y + dy * distance;
            let sample_elev = *terrain.elevation_at(sample_x, sample_y);

            if sample_elev > center_elevation + 10.0 {
                width_samples.push(distance);
                break;
            }
        }
    }

    let width = if width_samples.len() == 2 {
        width_samples[0] + width_samples[1]
    } else if width_samples.len() == 1 {
        width_samples[0] * 2.0
    } else {
        // When ridge detection fails on both sides, we cannot reliably determine valley width.
        // Rather than use an arbitrary fallback (which would violate "NEVER SIMPLIFY PHYSICS"),
        // mark this location as not being in a valley. This is more scientifically honest:
        // if we can't detect valley walls, we shouldn't apply valley physics.
        return ValleyGeometry {
            in_valley: false,
            ..Default::default()
        };
    };

    // Estimate valley depth (difference between center and average ridge height)
    #[expect(clippy::cast_precision_loss)]
    let avg_ridge_elevation = elevations.iter().sum::<f32>() / elevations.len() as f32;
    let depth = (avg_ridge_elevation - center_elevation).max(0.0);

    // Estimate distance from valley head using empirical depth-to-distance ratio
    // VALLEY_DEPTH_TO_HEAD_DISTANCE_RATIO is documented above with scientific justification.
    // TODO: PHASE 10 for proper upstream valley head detection via terrain tracing.
    let distance_from_head = depth * VALLEY_DEPTH_TO_HEAD_DISTANCE_RATIO;

    ValleyGeometry {
        width,
        depth,
        orientation: valley_direction,
        distance_from_head,
        in_valley: true,
    }
}

/// Calculate wind acceleration in valley
///
/// Wind speed in valley: `U_valley = U_ambient × (W_open / W_valley)^0.5`
///
/// Acceleration typically ranges from 1.5-2.5×.
///
/// # Arguments
///
/// * `geometry` - Valley geometry information
/// * `reference_width` - Width of open terrain (m)
///
/// # Returns
///
/// Wind acceleration factor (1.0 = no acceleration, 2.5 = maximum)
pub fn valley_wind_factor(geometry: &ValleyGeometry, reference_width: f32) -> f32 {
    if !geometry.in_valley {
        return 1.0;
    }

    (reference_width / geometry.width).sqrt().clamp(1.0, 2.5)
}

/// Calculate chimney updraft velocity
///
/// Updraft velocity at valley head: `w = sqrt(2 × g × H × ΔT / T_ambient)`
///
/// Creates strong lofting effect when fire reaches valley head.
///
/// # Arguments
///
/// * `geometry` - Valley geometry information
/// * `fire_temperature` - Temperature of fire gases (°C)
/// * `ambient_temperature` - Ambient air temperature (°C)
/// * `head_distance_threshold` - Maximum distance from valley head for chimney effect (m)
///
/// # Returns
///
/// Updraft velocity (m/s), zero if not near valley head
pub fn chimney_updraft(
    geometry: &ValleyGeometry,
    fire_temperature: f32,
    ambient_temperature: f32,
    head_distance_threshold: f32,
) -> f32 {
    if !geometry.in_valley || geometry.distance_from_head > head_distance_threshold {
        return 0.0;
    }

    let delta_t = fire_temperature - ambient_temperature;
    if delta_t <= 0.0 {
        return 0.0;
    }

    const G: f32 = 9.81; // Gravity (m/s²)
    let t_kelvin = ambient_temperature + 273.15;

    (2.0 * G * geometry.depth * delta_t / t_kelvin).sqrt()
}

/// Calculate view factor for cross-valley radiant heat transfer
///
/// Exact view factor for opposing valley walls modeled as parallel infinite strips.
/// Significant when valley width < 100m.
///
/// Uses the Siegel-Howell formula for parallel infinite strips:
/// F₁₋₂ = √((D/H)² + 1) - (D/H)
///
/// Where:
/// - D = valley width (distance between opposing walls)
/// - H = valley depth (height of valley walls)
///
/// This is the exact analytical solution from radiative heat transfer theory
/// for opposing parallel surfaces (Siegel & Howell 2002).
///
/// Physical behavior:
/// - Narrower valleys (smaller D) → Higher view factor (walls "see" more of each other)
/// - Wider valleys (larger D) → Lower view factor (walls obscured by distance)
/// - As D→0: F→1 (perfect view of opposing wall)
/// - As D→∞: F→0 (no view of opposing wall)
///
/// # Arguments
///
/// * `valley_width` - Width of valley (m), distance D between opposing walls
/// * `valley_depth` - Depth of valley (m), height H of valley walls
///
/// # Returns
///
/// View factor (0.0 to 1.0)
///
/// # References
///
/// - Siegel, R. & Howell, J.R. (2002). Thermal Radiation Heat Transfer, 4th ed.
/// - Modest, M.F. (2013). Radiative Heat Transfer, 3rd ed. Academic Press.
pub fn cross_valley_view_factor(valley_width: f32, valley_depth: f32) -> f32 {
    if valley_width > 100.0 {
        return 0.0; // Negligible for wide valleys
    }

    // Prevent division by zero
    let depth = valley_depth.max(0.1);
    let width = valley_width.max(0.1);
    
    // Siegel-Howell formula for parallel infinite strips
    // F = √((D/H)² + 1) - (D/H)
    // where D is width (separation) and H is depth (height)
    let d_over_h = width / depth;
    let term = (d_over_h * d_over_h + 1.0).sqrt();
    
    term - d_over_h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::units::Meters;

    #[test]
    fn test_flat_terrain_not_valley() {
        let terrain = TerrainData::flat(
            Meters::new(200.0),
            Meters::new(200.0),
            Meters::new(10.0),
            Meters::new(100.0),
        );

        let geometry = detect_valley_geometry(&terrain, 100.0, 100.0, 50.0);

        assert!(!geometry.in_valley, "Flat terrain should not be a valley");
    }

    #[test]
    fn test_hill_creates_no_valley_at_peak() {
        let terrain = TerrainData::single_hill(
            Meters::new(200.0),
            Meters::new(200.0),
            Meters::new(10.0),
            Meters::new(0.0),
            Meters::new(50.0),
            Meters::new(50.0),
        );

        // At peak of hill (center)
        let geometry = detect_valley_geometry(&terrain, 100.0, 100.0, 50.0);

        assert!(
            !geometry.in_valley,
            "Peak of hill should not be detected as valley"
        );
    }

    #[test]
    fn test_valley_wind_factor_increases_in_narrow_valleys() {
        let narrow_valley = ValleyGeometry {
            width: 50.0,
            depth: 30.0,
            orientation: 0.0,
            distance_from_head: 50.0,
            in_valley: true,
        };

        let wide_valley = ValleyGeometry {
            width: 200.0,
            depth: 30.0,
            orientation: 0.0,
            distance_from_head: 50.0,
            in_valley: true,
        };

        let reference_width = 200.0;

        let narrow_factor = valley_wind_factor(&narrow_valley, reference_width);
        let wide_factor = valley_wind_factor(&wide_valley, reference_width);

        assert!(
            narrow_factor > wide_factor,
            "Narrow valleys should have higher wind acceleration"
        );
        assert!(
            (1.0..=2.5).contains(&narrow_factor),
            "Wind factor should be in valid range"
        );
    }

    #[test]
    fn test_chimney_updraft_only_near_valley_head() {
        let near_head = ValleyGeometry {
            width: 100.0,
            depth: 50.0,
            orientation: 0.0,
            distance_from_head: 50.0, // Within 100m threshold
            in_valley: true,
        };

        let far_from_head = ValleyGeometry {
            width: 100.0,
            depth: 50.0,
            orientation: 0.0,
            distance_from_head: 200.0, // Beyond threshold
            in_valley: true,
        };

        let fire_temp = 800.0;
        let ambient_temp = 25.0;
        let threshold = 100.0;

        let near_updraft = chimney_updraft(&near_head, fire_temp, ambient_temp, threshold);
        let far_updraft = chimney_updraft(&far_from_head, fire_temp, ambient_temp, threshold);

        assert!(near_updraft > 0.0, "Should have updraft near valley head");
        assert_eq!(far_updraft, 0.0, "Should have no updraft far from head");
    }

    #[test]
    fn test_chimney_updraft_requires_temperature_difference() {
        let geometry = ValleyGeometry {
            width: 100.0,
            depth: 50.0,
            orientation: 0.0,
            distance_from_head: 50.0,
            in_valley: true,
        };
        let threshold = 100.0;

        let no_fire = chimney_updraft(&geometry, 25.0, 25.0, threshold);

        assert_eq!(no_fire, 0.0, "No updraft without temperature difference");
    }

    #[test]
    fn test_cross_valley_view_factor_zero_for_wide_valleys() {
        let vf = cross_valley_view_factor(150.0, 50.0);

        assert_eq!(vf, 0.0, "Wide valleys (>100m) should have zero view factor");
    }

    #[test]
    fn test_cross_valley_view_factor_increases_for_narrow_valleys() {
        let narrow_vf = cross_valley_view_factor(30.0, 50.0);
        let wide_vf = cross_valley_view_factor(80.0, 50.0);

        assert!(
            narrow_vf > wide_vf,
            "Narrower valleys should have higher view factor"
        );
        assert!(
            (0.0..=1.0).contains(&narrow_vf),
            "View factor should be in valid range [0, 1], got {narrow_vf}"
        );
        assert!(
            (0.0..=1.0).contains(&wide_vf),
            "View factor should be in valid range [0, 1], got {wide_vf}"
        );
    }

    #[test]
    fn test_valley_wind_factor_no_acceleration_outside_valley() {
        let not_valley = ValleyGeometry {
            in_valley: false,
            ..Default::default()
        };

        let factor = valley_wind_factor(&not_valley, 200.0);

        assert_eq!(factor, 1.0, "No acceleration outside valley");
    }

    #[test]
    fn test_valley_wind_factor_clamped_to_max() {
        let very_narrow = ValleyGeometry {
            width: 10.0, // Very narrow
            depth: 50.0,
            orientation: 0.0,
            distance_from_head: 50.0,
            in_valley: true,
        };

        let factor = valley_wind_factor(&very_narrow, 1000.0);

        assert_eq!(
            factor, 2.5,
            "Wind factor should be clamped to maximum of 2.5"
        );
    }
}
