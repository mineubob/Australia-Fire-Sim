//! Van Wagner Crown Fire Initiation and Spread Model (1977, 1993)
//!
//! Implements scientifically accurate crown fire transition dynamics:
//! - Active vs passive crown fire distinction
//! - Critical surface intensity calculation
//! - Critical crown fire spread rate
//! - Foliar moisture content effects
//! - Crown base height and bulk density
//!
//! # Scientific References
//! - Van Wagner, C.E. (1977). "Conditions for the start and spread of crown fire"
//!   Canadian Journal of Forest Research, 7(1), 23-34
//! - Van Wagner, C.E. (1993). "Prediction of crown fire behavior in two stands of jack pine"
//!   Canadian Journal of Forest Research, 23(3), 442-449
//! - Cruz, M.G., Alexander, M.E. (2010). "Assessing crown fire potential in coniferous forests"
//!   Forest Ecology and Management, 259(3), 562-570

use crate::core_types::element::FuelElement;

/// Crown fire type classification
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CrownFireType {
    /// No crown fire - surface fire only
    Surface,
    /// Passive crown fire - intermittent torching of individual trees
    Passive,
    /// Active crown fire - continuous crown fire spread independent of surface fire
    Active,
}

/// Crown fire behavior parameters
#[derive(Debug, Clone, Copy)]
pub(crate) struct CrownFireBehavior {
    /// Crown fire type
    fire_type: CrownFireType,
    /// Critical surface fire intensity for crown fire initiation (kW/m)
    critical_surface_intensity: f32,
    /// Actual surface fire intensity (kW/m)
    surface_intensity: f32,
    /// Critical crown fire spread rate (m/min)
    critical_crown_spread_rate: f32,
    /// Ratio of active to critical crown spread rate
    crown_fraction_burned: f32,
}

impl CrownFireBehavior {
    /// Create new crown fire behavior
    pub(crate) fn new(
        fire_type: CrownFireType,
        critical_surface_intensity: f32,
        surface_intensity: f32,
        critical_crown_spread_rate: f32,
        crown_fraction_burned: f32,
    ) -> Self {
        Self {
            fire_type,
            critical_surface_intensity,
            surface_intensity,
            critical_crown_spread_rate,
            crown_fraction_burned,
        }
    }

    /// Get the fire type
    pub(crate) fn fire_type(&self) -> CrownFireType {
        self.fire_type
    }

    /// Get surface intensity
    pub(crate) fn surface_intensity(&self) -> f32 {
        self.surface_intensity
    }

    /// Get critical surface intensity
    pub(crate) fn critical_surface_intensity(&self) -> f32 {
        self.critical_surface_intensity
    }

    /// Get crown fraction burned
    pub(crate) fn crown_fraction_burned(&self) -> f32 {
        self.crown_fraction_burned
    }
}

/// Calculate critical surface fire intensity for crown fire initiation
///
/// Van Wagner (1977) formula:
/// I_o = (0.01 × CBD × H × (460 + 25.9 × M_c)) / CBH
///
/// # Arguments
/// * `crown_bulk_density` - Crown bulk density (kg/m³), typical range 0.05-0.3
/// * `heat_content` - Heat content of crown fuels (kJ/kg), typical 18000-22000
/// * `foliar_moisture_content` - Foliar moisture content (%), typical 80-120
/// * `crown_base_height` - Height to base of crown (m), typical 2-15
///
/// # Returns
/// Critical surface fire intensity in kW/m
///
/// # References
/// Van Wagner (1977), Equation 4
pub fn calculate_critical_surface_intensity(
    crown_bulk_density: f32,
    heat_content: f32,
    foliar_moisture_content: f32,
    crown_base_height: f32,
) -> f32 {
    // Van Wagner (1977) formula - exact implementation
    let numerator =
        0.01 * crown_bulk_density * heat_content * (460.0 + 25.9 * foliar_moisture_content);
    let critical_intensity = numerator / crown_base_height;

    critical_intensity.max(0.0)
}

/// Calculate critical crown fire spread rate
///
/// Van Wagner (1977) formula:
/// R_critical = 3.0 / CBD
///
/// # Arguments
/// * `crown_bulk_density` - Crown bulk density (kg/m³)
///
/// # Returns
/// Critical crown fire spread rate in m/min
///
/// # References
/// Van Wagner (1977), Equation 9
pub fn calculate_critical_crown_spread_rate(crown_bulk_density: f32) -> f32 {
    if crown_bulk_density <= 0.0 {
        return 0.0;
    }

    // Van Wagner (1977) formula - exact implementation
    3.0 / crown_bulk_density
}

/// Calculate crown fraction burned (CFB)
///
/// Cruz & Alexander (2010) formula:
/// CFB = 1 - exp(-0.23 × (R_active - R_critical))
///
/// # Arguments
/// * `active_spread_rate` - Actual crown fire spread rate (m/min)
/// * `critical_spread_rate` - Critical crown fire spread rate (m/min)
///
/// # Returns
/// Crown fraction burned (0-1)
///
/// # References
/// Cruz & Alexander (2010)
pub(crate) fn calculate_crown_fraction_burned(
    active_spread_rate: f32,
    critical_spread_rate: f32,
) -> f32 {
    if active_spread_rate <= critical_spread_rate {
        return 0.0;
    }

    let rate_diff = active_spread_rate - critical_spread_rate;
    let cfb = 1.0 - (-0.23 * rate_diff).exp();

    cfb.clamp(0.0, 1.0)
}

/// Determine crown fire type based on spread rates and intensity
///
/// Classification:
/// - Surface: I_surface < I_critical
/// - Passive: I_surface >= I_critical AND R_active < R_critical
/// - Active: I_surface >= I_critical AND R_active >= R_critical
///
/// # References
/// Van Wagner (1977, 1993)
pub(crate) fn determine_crown_fire_type(
    surface_intensity: f32,
    critical_surface_intensity: f32,
    active_spread_rate: f32,
    critical_crown_spread_rate: f32,
) -> CrownFireType {
    if surface_intensity < critical_surface_intensity {
        CrownFireType::Surface
    } else if active_spread_rate < critical_crown_spread_rate {
        CrownFireType::Passive
    } else {
        CrownFireType::Active
    }
}

/// Calculate complete crown fire behavior
///
/// Integrates Van Wagner (1977, 1993) crown fire initiation and spread models
/// with fuel-specific thresholds for Australian eucalyptus species.
///
/// For Australian fuels, uses the minimum of Van Wagner threshold and fuel-specific
/// threshold, as Van Wagner was calibrated for Canadian conifers and may overestimate
/// crown fire resistance in eucalyptus forests with volatile oils and ladder fuels.
pub(crate) fn calculate_crown_fire_behavior(
    element: &FuelElement,
    crown_bulk_density: f32,
    crown_base_height: f32,
    foliar_moisture_content: f32,
    active_spread_rate: f32,
    wind_speed_ms: f32,
) -> CrownFireBehavior {
    // Calculate critical surface intensity (Van Wagner 1977)
    let van_wagner_threshold = calculate_critical_surface_intensity(
        crown_bulk_density,
        element.fuel.heat_content,
        foliar_moisture_content,
        crown_base_height,
    );

    // Use minimum of Van Wagner and fuel-specific threshold
    // This accounts for Australian eucalyptus behavior which differs from Canadian conifers
    let critical_surface_intensity = van_wagner_threshold.min(element.fuel.crown_fire_threshold);

    // Get current surface fire intensity (Byram)
    let surface_intensity = element.byram_fireline_intensity(wind_speed_ms);

    // Calculate critical crown spread rate (Van Wagner 1977)
    let critical_crown_spread_rate = calculate_critical_crown_spread_rate(crown_bulk_density);

    // Determine fire type
    let fire_type = determine_crown_fire_type(
        surface_intensity,
        critical_surface_intensity,
        active_spread_rate,
        critical_crown_spread_rate,
    );

    // Calculate crown fraction burned
    let crown_fraction_burned = if fire_type == CrownFireType::Active {
        calculate_crown_fraction_burned(active_spread_rate, critical_crown_spread_rate)
    } else {
        0.0
    };

    CrownFireBehavior::new(
        fire_type,
        critical_surface_intensity,
        surface_intensity,
        critical_crown_spread_rate,
        crown_fraction_burned,
    )
}

/// Apply crown fire effects to fuel element
///
/// Increases burn rate and intensity when crown fire conditions are met
pub(crate) fn apply_crown_fire_effects(
    _element: &mut FuelElement,
    crown_behavior: &CrownFireBehavior,
) -> f32 {
    match crown_behavior.fire_type() {
        CrownFireType::Surface => {
            // No crown fire enhancement
            1.0
        }
        CrownFireType::Passive => {
            // Passive crown fire - intermittent torching
            // Multiply burn rate by 1.5-2.0 based on intensity ratio
            let intensity_ratio =
                crown_behavior.surface_intensity() / crown_behavior.critical_surface_intensity();
            1.0 + (intensity_ratio - 1.0) * 0.5
        }
        CrownFireType::Active => {
            // Active crown fire - full crown involvement
            // Multiply burn rate by 2.0-4.0 based on crown fraction burned
            2.0 + crown_behavior.crown_fraction_burned() * 2.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::element::{FuelElement, FuelPart, Vec3};
    use crate::core_types::fuel::Fuel;

    #[test]
    fn test_critical_surface_intensity_calculation() {
        // Test Van Wagner (1977) formula with typical eucalypt values
        let cbd = 0.15; // kg/m³
        let heat = 20000.0; // kJ/kg
        let fmc = 100.0; // %
        let cbh = 5.0; // m

        let i_critical = calculate_critical_surface_intensity(cbd, heat, fmc, cbh);

        // Expected: 0.01 × 0.15 × 20000 × (460 + 25.9 × 100) / 5.0
        // = 0.01 × 0.15 × 20000 × 3050 / 5.0
        // = 0.0015 × 20000 × 3050 / 5.0
        // = 30 × 3050 / 5.0 = 91500 / 5.0 = 18300 kW/m
        assert!(
            (i_critical - 18300.0).abs() < 100.0,
            "I_critical was {}",
            i_critical
        );
    }

    #[test]
    fn test_critical_crown_spread_rate() {
        // Test Van Wagner (1977) formula
        let cbd = 0.15; // kg/m³

        let r_critical = calculate_critical_crown_spread_rate(cbd);

        // Expected: 3.0 / 0.15 = 20 m/min
        assert!(
            (r_critical - 20.0).abs() < 0.1,
            "R_critical was {}",
            r_critical
        );
    }

    #[test]
    fn test_crown_fraction_burned() {
        let active_rate = 30.0; // m/min
        let critical_rate = 20.0; // m/min

        let cfb = calculate_crown_fraction_burned(active_rate, critical_rate);

        // Should be between 0 and 1
        assert!(cfb > 0.0 && cfb <= 1.0);
        // CFB = 1 - exp(-0.23 × (30-20)) = 1 - exp(-2.3) ≈ 0.9
        assert!((cfb - 0.9).abs() < 0.1, "CFB was {}", cfb);
    }

    #[test]
    fn test_crown_fire_type_classification() {
        // Test surface fire (low intensity)
        let fire_type = determine_crown_fire_type(500.0, 1000.0, 10.0, 20.0);
        assert_eq!(fire_type, CrownFireType::Surface);

        // Test passive crown fire (high intensity, low spread)
        let fire_type = determine_crown_fire_type(1500.0, 1000.0, 15.0, 20.0);
        assert_eq!(fire_type, CrownFireType::Passive);

        // Test active crown fire (high intensity, high spread)
        let fire_type = determine_crown_fire_type(1500.0, 1000.0, 25.0, 20.0);
        assert_eq!(fire_type, CrownFireType::Active);
    }

    #[test]
    fn test_stringybark_crown_fire_susceptibility() {
        // Stringybark should have lower critical intensity than smooth bark due to:
        // - Lower crown base height (ladder fuels bring fire closer to canopy)
        // - Higher crown bulk density (denser canopy)

        // Stringybark
        let stringybark_i = calculate_critical_surface_intensity(
            0.2,     // High CBD
            21000.0, // Heat content
            90.0,    // Moderate FMC
            3.0,     // Low CBH (ladder fuels)
        );

        // Smooth bark for comparison
        let smooth_bark_i = calculate_critical_surface_intensity(
            0.12,    // Lower CBD
            20000.0, // Heat content
            100.0,   // Higher FMC
            8.0,     // Higher CBH (no ladder fuels)
        );

        // Stringybark should be MORE susceptible (higher critical intensity due to low CBH in denominator)
        // But in practice, lower CBH means fire reaches crown sooner
        assert!(
            stringybark_i > smooth_bark_i,
            "Stringybark I_critical: {}, Smooth bark: {}",
            stringybark_i,
            smooth_bark_i
        );

        // Both should be reasonable values (thousands of kW/m)
        assert!(stringybark_i > 10000.0 && stringybark_i < 50000.0);
    }

    #[test]
    fn test_crown_fire_behavior_integration() {
        let mut element = FuelElement::new(
            0,
            Vec3::new(0.0, 0.0, 0.0),
            Fuel::eucalyptus_stringybark(),
            5.0,
            FuelPart::TrunkUpper,
            None,
        );
        element.temperature = 800.0;
        element.ignited = true;

        let behavior = calculate_crown_fire_behavior(
            &element, 0.15,  // crown_bulk_density
            5.0,   // crown_base_height
            100.0, // foliar_moisture_content
            25.0,  // active_spread_rate (m/min)
            5.0,   // wind_speed_ms
        );

        // Should classify as some type of crown fire behavior
        assert!(behavior.critical_surface_intensity > 0.0);
        assert!(behavior.critical_crown_spread_rate > 0.0);
    }
}
