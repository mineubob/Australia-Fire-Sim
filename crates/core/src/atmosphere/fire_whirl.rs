//! Fire whirl formation detection.
//!
//! Implements vorticity-based detection of fire whirl formation conditions
//! based on Clark et al. (1996) coupled atmosphere-fire modeling.
//!
//! # Scientific Background
//!
//! Fire whirls (fire devils, fire tornadoes) form when:
//! 1. Strong horizontal wind shear exists
//! 2. High convective intensity from the fire
//! 3. Terrain or fuel configuration creates vorticity concentration
//!
//! They represent extremely dangerous fire behavior with localized
//! wind speeds exceeding 100 km/h and unpredictable movement.
//!
//! # References
//!
//! - Clark, T.L. et al. (1996). "Coupled atmosphere-fire model simulations."
//!   International Journal of Wildland Fire.

use crate::core_types::units::{Meters, MetersPerSecond, RatePerSecond};

/// Fire whirl detection parameters.
///
/// Detects conditions favorable for fire whirl formation based on
/// local vorticity and fire intensity.
#[derive(Clone, Debug)]
pub struct FireWhirlDetector {
    /// Vorticity threshold for whirl formation (1/s).
    ///
    /// Typical threshold is 0.1-0.5 s⁻¹ for fire whirls.
    pub vorticity_threshold: RatePerSecond,

    /// Minimum intensity for whirl formation (kW/m).
    ///
    /// Fire whirls require significant buoyant forcing,
    /// typically above 10,000 kW/m.
    pub intensity_threshold_kw_m: f32,
}

impl Default for FireWhirlDetector {
    fn default() -> Self {
        Self {
            vorticity_threshold: RatePerSecond::new(0.2), // s⁻¹
            intensity_threshold_kw_m: 10_000.0,           // kW/m
        }
    }
}

impl FireWhirlDetector {
    /// Create a new detector with custom thresholds.
    ///
    /// # Arguments
    ///
    /// * `vorticity_threshold` - Vorticity threshold (1/s)
    /// * `intensity_threshold_kw_m` - Intensity threshold (kW/m)
    #[must_use]
    pub fn new(vorticity_threshold: RatePerSecond, intensity_threshold_kw_m: f32) -> Self {
        Self {
            vorticity_threshold,
            intensity_threshold_kw_m,
        }
    }

    /// Calculate vorticity from wind field derivatives.
    ///
    /// Vertical vorticity (rotation about vertical axis):
    /// ```text
    /// ω = ∂v/∂x - ∂u/∂y
    /// ```
    ///
    /// Approximated using central differences:
    /// ```text
    /// ω ≈ (v_right - v_left)/(2×dx) - (u_up - u_down)/(2×dy)
    /// ```
    ///
    /// # Arguments
    ///
    /// * `u_up` - u-component of wind at (x, y+dy)
    /// * `u_down` - u-component of wind at (x, y-dy)
    /// * `v_left` - v-component of wind at (x-dx, y)
    /// * `v_right` - v-component of wind at (x+dx, y)
    /// * `cell_size` - Grid cell size in meters (dx = dy)
    ///
    /// # Returns
    ///
    /// Vertical vorticity in s⁻¹ (positive = counterclockwise)
    #[must_use]
    pub fn calculate_vorticity(
        u_up: MetersPerSecond,
        u_down: MetersPerSecond,
        v_left: MetersPerSecond,
        v_right: MetersPerSecond,
        cell_size: Meters,
    ) -> RatePerSecond {
        let dv_dx = (v_right - v_left) / (2.0 * *cell_size);
        let du_dy = (u_up - u_down) / (2.0 * *cell_size);

        RatePerSecond::new(*dv_dx - *du_dy)
    }

    /// Check if conditions support fire whirl formation.
    ///
    /// Requires both high vorticity magnitude and high fire intensity.
    ///
    /// # Arguments
    ///
    /// * `vorticity` - Local vorticity (1/s)
    /// * `intensity_kw_m` - Local fire intensity (kW/m)
    ///
    /// # Returns
    ///
    /// True if fire whirl conditions are met
    #[must_use]
    pub fn check_conditions(&self, vorticity: RatePerSecond, intensity_kw_m: f32) -> bool {
        vorticity.abs() > *self.vorticity_threshold
            && intensity_kw_m > self.intensity_threshold_kw_m
    }

    /// Calculate fire whirl intensity index (0-1).
    ///
    /// Combines vorticity and fire intensity into a single metric
    /// for visualization or risk assessment.
    ///
    /// # Arguments
    ///
    /// * `vorticity` - Local vorticity (1/s)
    /// * `intensity_kw_m` - Local fire intensity (kW/m)
    ///
    /// # Returns
    ///
    /// Fire whirl index (0.0 = no risk, 1.0 = maximum risk)
    #[must_use]
    pub fn intensity_index(&self, vorticity: RatePerSecond, intensity_kw_m: f32) -> f32 {
        let vort_factor = (vorticity.abs() / *self.vorticity_threshold).min(2.0) / 2.0;
        let int_factor = (intensity_kw_m / self.intensity_threshold_kw_m).min(5.0) / 5.0;

        (vort_factor * int_factor).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test vorticity calculation for known shear.
    #[test]
    fn vorticity_calculation() {
        // Pure shear: v increasing with x
        // v_left = 0, v_right = 10, u uniform
        let vorticity = FireWhirlDetector::calculate_vorticity(
            MetersPerSecond::new(5.0),  // u_up
            MetersPerSecond::new(5.0),  // u_down (uniform)
            MetersPerSecond::new(0.0),  // v_left
            MetersPerSecond::new(10.0), // v_right (shear)
            Meters::new(10.0),          // cell_size
        );

        // Expected: dv/dx = 10/20 = 0.5, du/dy = 0
        assert!(
            (*vorticity - 0.5).abs() < 0.01,
            "Vorticity should be 0.5, got {vorticity}"
        );
    }

    /// Test vorticity for counter-rotating shear.
    #[test]
    fn vorticity_opposite_shear() {
        // u increasing with y (du/dy positive), v uniform
        let vorticity = FireWhirlDetector::calculate_vorticity(
            MetersPerSecond::new(10.0), // u_up (shear)
            MetersPerSecond::new(0.0),  // u_down
            MetersPerSecond::new(5.0),  // v_left (uniform)
            MetersPerSecond::new(5.0),  // v_right
            Meters::new(10.0),          // cell_size
        );

        // Expected: dv/dx = 0, du/dy = 10/20 = 0.5
        // ω = -du/dy = -0.5
        assert!(
            (*vorticity + 0.5).abs() < 0.01,
            "Vorticity should be -0.5, got {vorticity}"
        );
    }

    /// Test fire whirl condition detection.
    #[test]
    fn vorticity_threshold() {
        let detector = FireWhirlDetector::default();

        // Below threshold
        assert!(!detector.check_conditions(RatePerSecond::new(0.1), 5_000.0));

        // Above threshold (both conditions met)
        assert!(detector.check_conditions(RatePerSecond::new(0.5), 20_000.0));

        // High vorticity but low intensity
        assert!(!detector.check_conditions(RatePerSecond::new(0.5), 5_000.0));

        // High intensity but low vorticity
        assert!(!detector.check_conditions(RatePerSecond::new(0.1), 50_000.0));
    }

    /// Test intensity index calculation.
    #[test]
    fn intensity_index_range() {
        let detector = FireWhirlDetector::default();

        // No risk
        let low = detector.intensity_index(RatePerSecond::new(0.0), 0.0);
        assert!(low < 0.01, "Low index should be near 0: {low}");

        // High risk
        let high = detector.intensity_index(RatePerSecond::new(0.5), 50_000.0);
        assert!(high > 0.5, "High index should be > 0.5: {high}");
        assert!(high <= 1.0, "Index should be <= 1.0: {high}");
    }
}
