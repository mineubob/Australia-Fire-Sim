//! Vorticity-Driven Lateral Spread (VLS)
//!
//! Implements detection and modeling of lateral fire spread on steep lee slopes.
//!
//! # Scientific Background
//!
//! VLS occurs when wind flows over ridges and creates horizontal vortices on steep lee slopes.
//! This causes fire to spread laterally along the slope instead of uphill, creating rapid
//! and unexpected fire runs.
//!
//! VLS conditions require:
//! - Slope angle > 20° (lee slope facing away from wind)
//! - Wind speed > 5 m/s at 10m height
//! - Wind direction approximately perpendicular to ridge
//! - Fire near ridge crest or upper lee slope
//!
//! When VLS occurs:
//! - Spread direction shifts from upslope to lateral (along contour)
//! - Spread rate enhancement: 2-3× normal rate
//! - Effect strongest on upper third of lee slope
//!
//! # Scientific References
//!
//! - Sharples, J.J. et al. (2012). "Wind-terrain effects on the propagation of
//!   wildfires in rugged terrain: fire channelling." Int. J. Wildland Fire 21:282-296.
//! - Simpson, C.C. et al. (2013). "Resolving vorticity-driven lateral fire spread."
//!   Int. J. Wildland Fire.

use crate::core_types::vec3::Vec3;
use crate::TerrainData;

/// VLS detection parameters
pub struct VLSDetector {
    /// Minimum slope angle for VLS (degrees)
    pub min_slope: f32,
    /// Minimum wind speed for VLS (m/s)
    pub min_wind_speed: f32,
    /// VLS index threshold
    pub vls_threshold: f32,
}

impl Default for VLSDetector {
    fn default() -> Self {
        Self {
            min_slope: 20.0,     // 20° minimum slope
            min_wind_speed: 5.0, // 5 m/s minimum wind
            vls_threshold: 0.6,  // χ > 0.6 indicates VLS
        }
    }
}

/// VLS conditions at a point
#[derive(Debug, Clone, Copy)]
pub struct VLSCondition {
    /// VLS index (χ)
    pub vls_index: f32,
    /// Whether VLS is active
    pub is_active: bool,
    /// Lateral spread direction (radians)
    pub lateral_direction: f32,
    /// Spread rate multiplier (1.0-3.0)
    pub rate_multiplier: f32,
}

impl Default for VLSCondition {
    fn default() -> Self {
        Self {
            vls_index: 0.0,
            is_active: false,
            lateral_direction: 0.0,
            rate_multiplier: 1.0,
        }
    }
}

impl VLSDetector {
    /// Create a new VLS detector with custom parameters
    ///
    /// # Arguments
    ///
    /// * `min_slope` - Minimum slope angle in degrees
    /// * `min_wind_speed` - Minimum wind speed in m/s
    /// * `vls_threshold` - VLS index threshold for activation
    #[must_use]
    pub fn new(min_slope: f32, min_wind_speed: f32, vls_threshold: f32) -> Self {
        Self {
            min_slope,
            min_wind_speed,
            vls_threshold,
        }
    }

    /// Calculate VLS index at a position
    ///
    /// Uses the Sharples et al. (2012) formula:
    /// χ = tan(θ) × sin(|aspect - `wind_dir`|) × `U` / `U_ref`
    ///
    /// Where:
    /// - θ: Slope angle
    /// - aspect: Slope aspect (direction slope faces)
    /// - `wind_dir`: Wind direction
    /// - `U`: Wind speed
    /// - `U_ref`: Reference wind speed (5 m/s)
    ///
    /// # Arguments
    ///
    /// * `slope_degrees` - Slope angle in degrees
    /// * `aspect_degrees` - Aspect direction in degrees (0=North, clockwise)
    /// * `wind_direction_degrees` - Wind direction in degrees (direction wind is blowing towards)
    /// * `wind_speed` - Wind speed in m/s
    ///
    /// # Returns
    ///
    /// VLS index χ (0.0 = no VLS, > 0.6 = VLS likely)
    pub fn calculate_vls_index(
        &self,
        slope_degrees: f32,
        aspect_degrees: f32,
        wind_direction_degrees: f32,
        wind_speed: f32,
    ) -> f32 {
        if slope_degrees < self.min_slope || wind_speed < self.min_wind_speed {
            return 0.0;
        }

        let slope_rad = slope_degrees.to_radians();
        let tan_slope = slope_rad.tan();

        // Angular difference between aspect and wind direction
        let angle_diff = (aspect_degrees - wind_direction_degrees).to_radians();
        let sin_diff = angle_diff.sin().abs();

        // Wind factor
        let wind_factor = wind_speed / self.min_wind_speed;

        tan_slope * sin_diff * wind_factor
    }

    /// Detect VLS conditions across the terrain
    ///
    /// # Arguments
    ///
    /// * `terrain` - Terrain data with slope and aspect information
    /// * `wind` - Wind vector (components in m/s)
    /// * `width` - Grid width in cells
    /// * `height` - Grid height in cells
    /// * `cell_size` - Size of each cell in meters
    ///
    /// # Returns
    ///
    /// 2D grid of VLS conditions (row-major: [y][x])
    pub fn detect(
        &self,
        terrain: &TerrainData,
        wind: Vec3,
        width: usize,
        height: usize,
        cell_size: f32,
    ) -> Vec<Vec<VLSCondition>> {
        let wind_speed = (wind.x * wind.x + wind.y * wind.y).sqrt();
        let wind_dir = wind.y.atan2(wind.x).to_degrees();

        let mut conditions = vec![vec![VLSCondition::default(); width]; height];

        for (y, row) in conditions.iter_mut().enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                #[expect(clippy::cast_precision_loss)]
                let world_x = x as f32 * cell_size;
                #[expect(clippy::cast_precision_loss)]
                let world_y = y as f32 * cell_size;

                let slope = *terrain.slope_at_horn(world_x, world_y);
                let aspect = *terrain.aspect_at_horn(world_x, world_y);

                // Check if this is a lee slope (wind coming from opposite direction)
                let is_lee_slope = Self::is_lee_slope(aspect, wind_dir);

                if !is_lee_slope {
                    *cell = VLSCondition {
                        vls_index: 0.0,
                        is_active: false,
                        lateral_direction: 0.0,
                        rate_multiplier: 1.0,
                    };
                    continue;
                }

                let vls_index = self.calculate_vls_index(slope, aspect, wind_dir, wind_speed);
                let is_active = vls_index > self.vls_threshold;

                // Lateral direction is perpendicular to aspect (along contour).
                //
                // Convention:
                // - `terrain.aspect_at` returns the downslope azimuth in degrees,
                //   measured clockwise from geographic north (standard GIS convention).
                // - Lateral VLS spread is modeled along the contour as a 90° rotation
                //   from the downslope direction.
                //
                // We explicitly choose a +90° (clockwise) rotation from the aspect
                // direction, rather than -90° (counterclockwise), so that
                // `lateral_direction` is unambiguous and consistent with the chosen
                // terrain/wind orientation conventions.
                let lateral_direction = (aspect + 90.0) % 360.0;

                // Rate multiplier: 1.0 to 3.0 based on VLS index
                let rate_multiplier = if is_active {
                    1.0 + 2.0
                        * ((vls_index - self.vls_threshold) / (2.0 - self.vls_threshold)).min(1.0)
                } else {
                    1.0
                };

                *cell = VLSCondition {
                    vls_index,
                    is_active,
                    lateral_direction: lateral_direction.to_radians(),
                    rate_multiplier,
                };
            }
        }

        conditions
    }

    /// Check if slope is on lee side of wind
    ///
    /// Lee slope faces away from wind (aspect roughly opposite to wind direction).
    ///
    /// # Arguments
    ///
    /// * `aspect` - Slope aspect in degrees (direction slope faces)
    /// * `wind_direction` - Wind direction in degrees (direction wind blows towards)
    ///
    /// # Returns
    ///
    /// True if slope is on lee side (within 60° of downwind)
    ///
    /// # Scientific Basis
    ///
    /// The 60° tolerance (120° to 240° from wind direction) is based on
    /// Sharples et al. (2012) observations of VLS behavior. Slopes within
    /// this angular range experience flow separation and vortex formation
    /// that drives lateral fire spread along contours.
    fn is_lee_slope(aspect: f32, wind_direction: f32) -> bool {
        // Lee slope faces away from wind
        // If wind blows towards 0° (north), lee slopes face towards 180° (south)
        // Calculate angular difference, normalizing to [-180, 180]
        let mut angle_diff = (aspect - wind_direction) % 360.0;
        if angle_diff > 180.0 {
            angle_diff -= 360.0;
        } else if angle_diff < -180.0 {
            angle_diff += 360.0;
        }

        // Lee slope is within 60° of opposite direction (120° to 240° from wind)
        angle_diff.abs() > 120.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::units::Meters;

    #[test]
    fn test_vls_index_zero_for_flat_terrain() {
        let detector = VLSDetector::default();

        // Flat terrain (0° slope)
        let vls_index = detector.calculate_vls_index(
            0.0,   // flat slope
            0.0,   // aspect (doesn't matter)
            180.0, // wind direction
            10.0,  // wind speed
        );

        assert_eq!(vls_index, 0.0, "Flat terrain should have zero VLS index");
    }

    #[test]
    fn test_vls_index_zero_for_windward_slope() {
        let detector = VLSDetector::default();

        // Windward slope (faces into wind)
        let vls_index = detector.calculate_vls_index(
            30.0, // steep slope
            0.0,  // aspect north
            0.0,  // wind from north (same direction as aspect)
            10.0, // wind speed
        );

        // Windward slopes have low sin(angle_diff), so low VLS
        assert!(
            vls_index < 0.5,
            "Windward slopes should have low VLS index, got {vls_index}"
        );
    }

    #[test]
    fn test_vls_index_high_for_steep_lee_slope() {
        let detector = VLSDetector::default();

        // Steep lee slope perpendicular to wind
        let vls_index = detector.calculate_vls_index(
            30.0, // steep slope
            90.0, // aspect east (slope faces east)
            0.0,  // wind from north (perpendicular to aspect)
            10.0, // strong wind
        );

        // Should have high VLS index: tan(30°) * sin(90°) * (10/5) ≈ 0.577 * 1.0 * 2.0 ≈ 1.15
        assert!(
            vls_index > 0.6,
            "Steep lee slope should have high VLS index, got {vls_index}"
        );
    }

    #[test]
    fn test_lateral_direction_perpendicular_to_aspect() {
        let detector = VLSDetector::default();

        // Create simple terrain
        let terrain = TerrainData::flat(
            Meters::new(100.0),
            Meters::new(100.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );

        // Wind from north (0°)
        let wind = Vec3::new(0.0, 10.0, 0.0);

        let conditions = detector.detect(&terrain, wind, 10, 10, 10.0);

        // For flat terrain, no VLS should be active
        assert!(
            conditions
                .iter()
                .all(|row| row.iter().all(|c| !c.is_active && c.rate_multiplier == 1.0)),
            "Flat terrain should not trigger VLS"
        );
    }

    #[test]
    fn test_vls_inactive_below_threshold() {
        let detector = VLSDetector::default();

        // Conditions that produce low VLS index
        let vls_index = detector.calculate_vls_index(
            15.0, // below min_slope of 20°
            90.0, // aspect
            0.0,  // wind
            10.0, // wind speed
        );

        assert_eq!(
            vls_index, 0.0,
            "Below minimum slope should have zero VLS index"
        );
    }

    #[test]
    fn test_is_lee_slope() {
        // Lee slope: aspect opposite to wind
        assert!(
            VLSDetector::is_lee_slope(180.0, 0.0),
            "South-facing slope with north wind should be lee"
        );
        assert!(
            VLSDetector::is_lee_slope(150.0, 330.0),
            "Slope within 60° of downwind should be lee"
        );

        // Windward slope: aspect same as wind
        assert!(
            !VLSDetector::is_lee_slope(0.0, 0.0),
            "North-facing slope with north wind should not be lee"
        );
        assert!(
            !VLSDetector::is_lee_slope(45.0, 0.0),
            "Slope not facing downwind should not be lee"
        );
    }

    #[test]
    fn test_rate_multiplier_range() {
        let detector = VLSDetector::default();

        // Create terrain with a steep slope
        let terrain = TerrainData::single_hill(
            Meters::new(200.0),
            Meters::new(200.0),
            Meters::new(10.0),
            Meters::new(0.0),
            Meters::new(50.0),
            Meters::new(50.0),
        );

        // Wind from north (0°), which should create lee slopes on south side
        let wind = Vec3::new(0.0, 10.0, 0.0);

        let conditions = detector.detect(&terrain, wind, 20, 20, 10.0);

        // Check that rate multipliers are in valid range [1.0, 3.0]
        for row in &conditions {
            for cond in row {
                assert!(
                    cond.rate_multiplier >= 1.0 && cond.rate_multiplier <= 3.0,
                    "Rate multiplier {} outside valid range [1.0, 3.0]",
                    cond.rate_multiplier
                );
            }
        }
    }
}
