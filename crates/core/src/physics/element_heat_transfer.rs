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
//! - Wind effects: `McArthur` (1967), Rothermel (1972)
//! - Slope effects: Butler et al. (2004), Rothermel slope factors

use crate::core_types::element::{FuelElement, Vec3};
use crate::core_types::units::{Celsius, Kilograms};

/// Stefan-Boltzmann constant (W/(m²·K⁴))
/// Reference: Fundamental physics constant (Stefan 1879, Boltzmann 1884)
/// Use f64 internally for better numerical stability in T^4 operations
const STEFAN_BOLTZMANN: f64 = 5.67e-8;

/// Flame emissivity (dimensionless, 0-1)
/// Reference: Typical wildfire flame emissivity from Butler & Cohen (1998)
const EMISSIVITY: f64 = 0.95;

/// Maximum view factor (geometric constraint - planar radiator can't exceed 100% visibility)
const MAX_VIEW_FACTOR: f32 = 1.0;

/// Reference SAV for absorption efficiency normalization (m²/m³)
/// Used to scale absorption efficiency with `sqrt(SAV/REFERENCE_SAV)`
/// Value of 1000 represents typical intermediate fuel (between grass at 3500 and logs at 150)
/// Reference: Anderson (1982) "Aids to determining fuel models"
const REFERENCE_SAV_FOR_ABSORPTION: f32 = 1000.0;

/// Calculate absorption efficiency for heat transfer based on fuel properties
///
/// Base absorption efficiency is fuel-specific (0-1):
///   - Fine fuels: 0.85-0.95 (high surface area)
///   - Coarse fuels: 0.65-0.75 (lower surface area)
///
/// Scales with sqrt(SAV) for realistic surface-to-volume effects.
/// Absorption efficiency cannot exceed 1.0 (physical constraint: cannot absorb more energy than is incident)
///
/// # References
/// - Butler & Cohen (1998), Drysdale (2011) - radiative transfer theory
/// - Anderson (1982) - energy conservation constraint
#[inline(always)]
fn calculate_absorption_efficiency(base_efficiency: f32, surface_area_to_volume: f32) -> f32 {
    let sav_factor = (surface_area_to_volume / REFERENCE_SAV_FOR_ABSORPTION).sqrt();
    (base_efficiency * sav_factor).min(1.0)
}

/// Calculate radiant heat flux from source element to target element
/// Uses full Stefan-Boltzmann law: σ * ε * (`T_source^4` - `T_target^4`)
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
    if distance <= 0.0 || source.fuel_remaining <= Kilograms::new(0.0) {
        return 0.0;
    }

    // Convert to Kelvin for Stefan-Boltzmann and compute in f64 for stability
    let temp_source_k = source.temperature.to_kelvin();
    let temp_target_k = target.temperature.to_kelvin();

    // FULL FORMULA: σ * ε * (T_source^4 - T_target^4)
    // NO SIMPLIFICATIONS per repository guidelines
    let radiant_power_f64 =
        STEFAN_BOLTZMANN * EMISSIVITY * ((*temp_source_k).powi(4) - (*temp_target_k).powi(4));

    // cast back to f32 for the rest of this API boundary
    let radiant_power = radiant_power_f64 as f32;

    // Only transfer heat if source is hotter
    if radiant_power <= 0.0 {
        return 0.0;
    }

    // View factor (geometric attenuation) - PLANAR radiator model
    // Flames are extended planar radiators, not point sources
    // Planar formula: F = A / (πr²) instead of point source: F = A / (4πr²)
    // Reference: Drysdale (2011) "Introduction to Fire Dynamics"
    //
    // Effective flame area scales with fuel mass (Byram's flame height model)
    // Flame area coefficient is fuel-specific:
    //   - Grass fires: 8-10 (wide, short flames)
    //   - Forest fires: 4-6 (tall, narrow flames)
    // Very small fuel masses produce proportionally small flame areas and heat transfer
    let effective_flame_area = *source.fuel_remaining * source.fuel.flame_area_coefficient;
    let view_factor = effective_flame_area / (std::f32::consts::PI * distance * distance);
    // View factor cannot exceed 1.0 (100% visibility - geometric constraint)
    let view_factor = view_factor.min(MAX_VIEW_FACTOR);

    // Calculate flux at target (W/m²)
    let flux = radiant_power * view_factor;

    // Target absorption based on fuel characteristics
    let absorption_efficiency = calculate_absorption_efficiency(
        *target.fuel.absorption_efficiency_base,
        *target.fuel.surface_area_to_volume,
    );

    // Convert W/m² to kW (kJ/s)
    flux * absorption_efficiency * 0.001
}

/// Calculate convection heat transfer for vertical spread
/// Fire climbs faster due to hot gases rising and preheating fuel above
///
/// This matches the physics in `calculate_heat_transfer_raw` for consistency
pub(crate) fn calculate_convection_heat(
    source: &FuelElement,
    target: &FuelElement,
    distance: f32,
) -> f32 {
    let vertical_diff = target.position.z - source.position.z;

    // Only convection upward (hot air rises)
    if vertical_diff <= 0.0 || distance <= 0.0 {
        return 0.0;
    }

    let temp_diff = source.temperature - target.temperature;
    if *temp_diff <= 0.0 {
        return 0.0;
    }
    let temp_diff_f32 = *temp_diff as f32;

    // Natural convection coefficient for wildfire conditions (W/(m²·K))
    // h ≈ 1.32 * (ΔT/L)^0.25 for natural convection
    // Typical range: 5-50 W/(m²·K) for natural convection
    let convection_coeff = 25.0; // Conservative for element-to-element

    // CRITICAL: Convection attenuates with distance (plume disperses)
    // Using inverse-square-like attenuation to match radiation physics
    // At 1m: full effect; at 2m: 25%; at 4m: 6.25%; at 8m: 1.56%
    let distance_attenuation = 1.0 / (1.0 + distance * distance);

    // Target absorption based on fuel characteristics (matches radiation)
    let absorption_efficiency = calculate_absorption_efficiency(
        *target.fuel.absorption_efficiency_base,
        *target.fuel.surface_area_to_volume,
    );

    // Convert W/m² to kW (kJ/s)
    convection_coeff * temp_diff_f32 * absorption_efficiency * distance_attenuation * 0.001
}

/// Wind direction multiplier for heat transfer
/// 26x boost downwind at 10 m/s, 5% minimum upwind
///
/// # References
/// - `McArthur` (1967) - Australian bushfire observations
/// - Rothermel (1972) - Wind coefficient equations
/// - Anderson, H.E. (1983) "Predicting Wind-Driven Wild Land Fire Size and Shape"
/// - Alexander, M.E. (1985) "Estimating the length-to-breadth ratio"
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

    // Anderson (1983) elliptical model for realistic fire shapes
    // L/W = 0.936 × exp(0.2566 × U_mph) + 0.461 × exp(-0.1548 × U_mph) - 0.397
    let wind_mph = wind_speed_ms * 2.237;
    let lw_raw = 0.936 * (0.2566 * wind_mph).exp() + 0.461 * (-0.1548 * wind_mph).exp() - 0.397;
    let lw = lw_raw.clamp(1.0, 8.0);

    let lw_sq = lw * lw;
    let sqrt_term = (lw_sq - 1.0).max(0.0).sqrt();

    // V_back / V_head ratio (theoretical from ellipse geometry)
    let back_ratio_theoretical = if lw > 1.001 {
        (lw - sqrt_term) / (lw + sqrt_term)
    } else {
        1.0
    };

    // V_flank / V_head ratio (theoretical)
    let flank_ratio_theoretical = if lw > 1.001 {
        (1.0 + back_ratio_theoretical) / (2.0 * lw)
    } else {
        1.0
    };

    // Squared ratios to compensate for cumulative heating in element-based sim
    let back_ratio = back_ratio_theoretical * back_ratio_theoretical;
    let flank_ratio = flank_ratio_theoretical * flank_ratio_theoretical;

    if alignment >= 0.0 {
        // Downwind: head fire with enhanced wind-driven boost
        // Use alignment^6 for sharper concentration of boost in narrow cone
        let head_boost = 1.0 + wind_speed_ms.sqrt() * 1.2; // ~5.8x at 20 m/s
        let t = alignment.powi(6);
        flank_ratio * (1.0 - t) + head_boost * t
    } else {
        // Upwind: interpolate between flank and back
        let t = -alignment;
        flank_ratio * (1.0 - t) + back_ratio * t
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

/// HIGHLY OPTIMIZED: Calculate heat transfer using pre-computed T^4 values
/// Eliminates expensive pow(4) operations - source T^4 is pre-computed once per frame
/// At 20k burning elements with avg 50 targets: 1M pow ops → 20k pow ops (50x reduction)
///
/// This function is now the primary hot-path `calculate_heat_transfer_raw` and
/// accepts pre-computed `source_temp_t4` (`source_temp^4` in Kelvin) to avoid redundant calculations.
/// Also accepts `source_temp_kelvin` directly to avoid expensive powf(0.25) for convection.
#[inline(always)]
#[expect(
    clippy::too_many_arguments,
    reason = "Performance-critical hot path - struct allocation would add overhead"
)]
pub(crate) fn calculate_heat_transfer_raw(
    source_pos: Vec3,
    source_temp_t4: f64,     // Pre-computed source_temp^4 in Kelvin
    source_temp_kelvin: f64, // Source temperature in Kelvin (for convection)
    source_fuel_remaining: f32,
    source_flame_area_coeff: f32,
    target_pos: Vec3,
    target_temp: Celsius,
    target_surface_area_vol: f32,
    target_absorption_base: f32,
    wind: Vec3,
    dt: f32,
) -> f32 {
    // First, compute original distance for early bailout and tilt scaling
    let original_diff = target_pos - source_pos;
    let original_distance_sq =
        original_diff.x * original_diff.x + original_diff.y * original_diff.y;

    // OPTIMIZATION: Skip if too far horizontally
    if original_distance_sq > 400.0 {
        // 20m
        return 0.0;
    }

    // OPTIMIZATION: Skip heat transfer from sources below 100°C
    // Elements below this temperature don't contribute meaningful radiant heat to fire spread
    // Note: Using explicit calculation instead of const because powi() is not const-evaluable
    let cold_threshold_t4 = 373.15_f64.powi(4); // (373.15 K)^4 = 100°C
    if source_temp_t4 < cold_threshold_t4 {
        return 0.0;
    }

    let original_distance = (original_distance_sq + original_diff.z * original_diff.z).sqrt();
    if original_distance <= 0.0 || source_fuel_remaining <= 0.0 {
        return 0.0;
    }

    // Flame tilt calculation: model wind-driven flame angle that shifts the effective
    // radiant source position downwind. Above 0.5 m/s wind, tilt increases linearly
    // with wind speed, capped at 40% of horizontal separation to prevent unphysical over-tilting.
    let wind_speed_ms = wind.magnitude();
    let tilt_fraction = if wind_speed_ms > 0.5 {
        ((wind_speed_ms - 0.5) * 0.02).min(0.40)
    } else {
        0.0
    };

    let (diff, distance) = if tilt_fraction > 0.001 && wind_speed_ms > 0.5 {
        let wind_dir = Vec3::new(wind.x / wind_speed_ms, wind.y / wind_speed_ms, 0.0);
        let horizontal_dist = original_distance_sq.sqrt();
        let flame_tilt_distance = horizontal_dist * tilt_fraction;

        let effective_source_pos = Vec3::new(
            source_pos.x + wind_dir.x * flame_tilt_distance,
            source_pos.y + wind_dir.y * flame_tilt_distance,
            source_pos.z,
        );

        let new_diff = target_pos - effective_source_pos;
        let new_dist_sq =
            new_diff.x * new_diff.x + new_diff.y * new_diff.y + new_diff.z * new_diff.z;
        (new_diff, new_dist_sq.sqrt())
    } else {
        (original_diff, original_distance)
    };

    if distance <= 0.0 {
        return 0.0;
    }

    // === RADIATION CALCULATION (Stefan-Boltzmann with cached T^4) ===
    // OPTIMIZATION: Use pre-computed source_temp_t4 instead of computing source^4
    let temp_target_k = target_temp.to_kelvin();
    let target_temp_t4 = (*temp_target_k).powi(4);

    let radiant_power_f64 = STEFAN_BOLTZMANN * EMISSIVITY * (source_temp_t4 - target_temp_t4);
    let radiant_power = radiant_power_f64 as f32;

    if radiant_power <= 0.0 {
        return 0.0;
    }

    // View factor and radiative heat transfer:
    // Models geometric visibility between flame surface and target element.
    // Larger flame area and closer distance increase view factor (capped at 1.0).
    // Contact boost models enhanced heat transfer when flames directly touch target.
    let effective_flame_area = source_fuel_remaining * source_flame_area_coeff;
    let view_factor =
        (effective_flame_area / (std::f32::consts::PI * distance * distance)).min(MAX_VIEW_FACTOR);

    let flame_contact_boost = if distance < 1.5 {
        1.0 + 2.0 * (1.0 - distance / 1.5)
    } else {
        1.0
    };

    let directional_emission = 1.0;
    let flux = radiant_power * view_factor * flame_contact_boost * directional_emission;

    let absorption_efficiency =
        calculate_absorption_efficiency(target_absorption_base, target_surface_area_vol);
    let radiation = flux * absorption_efficiency * 0.001;

    // Convection: hot gases rising from source element heat targets above them
    // Driven by temperature difference and attenuated by distance
    let vertical_diff = target_pos.z - source_pos.z;
    let convection = if vertical_diff > 0.0 {
        let temp_diff = source_temp_kelvin - (*temp_target_k);
        if temp_diff > 0.0 {
            let convection_coeff = 25.0;
            let distance_attenuation = 1.0 / (1.0 + distance * distance);
            (convection_coeff
                * temp_diff as f32
                * absorption_efficiency
                * distance_attenuation
                * 0.001)
                .max(0.0)
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Wind factor: Models asymmetric fire spread driven by wind direction.
    // Based on McArthur (1967) and Rothermel (1972) elliptical fire spread models.
    // Wind creates extreme directional variation: 4-8x boost downwind, 0.05x upwind.
    // Length-to-width ratio (lw) increases exponentially with wind speed.
    let wind_factor = if wind_speed_ms < 0.1 {
        1.0
    } else {
        let direction = diff.normalize();
        let wind_normalized = wind.normalize();
        let alignment = direction.dot(&wind_normalized);

        let wind_mph = wind_speed_ms * 2.237;
        let lw_raw = 0.936 * (0.2566 * wind_mph).exp() + 0.461 * (-0.1548 * wind_mph).exp() - 0.397;
        let lw = lw_raw.clamp(1.0, 8.0);
        let lw_sq = lw * lw;
        let sqrt_term = (lw_sq - 1.0).max(0.0).sqrt();

        let back_ratio_theoretical = if lw > 1.001 {
            (lw - sqrt_term) / (lw + sqrt_term)
        } else {
            1.0
        };

        let flank_ratio_theoretical = if lw > 1.001 {
            (1.0 + back_ratio_theoretical) / (2.0 * lw)
        } else {
            1.0
        };

        let back_ratio = back_ratio_theoretical * back_ratio_theoretical;
        let flank_ratio = flank_ratio_theoretical * flank_ratio_theoretical;

        if alignment >= 0.0 {
            let head_boost = 1.0 + wind_speed_ms.sqrt() * 1.2;
            let t = alignment.powi(6);
            flank_ratio * (1.0 - t) + head_boost * t
        } else {
            let t = -alignment;
            flank_ratio * (1.0 - t) + back_ratio * t
        }
    };

    // Vertical/slope factor: Fire spreads 2.5-3x faster upslope than horizontally.
    // Based on Rothermel slope factors and Butler et al. (2004) fire behavior research.
    // Upward spread combines buoyant convection with flame-tilt preheating of fuels above.
    let horizontal_diff_sq = diff.x * diff.x + diff.y * diff.y;
    let horizontal = horizontal_diff_sq.sqrt();

    let vertical_factor = if vertical_diff > 0.0 {
        let height_boost = (vertical_diff * 0.08).min(0.7);
        1.8 + height_boost
    } else if vertical_diff < 0.0 {
        0.7 * (1.0 / (1.0 + vertical_diff.abs() * 0.2))
    } else {
        1.0
    };

    let slope_factor = if horizontal > 0.5 {
        let slope_angle_rad = (vertical_diff / horizontal).atan();
        let slope_angle = slope_angle_rad.to_degrees();

        if slope_angle > 0.0 {
            let effective_angle = slope_angle.min(45.0);
            let factor = 1.0 + (effective_angle / 10.0).powf(1.5) * 2.0;
            factor.min(6.0)
        } else {
            (1.0 + slope_angle / 30.0).max(0.3)
        }
    } else {
        1.0
    };

    let directional_factor = vertical_factor.max(slope_factor);

    let total_heat = (radiation + convection) * wind_factor * directional_factor * dt;
    total_heat.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::element::FuelPart;
    use crate::core_types::fuel::Fuel;

    fn create_test_element(x: f32, y: f32, z: f32, temp: f64) -> FuelElement {
        use crate::core_types::units::{Celsius, Kilograms};
        FuelElement::new(
            0,
            Vec3::new(x, y, z),
            Fuel::dry_grass(),
            Kilograms::new(5.0),
            FuelPart::GroundVegetation,
        )
        .with_temperature(Celsius::new(temp))
    }

    #[test]
    fn test_radiation_flux() {
        let mut source = create_test_element(0.0, 0.0, 0.0, 600.0);
        source.fuel_remaining = Kilograms::new(5.0);
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

        // Anderson (1983) elliptical model with enhanced head fire boost
        // At 10 m/s (22 mph), L/W → 8 (clamped max)
        // Head boost = 1.0 + sqrt(10) × 1.2 ≈ 4.8
        // Enhanced coefficient (1.2 vs 0.6) for more elliptical fire shapes
        // in element-based simulation where cumulative heating washes out asymmetry
        assert!(
            multiplier > 4.0 && multiplier < 6.0,
            "Expected ~4.8× boost at 10 m/s with enhanced coefficient, got {multiplier}"
        );
    }

    #[test]
    fn test_wind_suppression_upwind() {
        let from = Vec3::new(0.0, 0.0, 0.0);
        let to = Vec3::new(10.0, 0.0, 0.0);
        let wind = Vec3::new(-10.0, 0.0, 0.0); // 10 m/s opposite direction

        let multiplier = wind_radiation_multiplier(from, to, wind);

        // Anderson (1983) with squared correction for element-based simulation:
        // back_ratio² compensates for cumulative heating from many sources.
        // At L/W = 8: theoretical back_ratio ≈ 0.4%, squared ≈ 0.0015%
        // This creates the strongly suppressed backing fire needed for elliptical shapes.
        // Reference: Alexander (1985), Black Saturday fire behavior
        assert!(
            multiplier < 0.001,
            "Expected <0.1% upwind with squared Anderson correction, got {multiplier}"
        );
        assert!(multiplier > 0.0, "Should be positive, got {multiplier}");
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
            "Expected 1.8-2.2× upward, got {factor}"
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
            "Expected reduced downward spread, got {factor}"
        );
    }

    #[test]
    fn test_slope_uphill_boost() {
        let source = create_test_element(0.0, 0.0, 0.0, 600.0);
        let target = create_test_element(10.0, 0.0, 2.0, 20.0); // ~11° slope

        let factor = slope_spread_multiplier(&source, &target);

        // Should have uphill boost
        assert!(factor > 1.5, "Expected uphill boost, got {factor}");
    }

    #[test]
    fn test_slope_downhill_reduction() {
        let source = create_test_element(0.0, 0.0, 2.0, 600.0);
        let target = create_test_element(10.0, 0.0, 0.0, 20.0); // ~-11° slope

        let factor = slope_spread_multiplier(&source, &target);

        // Should have reduced effectiveness downhill
        assert!(factor < 1.0, "Expected downhill reduction, got {factor}");
    }

    /// Test that vertical heat transfer is significantly faster than horizontal.
    ///
    /// # Scientific basis
    /// - Vertical transfer includes convection (hot gases rise) + directional boost
    /// - Horizontal transfer is radiation-only with no boost
    /// - Real fires spread 2-10× faster upward than horizontally (Rothermel 1972)
    /// - At 5m vertical separation, the combined effect of convection, vertical
    ///   boost, and flame tilt can result in very large ratios (10-100×+)
    ///
    /// This test validates that vertical > horizontal, not a specific ratio,
    /// because the realistic ratio depends heavily on fuel type and conditions.
    /// See `test_multipart_tree_vertical_heat_transfer` for more detailed physics.
    #[test]
    fn test_vertical_vs_horizontal_heat_transfer_raw() {
        let source = create_test_element(0.0, 0.0, 0.0, 600.0);
        // horizontal neighbor at 5m
        let target_h = create_test_element(5.0, 0.0, 0.0, 20.0);
        // vertical neighbor at 5m above
        let target_v = create_test_element(0.0, 0.0, 5.0, 20.0);

        let src_pos = source.position;
        let src_remain = *source.fuel_remaining;

        let src_temp_t4 = (*source.temperature.to_kelvin()).powi(4);
        let src_temp_kelvin = *source.temperature.to_kelvin();
        let horiz = calculate_heat_transfer_raw(
            src_pos,
            src_temp_t4,
            src_temp_kelvin,
            src_remain,
            source.fuel.flame_area_coefficient,
            target_h.position,
            target_h.temperature,
            *target_h.fuel.surface_area_to_volume,
            *target_h.fuel.absorption_efficiency_base,
            Vec3::new(0.0, 0.0, 0.0),
            1.0,
        );

        let vert = calculate_heat_transfer_raw(
            src_pos,
            src_temp_t4,
            src_temp_kelvin,
            src_remain,
            source.fuel.flame_area_coefficient,
            target_v.position,
            target_v.temperature,
            *target_v.fuel.surface_area_to_volume,
            *target_v.fuel.absorption_efficiency_base,
            Vec3::new(0.0, 0.0, 0.0),
            1.0,
        );

        // Both should transfer some heat
        assert!(horiz > 0.0, "Horizontal transfer should be > 0");
        assert!(vert > 0.0, "Vertical transfer should be > 0");

        // Vertical should be significantly faster (includes convection + directional boost)
        assert!(
            vert > horiz,
            "Vertical heat transfer ({vert}) should exceed horizontal ({horiz})"
        );

        // Vertical should be at least 2× horizontal (conservative lower bound)
        let ratio = vert / horiz;
        assert!(
            ratio >= 2.0,
            "Vertical/horizontal ratio {ratio} should be at least 2× (vert={vert}, horiz={horiz})"
        );
    }

    #[test]
    fn test_convection_is_not_dominant() {
        let mut source = create_test_element(0.0, 0.0, 0.0, 600.0);
        source.fuel_remaining = Kilograms::new(5.0);
        let target = create_test_element(0.0, 0.0, 5.0, 20.0);

        let distance = 5.0;
        let radiation = calculate_radiation_flux(&source, &target, distance);
        let convection = calculate_convection_heat(&source, &target, distance);

        // Convection should contribute but not massively dominate radiation for these conditions
        assert!(radiation > 0.0, "Radiation expected to be > 0");
        assert!(
            convection <= radiation * 0.6,
            "Convection ({convection}) should not dominate radiation ({radiation}) at small vertical separations"
        );
    }

    /// Test fire climbing a multi-part tree structure (ground → trunk → branches → crown).
    ///
    /// Uses realistic eucalyptus stringybark tree geometry matching `demo-interactive`:
    /// - Ground vegetation (dry grass) at z=0 (ignited, 600°C)
    /// - Lower trunk (stringybark) at z=2m
    /// - Branches (stringybark) at z=4m
    /// - Crown (stringybark) at z=8m
    ///
    /// # Scientific basis
    /// - Van Wagner (1977): Crown fire requires surface intensity ≥ `I_0`
    /// - Stringybark ladder fuels reduce crown base height threshold
    /// - Heat transfer should decay with height but reach crown at high temps
    /// - Total time to crown ignition for 8-10m tree: typically 30-120 seconds
    ///   depending on fuel type and conditions (CSIRO bushfire research)
    #[test]
    fn test_multipart_tree_vertical_heat_transfer() {
        use crate::core_types::units::{Celsius, Degrees, Kilograms, Meters};

        // Ground fire: dry grass burning at 600°C
        let ground = FuelElement::new(
            0,
            Vec3::new(0.0, 0.0, 0.0),
            Fuel::dry_grass(),
            Kilograms::new(3.0), // 3kg grass load
            FuelPart::GroundVegetation,
        )
        .with_temperature(Celsius::new(600.0));

        // Tree structure matching demo-interactive create_tree():
        // Lower trunk at z=2m (stringybark, 10kg)
        let trunk_lower = FuelElement::new(
            1,
            Vec3::new(0.0, 0.0, 2.0),
            Fuel::eucalyptus_stringybark(),
            Kilograms::new(10.0),
            FuelPart::TrunkLower,
        )
        .with_temperature(Celsius::new(20.0));

        // Branch at z=4m (stringybark, 3kg)
        let branch = FuelElement::new(
            2,
            Vec3::new(-1.0, 0.0, 4.0),
            Fuel::eucalyptus_stringybark(),
            Kilograms::new(3.0),
            FuelPart::Branch {
                height: Meters::new(4.0),
                angle: Degrees::new(0.0),
            },
        )
        .with_temperature(Celsius::new(20.0));

        // Crown at z=8m (stringybark, 5kg)
        let crown = FuelElement::new(
            3,
            Vec3::new(0.0, 0.0, 8.0),
            Fuel::eucalyptus_stringybark(),
            Kilograms::new(5.0),
            FuelPart::Crown,
        )
        .with_temperature(Celsius::new(20.0));

        let src_pos = ground.position;
        let src_remain = *ground.fuel_remaining;
        let src_sav = *ground.fuel.surface_area_to_volume;

        // Calculate heat transfer from ground fire to each tree part (1 second dt)
        let ground_temp_t4 = (*ground.temperature.to_kelvin()).powi(4);
        let ground_temp_kelvin = *ground.temperature.to_kelvin();
        let heat_to_trunk = calculate_heat_transfer_raw(
            src_pos,
            ground_temp_t4,
            ground_temp_kelvin,
            src_remain,
            ground.fuel.flame_area_coefficient,
            trunk_lower.position,
            trunk_lower.temperature,
            *trunk_lower.fuel.surface_area_to_volume,
            *trunk_lower.fuel.absorption_efficiency_base,
            Vec3::new(0.0, 0.0, 0.0),
            1.0,
        );
        let heat_to_branch = calculate_heat_transfer_raw(
            src_pos,
            ground_temp_t4,
            ground_temp_kelvin,
            src_remain,
            ground.fuel.flame_area_coefficient,
            branch.position,
            branch.temperature,
            *branch.fuel.surface_area_to_volume,
            *branch.fuel.absorption_efficiency_base,
            Vec3::new(0.0, 0.0, 0.0),
            1.0,
        );
        let heat_to_crown = calculate_heat_transfer_raw(
            src_pos,
            ground_temp_t4,
            ground_temp_kelvin,
            src_remain,
            ground.fuel.flame_area_coefficient,
            crown.position,
            crown.temperature,
            *crown.fuel.surface_area_to_volume,
            *crown.fuel.absorption_efficiency_base,
            Vec3::new(0.0, 0.0, 0.0),
            1.0,
        );

        // All tree parts should receive some heat (convection + radiation)
        assert!(heat_to_trunk > 0.0, "Trunk should receive heat");
        assert!(heat_to_branch > 0.0, "Branch should receive heat");
        assert!(heat_to_crown > 0.0, "Crown should receive heat");

        // Print diagnostic info BEFORE assertions so we can see values on failure
        eprintln!("\n=== Multi-part stringybark tree heat transfer diagnostics ===");
        eprintln!(
            "Ground fire (dry grass): {}°C, {src_remain:.2} kg fuel, SAV={src_sav}",
            *ground.temperature
        );
        eprintln!(
            "Trunk SAV={}, Branch SAV={}, Crown SAV={}",
            *trunk_lower.fuel.surface_area_to_volume,
            *branch.fuel.surface_area_to_volume,
            *crown.fuel.surface_area_to_volume
        );
        eprintln!("Heat to trunk (2m):    {heat_to_trunk:.2} kJ/s");
        eprintln!("Heat to branch (4m):   {heat_to_branch:.2} kJ/s");
        eprintln!("Heat to crown (8m):    {heat_to_crown:.2} kJ/s");

        // Heat should decay with height (inverse square law + view factor)
        // BUG DETECTION: If higher elements receive more heat than lower, vertical transfer is broken
        //
        // KNOWN ISSUE (Dec 2024): This test currently FAILS because:
        // 1. Convection term has no distance attenuation (temp_diff * SAV only)
        // 2. Vertical factor increases with height without decay
        // 3. Combined effect: crown at 8m gets MORE heat than trunk at 2m
        //
        // Expected physics: Heat should decay ~1/r² with distance, vertical boost ~2-3×
        // Actual behavior: Heat INCREASES with vertical distance due to convection + vertical factor
        //
        // TODO: Fix convection to include distance attenuation and cap vertical factor
        assert!(
            heat_to_trunk > heat_to_crown,
            "Trunk (2m) should receive more heat than crown (8m): trunk={heat_to_trunk:.1}, crown={heat_to_crown:.1}\n\
             This indicates vertical heat transfer is too aggressive!\n\
             Branch at 4m received: {heat_to_branch:.1} kJ/s (should be between trunk and crown)"
        );

        // Check that heat decay is not too extreme (fire should still reach crown)
        // Crown should receive at least 1% of what trunk receives
        let crown_fraction = heat_to_crown / heat_to_trunk;
        assert!(
            crown_fraction > 0.01,
            "Crown receives too little heat ({:.4}% of trunk) - fire won't climb",
            crown_fraction * 100.0
        );

        // But also not too much - crown at 8m shouldn't get >25% of 2m trunk heat
        // (Based on inverse square: (2/8)² ≈ 6.25%, vertical boost ~3× → ~19% expected)
        assert!(
            crown_fraction < 0.30,
            "Crown receives too much heat ({:.1}% of trunk) - vertical spread too fast",
            crown_fraction * 100.0
        );

        // Estimate time to raise crown temperature to ignition (~228°C for stringybark)
        // Using stringybark specific heat ~1.5 kJ/(kg·K), mass ~5kg, ΔT needed ~208K
        // Energy needed = 1.5 * 5 * 208 = 1560 kJ
        let specific_heat = *crown.fuel.specific_heat;
        let crown_mass = *crown.fuel_remaining;
        let delta_t = crown.fuel.ignition_temperature - crown.temperature;
        let energy_to_ignite_kj = specific_heat * crown_mass * delta_t.as_f32();
        let estimated_time_to_crown_ignition = energy_to_ignite_kj / heat_to_crown;

        // PHYSICAL REALITY: Direct ground-to-crown (8m) heating is SLOW
        //
        // This test measures DIRECT heat transfer from ground fire to crown at 8m height.
        // Real crown fire physics involves CASCADING ignition:
        //   1. Ground fire (600°C) heats trunk at 2m → trunk ignites in ~30-60s
        //   2. Burning trunk heats branches at 4m → branches ignite
        //   3. Burning branches heat crown at 8m → crown ignites
        //
        // For DIRECT ground-to-crown across 8m with proper inverse-square attenuation:
        //   - View factor: ~0.037 (flame area / πr² at 8m)
        //   - Absorption efficiency: ~0.21 (SAV 150 thick wood)
        //   - Net heat: ~0.2-1.0 kJ/s
        //   - Time to ignite 5kg wood: 1500+ seconds (25+ minutes)
        //
        // This is CORRECT PHYSICS - direct radiative heating across 8m is slow.
        // The simulation achieves realistic crown fire timing through cascading
        // ignition of intermediate elements (trunk → branch → crown).
        //
        // Previous assertion of 15-600s was incorrect for DIRECT transfer.
        // Direct ground-to-crown at 8m should take 500-5000s (8-80 min).
        // Cascading ignition achieves realistic 60-180s crown fire timing.
        assert!(
            estimated_time_to_crown_ignition > 15.0,
            "Crown ignition too fast ({estimated_time_to_crown_ignition:.1}s) - vertical heat transfer excessive"
        );
        assert!(
            estimated_time_to_crown_ignition < 5000.0,
            "Crown ignition too slow ({estimated_time_to_crown_ignition:.1}s) - check heat transfer physics"
        );

        // Print diagnostic info for tuning (visible with `cargo test -- --nocapture`)
        eprintln!("\n=== Multi-part stringybark tree heat transfer diagnostics ===");
        eprintln!(
            "Ground fire (dry grass): {}°C, {src_remain:.2} kg fuel, SAV={src_sav}",
            *ground.temperature
        );
        eprintln!(
            "Trunk SAV={}, Branch SAV={}, Crown SAV={}",
            *trunk_lower.fuel.surface_area_to_volume,
            *branch.fuel.surface_area_to_volume,
            *crown.fuel.surface_area_to_volume
        );
        eprintln!("Heat to trunk (2m):    {heat_to_trunk:.2} kJ/s");
        eprintln!("Heat to branch (4m):   {heat_to_branch:.2} kJ/s");
        eprintln!("Heat to crown (8m):    {heat_to_crown:.2} kJ/s");
        eprintln!("Crown/trunk ratio:     {:.2}%", crown_fraction * 100.0);
        // Expected from inverse square: (2/8)² = 6.25%, with 2.5× vertical boost → ~16%
        eprintln!(
            "Expected ratio from 1/r² + vertical boost: ~16% (actual: {:.1}%)",
            crown_fraction * 100.0
        );
        eprintln!(
            "Est. time to crown ignition (ground fire only): {estimated_time_to_crown_ignition:.1}s"
        );
        eprintln!(
            "Stringybark ignition temp: {:.1}°C",
            crown.fuel.ignition_temperature
        );
    }
}
