//! Direct element-to-element heat transfer with Stefan-Boltzmann radiation
//!
//! Implements realistic radiation and convection between fuel elements:
//! - Full Stefan-Boltzmann law with T^4 formula
//! - Geometric view factors
//! - Wind direction effects (26x downwind boost)
//! - Vertical spread (2.5x+ climbing)
//! - Slope effects (exponential uphill)

use crate::core_types::element::{FuelElement, Vec3};

/// Stefan-Boltzmann constant (W/(m²·K⁴))
const STEFAN_BOLTZMANN: f32 = 5.67e-8;

/// Flame emissivity (dimensionless, 0-1)
const EMISSIVITY: f32 = 0.95;

/// Calculate radiant heat flux from source element to target element
/// Uses full Stefan-Boltzmann law: σ * ε * (T_source^4 - T_target^4)
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
        // Downwind: 26x multiplier at 10 m/s wind
        // 1.0 + alignment * wind_speed_ms * 2.5
        // At 10 m/s fully aligned: 1.0 + 1.0 * 10 * 2.5 = 26x
        1.0 + alignment * wind_speed_ms * 2.5
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
pub(crate) fn vertical_spread_factor(from: &FuelElement, to: &FuelElement) -> f32 {
    let height_diff = to.position.z - from.position.z;

    if height_diff > 0.0 {
        // Fire climbs (convection + radiation push flames upward)
        // Base 2.5x + additional boost for each meter of height
        2.5 + (height_diff * 0.1)
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
pub(crate) fn calculate_total_heat_transfer(
    source: &FuelElement,
    target: &FuelElement,
    wind: Vec3,
    dt: f32,
) -> f32 {
    let distance = (target.position - source.position).magnitude();

    // Skip if too far (beyond reasonable radiation distance)
    if distance > 50.0 {
        return 0.0;
    }

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

        // Should be 26x at 10 m/s fully aligned
        assert!(
            multiplier > 20.0,
            "Expected ~26x boost downwind, got {}",
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

        // Fire climbs at least 2.5x faster
        assert!(
            factor >= 2.5,
            "Expected at least 2.5x upward, got {}",
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
