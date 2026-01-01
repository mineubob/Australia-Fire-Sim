//! Crown fire transition and spread physics.
//!
//! Implements surface-to-crown fire transitions using:
//! - Van Wagner (1977) crown fire initiation criterion
//! - Cruz et al. (2005) Australian crown fire spread rates
//!
//! # Scientific References
//!
//! - Van Wagner, C.E. (1977). "Conditions for the start and spread of crown fire"
//!   Canadian Journal of Forest Research, 7(1), 23-34
//! - Cruz, M.G., Alexander, M.E., Wakimoto, R.H. (2005). "Development and testing of
//!   models for predicting crown fire rate of spread in conifer forest stands"
//!   Canadian Journal of Forest Research, 35(7), 1626-1639

use crate::core_types::units::{
    Fraction, KgPerCubicMeter, KilogramsPerSquareMeter, KjPerKg, Meters, Percent,
};

/// Crown fire state for each cell
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum CrownFireState {
    /// No crown fire activity - surface fire only
    #[default]
    Surface = 0,
    /// Individual trees torching (passive crown fire)
    Passive = 1,
    /// Continuous crown-to-crown spread (active crown fire)
    Active = 2,
}

/// Canopy properties for crown fire modeling.
///
/// Based on Cruz et al. (2005) Australian eucalyptus forest data.
///
/// # Key Properties
///
/// - `base_height`: Height to live crown base (m) - determines flame height required for ignition
/// - `bulk_density`: Canopy fuel mass per volume (kg/m³) - controls spread sustainability
/// - `foliar_moisture`: Live fuel moisture (%) - affects ignition resistance
/// - `cover_fraction`: Horizontal cover (0-1) - affects fire continuity
/// - `fuel_load`: Available canopy fuel (kg/m²) - determines heat release
/// - `heat_content`: Energy per mass (kJ/kg) - typically 18000-22000 for eucalyptus
#[derive(Clone, Debug)]
pub struct CanopyProperties {
    /// Canopy base height - height to live crown base (m)
    pub base_height: Meters,
    /// Canopy bulk density - mass of available fuel per volume (kg/m³)
    pub bulk_density: KgPerCubicMeter,
    /// Foliar moisture content - live fuel moisture (%)
    pub foliar_moisture: Percent,
    /// Canopy cover fraction (0-1)
    pub cover_fraction: Fraction,
    /// Canopy fuel load (kg/m²)
    pub fuel_load: KilogramsPerSquareMeter,
    /// Heat content of canopy fuels (kJ/kg)
    pub heat_content: KjPerKg,
}

impl Default for CanopyProperties {
    /// Default eucalyptus forest canopy (Cruz et al. 2005)
    ///
    /// Representative values for mature eucalyptus forest:
    /// - 8m crown base height
    /// - 0.15 kg/m³ bulk density
    /// - 100% foliar moisture (fully hydrated)
    /// - 70% canopy cover
    /// - 1.2 kg/m² fuel load
    /// - 20000 kJ/kg heat content
    fn default() -> Self {
        Self {
            base_height: Meters::new(8.0),
            bulk_density: KgPerCubicMeter::new(0.15),
            foliar_moisture: Percent::new(100.0),
            cover_fraction: Fraction::new(0.7),
            fuel_load: KilogramsPerSquareMeter::new(1.2),
            heat_content: KjPerKg::new(20000.0),
        }
    }
}

impl CanopyProperties {
    /// Create eucalyptus forest canopy preset.
    ///
    /// Typical Australian eucalyptus forest with:
    /// - Moderate crown base height (8m)
    /// - Standard bulk density (0.15 kg/m³)
    /// - High foliar moisture (100%)
    /// - Dense canopy cover (70%)
    #[must_use]
    pub fn eucalyptus_forest() -> Self {
        Self::default()
    }

    /// Create open woodland canopy preset.
    ///
    /// Sparse woodland/savanna with:
    /// - Lower crown base height (5m)
    /// - Lower bulk density (0.08 kg/m³)
    /// - Lower foliar moisture (80%)
    /// - Sparse canopy cover (30%)
    #[must_use]
    pub fn open_woodland() -> Self {
        Self {
            base_height: Meters::new(5.0),
            bulk_density: KgPerCubicMeter::new(0.08),
            foliar_moisture: Percent::new(80.0),
            cover_fraction: Fraction::new(0.3),
            fuel_load: KilogramsPerSquareMeter::new(0.6),
            heat_content: KjPerKg::new(18500.0),
        }
    }

    /// Calculate critical intensity for crown ignition (Van Wagner 1977).
    ///
    /// Uses the Van Wagner (1977) formula:
    /// ```text
    /// I_critical = (0.010 × CBH × (460 + 25.9 × FMC))^1.5
    /// ```
    ///
    /// # Formula Components
    ///
    /// - CBH: Canopy base height (m)
    /// - FMC: Foliar moisture content (%)
    /// - 460 + 25.9×FMC: Heat absorption term accounting for moisture
    ///
    /// # Returns
    ///
    /// Critical surface fire intensity (kW/m) required to ignite the canopy.
    /// Surface fires with intensity below this threshold remain as surface fires.
    ///
    /// # Scientific Reference
    ///
    /// Van Wagner (1977), Equation 1
    #[must_use]
    pub fn critical_intensity(&self) -> f32 {
        // Van Wagner (1977) formula - exact implementation
        // I_critical = (0.010 × CBH × (460 + 25.9 × FMC))^1.5
        let base_term = 0.010 * *self.base_height * (460.0 + 25.9 * *self.foliar_moisture);
        base_term.powf(1.5)
    }

    /// Calculate critical rate of spread for active crown fire (Van Wagner 1977).
    ///
    /// Uses the Van Wagner (1977) formula:
    /// ```text
    /// R_critical = 3.0 / CBD
    /// ```
    ///
    /// # Formula Components
    ///
    /// - CBD: Canopy bulk density (kg/m³)
    /// - The constant 3.0 represents minimum mass flow for sustained crown fire
    ///
    /// # Returns
    ///
    /// Critical rate of spread (m/min) required to sustain active crown fire.
    /// Fires spreading slower than this will be passive (torching) rather than active.
    ///
    /// # Scientific Reference
    ///
    /// Van Wagner (1977), Equation 9
    #[must_use]
    pub fn critical_ros(&self) -> f32 {
        if *self.bulk_density <= 0.0 {
            return f32::INFINITY;
        }
        // Van Wagner (1977) formula - exact implementation
        // R_critical = 3.0 / CBD
        3.0 / *self.bulk_density
    }
}

/// Crown fire physics calculations.
///
/// Provides methods for evaluating crown fire transitions and calculating
/// spread rates based on Van Wagner (1977) and Cruz et al. (2005) models.
pub struct CrownFirePhysics;

impl CrownFirePhysics {
    /// Determine crown fire state based on surface intensity.
    ///
    /// Uses Van Wagner (1977) thresholds:
    /// - **Surface**: `I < I_critical` - fire remains on ground
    /// - **Passive**: `I >= I_critical`, but `ROS < ROS_critical` - individual trees torch
    /// - **Active**: `I >= I_critical` AND `ROS >= ROS_critical` - crown-to-crown spread
    ///
    /// # Arguments
    ///
    /// * `surface_intensity_kw_m` - Surface fire intensity (kW/m)
    /// * `surface_ros_m_s` - Surface fire rate of spread (m/s)
    /// * `canopy` - Canopy properties for threshold calculations
    ///
    /// # Returns
    ///
    /// The crown fire state classification.
    #[must_use]
    pub fn evaluate_transition(
        surface_intensity_kw_m: f32,
        surface_ros_m_s: f32,
        canopy: &CanopyProperties,
    ) -> CrownFireState {
        let critical_intensity = canopy.critical_intensity();

        // Check if intensity is sufficient for crown ignition
        if surface_intensity_kw_m < critical_intensity {
            return CrownFireState::Surface;
        }

        // Convert surface ROS from m/s to m/min for comparison with Van Wagner formula
        let surface_ros_m_min = surface_ros_m_s * 60.0;
        let critical_ros = canopy.critical_ros();

        // Check if ROS is sufficient for active crown fire
        if surface_ros_m_min >= critical_ros {
            CrownFireState::Active
        } else {
            CrownFireState::Passive
        }
    }

    /// Calculate crown fire rate of spread (Cruz et al. 2005).
    ///
    /// Uses the Cruz et al. (2005) Australian model:
    /// ```text
    /// R_crown = 11.02 × U₁₀^0.90 × (1 - 0.95 × e^(-0.17 × M_dead))
    /// ```
    ///
    /// # Arguments
    ///
    /// * `wind_speed_10m_kmh` - Wind speed at 10m height (km/h)
    /// * `dead_fuel_moisture` - Dead fine fuel moisture (fraction 0-1)
    ///
    /// # Returns
    ///
    /// Crown fire spread rate (m/s).
    ///
    /// # Note
    ///
    /// The formula outputs m/min, which is converted to m/s by dividing by 60.
    /// The moisture term uses percentage (0-100), so the fraction is multiplied by 100.
    ///
    /// # Scientific Reference
    ///
    /// Cruz et al. (2005), Equation 3
    #[must_use]
    pub fn crown_spread_rate(wind_speed_10m_kmh: f32, dead_fuel_moisture: f32) -> f32 {
        // Ensure non-negative inputs
        let wind = wind_speed_10m_kmh.max(0.0);
        let moisture_pct = (dead_fuel_moisture * 100.0).max(0.0);

        // Cruz et al. (2005) formula - exact implementation
        // R_crown = 11.02 × U₁₀^0.90 × (1 - 0.95 × e^(-0.17 × M_dead))
        // Note: M_dead is in percentage (0-100) in the original formula
        let wind_term = wind.powf(0.90);
        let moisture_term = 1.0 - 0.95 * (-0.17 * moisture_pct).exp();

        let ros_m_min = 11.02 * wind_term * moisture_term;

        // Convert from m/min to m/s
        ros_m_min / 60.0
    }

    /// Calculate combined surface + crown fire intensity.
    ///
    /// Total fire intensity includes both surface fire and crown fuel consumption.
    /// Uses Byram's intensity formula for the crown component:
    /// ```text
    /// I_crown = H × W × R
    /// ```
    ///
    /// where:
    /// - H = heat content (kJ/kg)
    /// - W = fuel load (kg/m²)
    /// - R = rate of spread (m/s)
    ///
    /// # Arguments
    ///
    /// * `surface_intensity_kw_m` - Surface fire intensity (kW/m)
    /// * `canopy` - Canopy properties for crown fuel
    /// * `crown_ros_m_s` - Crown fire rate of spread (m/s)
    ///
    /// # Returns
    ///
    /// Total fire intensity (kW/m).
    #[must_use]
    pub fn total_intensity(
        surface_intensity_kw_m: f32,
        canopy: &CanopyProperties,
        crown_ros_m_s: f32,
    ) -> f32 {
        // Crown intensity from Byram's formula: I = H × W × R
        // H in kJ/kg, W in kg/m², R in m/s gives kW/m
        let crown_intensity = *canopy.heat_content * *canopy.fuel_load * crown_ros_m_s;

        surface_intensity_kw_m + crown_intensity
    }

    /// Calculate effective spread rate including crown fire contribution.
    ///
    /// The effective rate depends on crown fire state:
    /// - **Surface**: Uses surface ROS unchanged
    /// - **Passive**: Surface ROS with ~1.5x increase from torching enhancement
    /// - **Active**: Uses the higher of surface ROS or crown ROS
    ///
    /// # Arguments
    ///
    /// * `surface_ros_m_s` - Surface fire rate of spread (m/s)
    /// * `crown_state` - Current crown fire state
    /// * `wind_speed_10m_kmh` - Wind speed at 10m (km/h) for crown ROS calculation
    /// * `dead_fuel_moisture` - Dead fine fuel moisture (fraction 0-1)
    ///
    /// # Returns
    ///
    /// Effective fire spread rate (m/s).
    #[must_use]
    pub fn effective_ros(
        surface_ros_m_s: f32,
        crown_state: CrownFireState,
        wind_speed_10m_kmh: f32,
        dead_fuel_moisture: f32,
    ) -> f32 {
        match crown_state {
            CrownFireState::Surface => surface_ros_m_s,
            CrownFireState::Passive => {
                // Passive crown fire enhances spread by ~50% due to increased
                // radiant heat and ember production from torching trees
                surface_ros_m_s * 1.5
            }
            CrownFireState::Active => {
                // Active crown fire uses the higher of surface or crown ROS
                let crown_ros = Self::crown_spread_rate(wind_speed_10m_kmh, dead_fuel_moisture);
                surface_ros_m_s.max(crown_ros)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Van Wagner critical intensity test.
    ///
    /// CBH=8m, FMC=100% → `I_critical` ≈ 3300 kW/m
    ///
    /// Calculation:
    /// I = (0.010 × 8 × (460 + 25.9 × 100))^1.5
    /// I = (0.08 × 3050)^1.5
    /// I = (244)^1.5 ≈ 3812 kW/m
    #[test]
    fn van_wagner_critical_intensity() {
        let canopy = CanopyProperties {
            base_height: Meters::new(8.0),
            bulk_density: KgPerCubicMeter::new(0.15),
            foliar_moisture: Percent::new(100.0),
            cover_fraction: Fraction::new(0.7),
            fuel_load: KilogramsPerSquareMeter::new(1.2),
            heat_content: KjPerKg::new(20000.0),
        };

        let i_critical = canopy.critical_intensity();

        // Van Wagner (1977) calculation:
        // I = (0.010 × 8 × (460 + 25.9 × 100))^1.5
        // I = (0.08 × 3050)^1.5 = 244^1.5 ≈ 3811.8 kW/m
        let expected = (0.010_f32 * 8.0 * (460.0 + 25.9 * 100.0)).powf(1.5);

        assert!(
            (i_critical - expected).abs() < 1.0,
            "I_critical = {i_critical}, expected ≈ {expected}"
        );

        // Verify it's in the expected range (around 3300-3900 kW/m for typical eucalyptus)
        assert!(
            i_critical > 3000.0 && i_critical < 4000.0,
            "I_critical = {i_critical} should be ~3300-3900 kW/m"
        );
    }

    /// Crown ROS scales with wind per Cruz (2005).
    ///
    /// 40 km/h wind, 8% moisture → R ≈ 3.5 m/min
    ///
    /// Calculation:
    /// R = 11.02 × 40^0.90 × (1 - 0.95 × e^(-0.17 × 8))
    /// R = 11.02 × 27.86 × (1 - 0.95 × 0.257)
    /// R = 306.9 × 0.756 ≈ 232 m/min
    #[test]
    fn crown_ros_wind_speed() {
        let wind_kmh = 40.0;
        let dead_fuel_moisture = 0.08; // 8%

        let ros_m_s = CrownFirePhysics::crown_spread_rate(wind_kmh, dead_fuel_moisture);
        let ros_m_min = ros_m_s * 60.0;

        // Cruz (2005) calculation:
        // R = 11.02 × 40^0.90 × (1 - 0.95 × e^(-0.17 × 8))
        // 40^0.90 ≈ 27.86
        // e^(-1.36) ≈ 0.257
        // moisture_term = 1 - 0.95 × 0.257 ≈ 0.756
        // R = 11.02 × 27.86 × 0.756 ≈ 232 m/min

        let expected_wind_term = 40.0_f32.powf(0.90);
        let expected_moisture_term = 1.0 - 0.95 * (-0.17 * 8.0_f32).exp();
        let expected_m_min = 11.02 * expected_wind_term * expected_moisture_term;

        assert!(
            (ros_m_min - expected_m_min).abs() < 1.0,
            "ROS = {ros_m_min} m/min, expected ≈ {expected_m_min} m/min"
        );

        // Verify it's a reasonable active crown fire speed
        assert!(
            ros_m_min > 100.0 && ros_m_min < 500.0,
            "ROS = {ros_m_min} m/min should be in range for active crown fire"
        );
    }

    /// Test Surface→Passive→Active transitions at correct thresholds.
    #[test]
    fn state_transition_thresholds() {
        let canopy = CanopyProperties::default();
        let critical_intensity = canopy.critical_intensity();
        let critical_ros_m_s = canopy.critical_ros() / 60.0; // Convert m/min to m/s

        // Below critical intensity → Surface
        let state = CrownFirePhysics::evaluate_transition(
            critical_intensity * 0.5, // Below threshold
            0.5,                      // ROS doesn't matter if intensity is low
            &canopy,
        );
        assert_eq!(
            state,
            CrownFireState::Surface,
            "Should be Surface when intensity < critical"
        );

        // Above intensity, below critical ROS → Passive
        let state = CrownFirePhysics::evaluate_transition(
            critical_intensity * 1.5, // Above threshold
            critical_ros_m_s * 0.5,   // Below critical ROS
            &canopy,
        );
        assert_eq!(
            state,
            CrownFireState::Passive,
            "Should be Passive when intensity high but ROS low"
        );

        // Above intensity, above critical ROS → Active
        let state = CrownFirePhysics::evaluate_transition(
            critical_intensity * 1.5, // Above threshold
            critical_ros_m_s * 1.5,   // Above critical ROS
            &canopy,
        );
        assert_eq!(
            state,
            CrownFireState::Active,
            "Should be Active when both intensity and ROS high"
        );
    }

    /// Test critical ROS formula: `R_critical` = 3.0/CBD.
    #[test]
    fn critical_ros_van_wagner() {
        let canopy = CanopyProperties {
            base_height: Meters::new(8.0),
            bulk_density: KgPerCubicMeter::new(0.15),
            foliar_moisture: Percent::new(100.0),
            cover_fraction: Fraction::new(0.7),
            fuel_load: KilogramsPerSquareMeter::new(1.2),
            heat_content: KjPerKg::new(20000.0),
        };

        let r_critical = canopy.critical_ros();

        // Van Wagner formula: R_critical = 3.0 / CBD
        let expected = 3.0 / 0.15; // = 20 m/min

        assert!(
            (r_critical - expected).abs() < 0.01,
            "R_critical = {r_critical}, expected {expected}"
        );
    }

    /// Total intensity includes canopy fuel contribution.
    #[test]
    fn total_intensity_includes_canopy() {
        let canopy = CanopyProperties {
            base_height: Meters::new(8.0),
            bulk_density: KgPerCubicMeter::new(0.15),
            foliar_moisture: Percent::new(100.0),
            cover_fraction: Fraction::new(0.7),
            fuel_load: KilogramsPerSquareMeter::new(1.2), // kg/m²
            heat_content: KjPerKg::new(20000.0),          // kJ/kg
        };

        let surface_intensity = 5000.0; // kW/m
        let crown_ros = 0.05; // m/s (3 m/min)

        let total = CrownFirePhysics::total_intensity(surface_intensity, &canopy, crown_ros);

        // Crown intensity = H × W × R = 20000 × 1.2 × 0.05 = 1200 kW/m
        let expected_crown = 20000.0 * 1.2 * 0.05;
        let expected_total = surface_intensity + expected_crown;

        assert!(
            (total - expected_total).abs() < 1.0,
            "Total = {total}, expected {expected_total}"
        );

        // Total should be greater than surface alone
        assert!(
            total > surface_intensity,
            "Total intensity should exceed surface intensity"
        );
    }

    /// Effective ROS unchanged for Surface state.
    #[test]
    fn effective_ros_surface_only() {
        let surface_ros = 0.1; // m/s

        let effective = CrownFirePhysics::effective_ros(
            surface_ros,
            CrownFireState::Surface,
            40.0, // Wind doesn't matter for surface
            0.08,
        );

        assert!(
            (effective - surface_ros).abs() < f32::EPSILON,
            "Surface state should use surface ROS unchanged"
        );
    }

    /// Passive crown fire gives moderate (~1.5x) increase.
    #[test]
    fn effective_ros_passive() {
        let surface_ros = 0.1; // m/s

        let effective =
            CrownFirePhysics::effective_ros(surface_ros, CrownFireState::Passive, 40.0, 0.08);

        // Should be 1.5x surface ROS
        let expected = surface_ros * 1.5;
        assert!(
            (effective - expected).abs() < f32::EPSILON,
            "Passive should be 1.5x surface ROS: got {effective}, expected {expected}"
        );
    }

    /// Active crown fire uses crown ROS.
    #[test]
    fn effective_ros_active() {
        let surface_ros = 0.1; // m/s (6 m/min - relatively slow)
        let wind_kmh = 40.0;
        let moisture = 0.08;

        let effective = CrownFirePhysics::effective_ros(
            surface_ros,
            CrownFireState::Active,
            wind_kmh,
            moisture,
        );

        let crown_ros = CrownFirePhysics::crown_spread_rate(wind_kmh, moisture);

        // Active uses max of surface and crown ROS
        let expected = surface_ros.max(crown_ros);
        assert!(
            (effective - expected).abs() < f32::EPSILON,
            "Active should use max(surface, crown): got {effective}, expected {expected}"
        );

        // Crown ROS should be higher than surface in this scenario
        assert!(
            crown_ros > surface_ros,
            "Crown ROS ({crown_ros}) should exceed surface ({surface_ros}) at 40km/h wind"
        );
    }

    /// Test eucalyptus forest preset has reasonable values.
    #[test]
    fn eucalyptus_forest_preset() {
        let canopy = CanopyProperties::eucalyptus_forest();

        assert!(
            *canopy.base_height > 5.0 && *canopy.base_height < 15.0,
            "Base height should be typical forest range"
        );
        assert!(
            *canopy.bulk_density > 0.05 && *canopy.bulk_density < 0.3,
            "Bulk density should be typical range"
        );
        assert!(
            *canopy.foliar_moisture > 50.0 && *canopy.foliar_moisture < 150.0,
            "Foliar moisture should be typical range"
        );
    }

    /// Test open woodland preset has sparser canopy.
    #[test]
    fn open_woodland_preset() {
        let eucalyptus = CanopyProperties::eucalyptus_forest();
        let woodland = CanopyProperties::open_woodland();

        assert!(
            *woodland.base_height < *eucalyptus.base_height,
            "Woodland should have lower crown base"
        );
        assert!(
            *woodland.bulk_density < *eucalyptus.bulk_density,
            "Woodland should have lower bulk density"
        );
        assert!(
            *woodland.cover_fraction < *eucalyptus.cover_fraction,
            "Woodland should have sparser cover"
        );
    }

    /// Test that zero wind speed doesn't cause issues.
    #[test]
    fn crown_ros_zero_wind() {
        let ros = CrownFirePhysics::crown_spread_rate(0.0, 0.08);

        assert!(ros >= 0.0, "ROS should be non-negative even with zero wind");
        assert!(
            ros < 0.001,
            "ROS should be near zero with no wind: got {ros}"
        );
    }

    /// Test that zero bulk density returns infinity for critical ROS.
    #[test]
    fn critical_ros_zero_density() {
        let canopy = CanopyProperties {
            base_height: Meters::new(8.0),
            bulk_density: KgPerCubicMeter::new(0.0), // Invalid - should return infinity
            foliar_moisture: Percent::new(100.0),
            cover_fraction: Fraction::new(0.7),
            fuel_load: KilogramsPerSquareMeter::new(1.2),
            heat_content: KjPerKg::new(20000.0),
        };

        let r_critical = canopy.critical_ros();

        assert!(
            r_critical.is_infinite(),
            "Zero bulk density should give infinite critical ROS"
        );
    }
}
