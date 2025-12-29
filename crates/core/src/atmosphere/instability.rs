//! Atmospheric stability indices for fire weather.
//!
//! Implements Haines Index and other atmospheric stability measures used
//! to predict extreme fire behavior potential including pyroCb formation.
//!
//! # Scientific Background
//!
//! Atmospheric instability promotes strong convection, which enhances
//! fire spread rates, ember transport, and pyroCb development. The Haines
//! Index combines stability and moisture terms to rate fire weather severity.
//!
//! # References
//!
//! - Haines, D.A. (1988). "A lower atmosphere severity index for wildlife fires."
//!   National Weather Digest, 13(2), 23-27.

/// Atmospheric stability indices for fire weather assessment.
///
/// Provides multiple stability metrics used to assess extreme fire behavior
/// potential, including pyroCb formation likelihood.
#[derive(Clone, Debug, Default)]
pub struct AtmosphericStability {
    /// Haines Index (2-6, higher = more unstable).
    ///
    /// - 2-3: Very low to low fire growth potential
    /// - 4: Moderate fire growth potential
    /// - 5-6: High to very high fire growth potential
    pub haines_index: u8,

    /// Continuous Haines Index for finer resolution (2.0-6.0).
    pub c_haines: f32,

    /// Mixing height (m) - depth of unstable boundary layer.
    pub mixing_height: f32,
}

impl AtmosphericStability {
    /// Calculate Haines Index from temperature profile (low-level formula).
    ///
    /// The Haines Index combines two terms:
    /// - **Stability term (A)**: Based on lapse rate between 950 and 850 hPa
    /// - **Moisture term (B)**: Based on dew point depression at 850 hPa
    ///
    /// ```text
    /// Stability term A:
    /// - A = 1 if (T_950 - T_850) < 4°C
    /// - A = 2 if 4 ≤ (T_950 - T_850) < 8°C
    /// - A = 3 if (T_950 - T_850) ≥ 8°C
    ///
    /// Moisture term B:
    /// - B = 1 if (T_850 - Td_850) < 6°C
    /// - B = 2 if 6 ≤ (T_850 - Td_850) < 10°C
    /// - B = 3 if (T_850 - Td_850) ≥ 10°C
    ///
    /// Haines Index = A + B (range 2-6)
    /// ```
    ///
    /// # Arguments
    ///
    /// * `t_950_c` - Temperature at 950 hPa (~500m altitude) in °C
    /// * `t_850_c` - Temperature at 850 hPa (~1500m altitude) in °C
    /// * `td_850_c` - Dew point temperature at 850 hPa in °C
    ///
    /// # Returns
    ///
    /// Haines Index (2-6)
    #[must_use]
    pub fn haines_index(t_950_c: f32, t_850_c: f32, td_850_c: f32) -> u8 {
        // Stability term based on lapse rate
        let lapse = t_950_c - t_850_c;
        let stability_term = if lapse < 4.0 {
            1
        } else if lapse < 8.0 {
            2
        } else {
            3
        };

        // Moisture term based on dew point depression
        let depression = t_850_c - td_850_c;
        let moisture_term = if depression < 6.0 {
            1
        } else if depression < 10.0 {
            2
        } else {
            3
        };

        stability_term + moisture_term
    }

    /// Calculate continuous Haines Index for finer resolution.
    ///
    /// Uses linear interpolation within the discrete Haines categories
    /// to provide a smoother metric for modeling purposes.
    ///
    /// # Arguments
    ///
    /// * `t_950_c` - Temperature at 950 hPa in °C
    /// * `t_850_c` - Temperature at 850 hPa in °C
    /// * `td_850_c` - Dew point temperature at 850 hPa in °C
    ///
    /// # Returns
    ///
    /// Continuous Haines Index (2.0-6.0)
    #[must_use]
    pub fn continuous_haines(t_950_c: f32, t_850_c: f32, td_850_c: f32) -> f32 {
        let lapse = t_950_c - t_850_c;
        let depression = t_850_c - td_850_c;

        // Continuous stability term (1.0-3.0)
        let stability = if lapse < 4.0 {
            1.0 + lapse / 4.0
        } else if lapse < 8.0 {
            2.0 + (lapse - 4.0) / 4.0
        } else {
            3.0_f32.min(2.5 + (lapse - 8.0) / 8.0)
        };

        // Continuous moisture term (1.0-3.0)
        let moisture = if depression < 6.0 {
            1.0 + depression / 6.0
        } else if depression < 10.0 {
            2.0 + (depression - 6.0) / 4.0
        } else {
            3.0_f32.min(2.5 + (depression - 10.0) / 8.0)
        };

        (stability + moisture).clamp(2.0, 6.0)
    }

    /// Create a new stability assessment from sounding data.
    ///
    /// # Arguments
    ///
    /// * `t_950_c` - Temperature at 950 hPa in °C
    /// * `t_850_c` - Temperature at 850 hPa in °C
    /// * `td_850_c` - Dew point temperature at 850 hPa in °C
    /// * `mixing_height` - Boundary layer mixing height in meters
    #[must_use]
    pub fn new(t_950_c: f32, t_850_c: f32, td_850_c: f32, mixing_height: f32) -> Self {
        Self {
            haines_index: Self::haines_index(t_950_c, t_850_c, td_850_c),
            c_haines: Self::continuous_haines(t_950_c, t_850_c, td_850_c),
            mixing_height,
        }
    }

    /// Estimate likelihood of pyroCb development (0-1).
    ///
    /// `PyroCb` formation requires both high atmospheric instability
    /// and high fire intensity. This function combines both factors.
    ///
    /// # Arguments
    ///
    /// * `fire_intensity_kw_m` - Fire line intensity in kW/m
    ///
    /// # Returns
    ///
    /// Probability estimate (0.0-1.0)
    #[must_use]
    pub fn pyrocb_potential(&self, fire_intensity_kw_m: f32) -> f32 {
        // Haines factor: scales from 0 at HI=2 to 1 at HI=6
        let haines_factor = (f32::from(self.haines_index) - 2.0) / 4.0;

        // Intensity factor: threshold around 50,000 kW/m
        let intensity_factor = (fire_intensity_kw_m / 50_000.0).min(1.0);

        // Combined probability (multiplicative)
        haines_factor * intensity_factor
    }

    /// Check if conditions are favorable for extreme fire behavior.
    ///
    /// Returns true if Haines Index >= 5.
    #[must_use]
    pub fn is_extreme(&self) -> bool {
        self.haines_index >= 5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test Haines Index calculation for known profiles.
    #[test]
    fn haines_index_calculation() {
        // Low instability case: small lapse, moist
        let low = AtmosphericStability::haines_index(15.0, 12.0, 10.0);
        assert_eq!(low, 2, "Low instability case should give HI=2");

        // Moderate case: moderate lapse, moderate dryness
        let moderate = AtmosphericStability::haines_index(20.0, 14.0, 8.0);
        assert_eq!(moderate, 4, "Moderate case should give HI=4");

        // High instability case: large lapse, very dry
        let high = AtmosphericStability::haines_index(25.0, 15.0, 3.0);
        assert_eq!(high, 6, "High instability case should give HI=6");
    }

    /// Test Haines Index is always in valid range 2-6.
    #[test]
    fn haines_index_range() {
        // Various edge cases
        let cases = [
            (0.0, 0.0, 0.0),
            (30.0, 10.0, -10.0),
            (15.0, 15.0, 15.0),
            (50.0, 0.0, -50.0),
        ];

        for (t950, t850, td850) in cases {
            let hi = AtmosphericStability::haines_index(t950, t850, td850);
            assert!(
                (2..=6).contains(&hi),
                "Haines Index {hi} should be in range 2-6 for ({t950}, {t850}, {td850})"
            );
        }
    }

    /// Test continuous Haines is in valid range.
    #[test]
    fn continuous_haines_range() {
        let ch = AtmosphericStability::continuous_haines(20.0, 14.0, 5.0);
        assert!(
            (2.0..=6.0).contains(&ch),
            "Continuous Haines {ch} should be in range 2-6"
        );
    }

    /// Test pyroCb potential calculation.
    #[test]
    fn pyrocb_potential() {
        // Low Haines, low intensity - no potential
        let stable = AtmosphericStability::new(15.0, 12.0, 10.0, 1000.0);
        assert!(stable.pyrocb_potential(10_000.0) < 0.1);

        // High Haines, high intensity - high potential
        let unstable = AtmosphericStability::new(25.0, 15.0, 3.0, 3000.0);
        assert!(unstable.pyrocb_potential(100_000.0) > 0.8);
    }

    /// Test extreme conditions detection.
    #[test]
    fn extreme_conditions() {
        let moderate = AtmosphericStability::new(18.0, 12.0, 4.0, 1500.0);
        assert!(!moderate.is_extreme());

        let extreme = AtmosphericStability::new(25.0, 15.0, 0.0, 3000.0);
        assert!(extreme.is_extreme());
    }
}
