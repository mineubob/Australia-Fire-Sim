//! Direct element-to-element heat transfer with Stefan-Boltzmann radiation
//!
//! Implements realistic radiation and convection between fuel elements:
//! - Full Stefan-Boltzmann law with T^4 formula
//! - Geometric view factors
//! - Wind direction effects (~4-5x downwind boost, calibrated for realistic spread)
//! - Vertical spread (2.5x+ climbing)
//! - Slope effects (exponential uphill)
//!
//! # Scientific References
//! - Stefan-Boltzmann Law: Stefan (1879), Boltzmann (1884)
//! - Wildfire radiation: Butler & Cohen (1998) - "Firefighter Safety Zones"
//! - Wind effects: McArthur (1967), Rothermel (1972)
//! - Slope effects: Butler et al. (2004), Rothermel slope factors

use crate::core_types::element::{FuelElement, Vec3};

/// Stefan-Boltzmann constant (W/(m²·K⁴))
/// Reference: Fundamental physics constant (Stefan 1879, Boltzmann 1884)
const STEFAN_BOLTZMANN: f32 = 5.67e-8;

/// Flame emissivity (dimensionless, 0-1)
/// Reference: Typical wildfire flame emissivity from Butler & Cohen (1998)
const EMISSIVITY: f32 = 0.95;

/// Calculate radiant heat flux from source element to target element
/// Uses full Stefan-Boltzmann law: σ * ε * (T_source^4 - T_target^4)
///
/// # References
/// - Stefan-Boltzmann Law applied to wildfire heat transfer
/// - Butler, B.W. & Cohen, J.D. (1998) - Int. J. Wildland Fire, 8(2)
#[inline(always)]
#[allow(dead_code)] // Kept for tests, superseded by calculate_heat_transfer_raw
pub(crate) fn calculate_radiation_flux(
    source: &FuelElement,
    target: &FuelElement,
    distance: f32,
) -> f32 {
    if distance <= 0.0 || source.fuel_remaining <= 0.0 {
        return 0.0;
    }

    // Convert to Kelvin for Stefan-Boltzmann
    let temp_source_k = source.temperature + 273.15;
    let temp_target_k = target.temperature + 273.15;

    // FULL FORMULA: σ * ε * (T_source^4 - T_target^4)
    // NO SIMPLIFICATIONS per repository guidelines
    let radiant_power =
        STEFAN_BOLTZMANN * EMISSIVITY * (temp_source_k.powi(4) - temp_target_k.powi(4));

    // Only transfer heat if source is hotter
    if radiant_power <= 0.0 {
        return 0.0;
    }

    // View factor (geometric attenuation with inverse square law)
    // Based on source element's radiating surface area
    let source_surface_area = source.fuel.surface_area_to_volume * source.fuel_remaining.sqrt();
    let view_factor = source_surface_area / (4.0 * std::f32::consts::PI * distance * distance);
    let view_factor = view_factor.min(1.0);

    // Calculate flux at target (W/m²)
    let flux = radiant_power * view_factor;

    // Convert to heat energy for target element (kJ/s)
    // Apply target's surface area for heat absorption
    let target_surface_area = target.fuel.surface_area_to_volume;
    flux * target_surface_area * 0.001 // W to kW (kJ/s)
}

/// Calculate convection heat transfer for vertical spread
/// Fire climbs faster due to hot gases rising and preheating fuel above
pub(crate) fn calculate_convection_heat(
    source: &FuelElement,
    target: &FuelElement,
    distance: f32,
) -> f32 {
    let height_diff = target.position.z - source.position.z;

    // Only convection upward (hot air rises)
    if height_diff <= 0.0 || distance <= 0.0 {
        return 0.0;
    }

    // Natural convection coefficient (W/(m²·K))
    // Stronger for larger temperature differences
    let temp_diff = source.temperature - target.temperature;
    if temp_diff <= 0.0 {
        return 0.0;
    }

    // Natural convection coefficient increases with temperature difference
    // h ≈ 1.32 * (ΔT/L)^0.25 for vertical surfaces
    let characteristic_length = height_diff.max(0.1);
    let h = 1.32 * (temp_diff / characteristic_length).powf(0.25);

    // Heat transfer (W)
    let area = source.fuel.surface_area_to_volume * source.fuel_remaining.sqrt();
    let heat_w = h * area * temp_diff;

    // Distance attenuation (convection weakens with horizontal distance)
    let horizontal_dist = ((target.position.x - source.position.x).powi(2)
        + (target.position.y - source.position.y).powi(2))
    .sqrt();
    let attenuation = 1.0 / (1.0 + horizontal_dist / 2.0);

    // Convert to kJ/s
    heat_w * attenuation * 0.001
}

/// Wind direction multiplier for heat transfer
/// 26x boost downwind at 10 m/s, 5% minimum upwind
///
/// # References
/// - McArthur (1967) - Australian bushfire observations
/// - Rothermel (1972) - Wind coefficient equations
/// - Empirical data from Australian fire behavior studies
#[inline(always)]
#[allow(dead_code)] // Kept for tests, superseded by calculate_heat_transfer_raw
pub(crate) fn wind_radiation_multiplier(from: Vec3, to: Vec3, wind: Vec3) -> f32 {
    let wind_speed_ms = wind.magnitude();

    // No wind effect if wind is negligible
    if wind_speed_ms < 0.1 {
        return 1.0;
    }

    let direction = (to - from).normalize();
    let wind_normalized = wind.normalize();
    let alignment = direction.dot(&wind_normalized);

    if alignment > 0.0 {
        // Downwind: Balanced scaling for realistic fire spread
        // Using reduced coefficients to achieve target spread rates:
        //   - Moderate (6.9 m/s): ~3x multiplier → 1-10 ha/hr
        //   - Catastrophic (16.7 m/s): ~5x multiplier → 100-300 ha/hr
        //
        // At 6.9 m/s (25 km/h): 1.0 + 1.0 × sqrt(6.9) × 0.8 = ~3.1× (Moderate)
        // At 16.7 m/s (60 km/h): 1.0 + 1.0 × sqrt(16.7) × 0.8 = ~4.3× (Extreme base)
        let base_multiplier = 1.0 + alignment * wind_speed_ms.sqrt() * 0.8;

        if wind_speed_ms > 15.0 {
            // Additional boost for catastrophic conditions, but gentler
            // At 16.7 m/s: 4.3 × 1.08 = ~4.7×
            let extreme_boost = ((wind_speed_ms - 15.0) / 10.0).min(1.0);
            base_multiplier * (1.0 + extreme_boost * 0.5)
        } else {
            base_multiplier
        }
    } else {
        // Upwind: exponential suppression to 5% minimum
        // alignment is negative, so we want exp(alignment * wind_speed_ms * 0.35)
        // which gives exp(negative) = small number
        // At -1.0 alignment and 10 m/s: exp(-1.0 * 10 * 0.35) = exp(-3.5) ≈ 0.03 (3%)
        ((alignment * wind_speed_ms * 0.35).exp()).max(0.05)
    }
}

/// Vertical spread factor - fire climbs much faster than it spreads horizontally
/// 2.5x+ faster upward due to convection and flame tilt
///
/// # References
/// - General fire behavior physics (convection drives upward spread)
/// - Sullivan (2009) - "Wildland surface fire spread modelling"
#[inline(always)]
#[allow(dead_code)] // Kept for tests, superseded by calculate_heat_transfer_raw
pub(crate) fn vertical_spread_factor(from: &FuelElement, to: &FuelElement) -> f32 {
    let height_diff = to.position.z - from.position.z;

    if height_diff > 0.0 {
        // Fire climbs (convection + radiation push flames upward)
        // Base 1.8x + additional boost for each meter of height
        // Reduced from 2.5x to prevent excessive vertical spread in moderate conditions
        1.8 + (height_diff * 0.08)
    } else if height_diff < 0.0 {
        // Fire descends (radiation only, no convection assist)
        // Weakens with depth, minimum 30% effectiveness
        0.7 * (1.0 / (1.0 + height_diff.abs() * 0.2))
    } else {
        1.0 // Horizontal spread
    }
}

/// Slope effects on fire spread
/// Exponential uphill boost (flames tilt toward fuel ahead)
/// Reduced downhill spread (gravity works against spread)
///
/// # References
/// - Rothermel (1972) - Slope factor equations
/// - Butler et al. (2004) - "Fire behavior on slopes"
#[inline(always)]
#[allow(dead_code)] // Kept for tests, superseded by calculate_heat_transfer_raw
pub(crate) fn slope_spread_multiplier(from: &FuelElement, to: &FuelElement) -> f32 {
    let horizontal = ((to.position.x - from.position.x).powi(2)
        + (to.position.y - from.position.y).powi(2))
    .sqrt();

    if horizontal < 0.01 {
        // Purely vertical, use vertical spread factor instead
        return 1.0;
    }

    let vertical = to.position.z - from.position.z;
    let slope_angle = (vertical / horizontal).atan().to_degrees();

    if slope_angle > 0.0 {
        // Uphill: exponential effect (flames tilt closer to fuel ahead)
        // Slope angle of 10° gives 2x boost, 20° gives ~4.8x boost
        1.0 + (slope_angle / 10.0).powf(1.5) * 2.0
    } else {
        // Downhill: much slower (gravity pulls flames away from unburned fuel)
        // -30° slope gives 0.3x (30% effectiveness)
        (1.0 + slope_angle / 30.0).max(0.3)
    }
}

/// Calculate total heat transfer from source to target element
/// Combines radiation, convection, wind, vertical, and slope effects
#[inline(always)]
#[allow(dead_code)] // Kept for tests, superseded by calculate_heat_transfer_raw
pub(crate) fn calculate_total_heat_transfer(
    source: &FuelElement,
    target: &FuelElement,
    wind: Vec3,
    dt: f32,
) -> f32 {
    // Optimize: avoid sqrt by checking distance_squared first
    let diff = target.position - source.position;
    let distance_sq = diff.x * diff.x + diff.y * diff.y + diff.z * diff.z;

    // Skip if too far (50m → 2500m²)
    if distance_sq > 2500.0 {
        return 0.0;
    }

    let distance = distance_sq.sqrt();

    // Calculate base radiation
    let radiation = calculate_radiation_flux(source, target, distance);

    // Calculate convection (vertical only)
    let convection = calculate_convection_heat(source, target, distance);

    // Apply multipliers
    let wind_factor = wind_radiation_multiplier(source.position, target.position, wind);
    let vertical_factor = vertical_spread_factor(source, target);
    let slope_factor = slope_spread_multiplier(source, target);

    // Total heat transfer (kJ)
    let total_heat = (radiation + convection) * wind_factor * vertical_factor * slope_factor * dt;

    total_heat.max(0.0)
}

/// OPTIMIZED: Calculate heat transfer using raw data instead of FuelElement structures
/// Eliminates 500,000+ temporary structure allocations per frame at 12.5k burning elements
/// Inline attribute ensures this hot function is optimized (called millions of times per frame)
#[inline(always)]
#[allow(clippy::too_many_arguments)] // Performance-critical: avoids 500k+ allocations/frame
pub(crate) fn calculate_heat_transfer_raw(
    source_pos: Vec3,
    source_temp: f32,
    source_fuel_remaining: f32,
    source_surface_area_vol: f32,
    target_pos: Vec3,
    target_temp: f32,
    target_surface_area_vol: f32,
    wind: Vec3,
    dt: f32,
) -> f32 {
    // Distance check (optimized with distance squared)
    let diff = target_pos - source_pos;
    let distance_sq = diff.x * diff.x + diff.y * diff.y + diff.z * diff.z;

    // Skip if too far (50m → 2500m²)
    if distance_sq > 2500.0 {
        return 0.0;
    }

    let distance = distance_sq.sqrt();

    if distance <= 0.0 || source_fuel_remaining <= 0.0 {
        return 0.0;
    }

    // === RADIATION CALCULATION (Stefan-Boltzmann) ===
    let temp_source_k = source_temp + 273.15;
    let temp_target_k = target_temp + 273.15;

    let radiant_power =
        STEFAN_BOLTZMANN * EMISSIVITY * (temp_source_k.powi(4) - temp_target_k.powi(4));

    if radiant_power <= 0.0 {
        return 0.0;
    }

    // View factor (geometric) - uses SOURCE surface area for radiation
    let source_surface_area = source_surface_area_vol * source_fuel_remaining.sqrt();
    let view_factor = source_surface_area / (4.0 * std::f32::consts::PI * distance * distance);
    let view_factor = view_factor.min(1.0);

    let flux = radiant_power * view_factor;
    // Convert to heat energy using TARGET surface area for absorption
    let radiation = flux * target_surface_area_vol * 0.001;

    // === CONVECTION CALCULATION (vertical only) ===
    let vertical_diff = target_pos.z - source_pos.z;
    let convection = if vertical_diff > 0.0 {
        let temp_diff = source_temp - target_temp;
        if temp_diff > 0.0 {
            // Natural convection coefficient for wildfire conditions (W/(m²·K))
            // Varies with temperature difference: h ≈ 1.32 * (ΔT/L)^0.25
            // Typical range: 5-50 W/(m²·K) for natural convection
            // NOTE: This is element-to-element transfer, not element-to-grid
            // Using reduced value (25.0) as conservative baseline for peer transfer
            // Grid transfer uses fuel-specific convective_heat_coefficient
            let convection_coeff = 25.0; // Conservative for element-to-element
            convection_coeff * temp_diff * target_surface_area_vol * 0.001
        } else {
            0.0
        }
    } else {
        0.0
    };

    // === WIND FACTOR ===
    let direction = (target_pos - source_pos).normalize();
    let wind_speed_ms = wind.magnitude();
    let wind_normalized = if wind_speed_ms > 0.1 {
        wind.normalize()
    } else {
        Vec3::new(0.0, 0.0, 0.0)
    };
    let alignment = direction.dot(&wind_normalized);

    let wind_factor = if alignment > 0.0 {
        // Downwind: Balanced scaling for realistic fire spread
        // Using reduced coefficients to achieve target spread rates:
        //   - Moderate (6.9 m/s): ~3x multiplier → 1-10 ha/hr
        //   - Catastrophic (16.7 m/s): ~5x multiplier → 100-300 ha/hr
        //
        // At 6.9 m/s (25 km/h): 1.0 + 1.0 × sqrt(6.9) × 0.8 = ~3.1× (Moderate)
        // At 16.7 m/s (60 km/h): 1.0 + 1.0 × sqrt(16.7) × 0.8 = ~4.3× (Extreme base)
        //
        // NOTE: 0.8 coefficient is for element-to-element transfer physics (universal)
        // Fuel-specific wind_sensitivity is applied at higher level (Rothermel model)
        let base_multiplier = 1.0 + alignment * wind_speed_ms.sqrt() * 0.8;

        if wind_speed_ms > 15.0 {
            // Additional boost for catastrophic conditions, but gentler
            // At 16.7 m/s: 4.3 × 1.08 = ~4.7×
            let extreme_boost = ((wind_speed_ms - 15.0) / 10.0).min(1.0);
            base_multiplier * (1.0 + extreme_boost * 0.5)
        } else {
            base_multiplier
        }
    } else {
        ((-alignment * wind_speed_ms * 0.35).exp()).max(0.05)
    };

    // === VERTICAL FACTOR ===
    let vertical_factor = if vertical_diff > 0.0 {
        // Reduced from 2.5 to 1.8 base to prevent excessive vertical spread
        1.8 + (vertical_diff * 0.08)
    } else if vertical_diff < 0.0 {
        0.7 * (1.0 / (1.0 + vertical_diff.abs() * 0.2))
    } else {
        1.0
    };

    // === SLOPE FACTOR ===
    let horizontal_diff_sq = diff.x * diff.x + diff.y * diff.y;
    let horizontal = horizontal_diff_sq.sqrt();
    let slope_factor = if horizontal > 0.1 {
        let slope_angle_rad = (vertical_diff / horizontal).atan();
        let slope_angle = slope_angle_rad.to_degrees();

        if slope_angle > 0.0 {
            1.0 + (slope_angle / 10.0).powf(1.5) * 2.0
        } else {
            (1.0 + slope_angle / 30.0).max(0.3)
        }
    } else {
        1.0
    };

    // Total heat transfer
    let total_heat = (radiation + convection) * wind_factor * vertical_factor * slope_factor * dt;
    total_heat.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::element::FuelPart;
    use crate::core_types::fuel::Fuel;

    fn create_test_element(x: f32, y: f32, z: f32, temp: f32) -> FuelElement {
        FuelElement::new(
            0,
            Vec3::new(x, y, z),
            Fuel::dry_grass(),
            5.0,
            FuelPart::GroundVegetation,
            None,
        )
        .with_temperature(temp)
    }

    #[test]
    fn test_radiation_flux() {
        let mut source = create_test_element(0.0, 0.0, 0.0, 600.0);
        source.fuel_remaining = 5.0;
        let target = create_test_element(5.0, 0.0, 0.0, 20.0);

        let flux = calculate_radiation_flux(&source, &target, 5.0);

        // Should have positive heat transfer from hot to cold
        assert!(flux > 0.0, "Expected positive radiation flux");
    }

    #[test]
    fn test_wind_boost_downwind() {
        let from = Vec3::new(0.0, 0.0, 0.0);
        let to = Vec3::new(10.0, 0.0, 0.0);
        let wind = Vec3::new(10.0, 0.0, 0.0); // 10 m/s in same direction

        let multiplier = wind_radiation_multiplier(from, to, wind);

        // Calibrated for realistic spread rates:
        //   - Perth Moderate (25 km/h): 1-10 ha/hr target
        //   - Catastrophic (60 km/h): 100-300 ha/hr target
        // At 10 m/s: 1.0 + 1.0 × sqrt(10) × 0.8 = ~3.5×
        // This creates appropriate FFDI-scaled spread differentiation
        assert!(
            multiplier > 3.0 && multiplier < 4.5,
            "Expected ~3.5× boost at 10 m/s, got {}",
            multiplier
        );
    }

    #[test]
    fn test_wind_suppression_upwind() {
        let from = Vec3::new(0.0, 0.0, 0.0);
        let to = Vec3::new(10.0, 0.0, 0.0);
        let wind = Vec3::new(-10.0, 0.0, 0.0); // 10 m/s opposite direction

        let multiplier = wind_radiation_multiplier(from, to, wind);

        // Should be suppressed to ~5% upwind
        assert!(multiplier < 0.1, "Expected ~5% upwind, got {}", multiplier);
        assert!(multiplier >= 0.05, "Should not go below 5%");
    }

    #[test]
    fn test_vertical_climbing() {
        let source = create_test_element(0.0, 0.0, 0.0, 600.0);
        let target_up = create_test_element(0.0, 0.0, 5.0, 20.0);

        let factor = vertical_spread_factor(&source, &target_up);

        // Fire climbs faster: base 1.8× + (5m × 0.08) = ~2.2×
        // Reduced from 2.5× to prevent excessive vertical spread in moderate conditions
        assert!(
            (1.8..=2.5).contains(&factor),
            "Expected 1.8-2.2× upward, got {}",
            factor
        );
    }

    #[test]
    fn test_vertical_descending() {
        let source = create_test_element(0.0, 0.0, 5.0, 600.0);
        let target_down = create_test_element(0.0, 0.0, 0.0, 20.0);

        let factor = vertical_spread_factor(&source, &target_down);

        // Fire spreads slower downward
        assert!(
            factor < 1.0,
            "Expected reduced downward spread, got {}",
            factor
        );
    }

    #[test]
    fn test_slope_uphill_boost() {
        let source = create_test_element(0.0, 0.0, 0.0, 600.0);
        let target = create_test_element(10.0, 0.0, 2.0, 20.0); // ~11° slope

        let factor = slope_spread_multiplier(&source, &target);

        // Should have uphill boost
        assert!(factor > 1.5, "Expected uphill boost, got {}", factor);
    }

    #[test]
    fn test_slope_downhill_reduction() {
        let source = create_test_element(0.0, 0.0, 2.0, 600.0);
        let target = create_test_element(10.0, 0.0, 0.0, 20.0); // ~-11° slope

        let factor = slope_spread_multiplier(&source, &target);

        // Should have reduced effectiveness downhill
        assert!(factor < 1.0, "Expected downhill reduction, got {}", factor);
    }
}
