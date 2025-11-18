use crate::core_types::element::{FuelElement, Vec3};

/// Stefan-Boltzmann constant (W/(m²·K⁴))
const STEFAN_BOLTZMANN: f32 = 5.67e-8;

/// Calculate radiation heat flux from source to target using Stefan-Boltzmann Law
/// Q = σ * ε * A * F * (T_source^4 - T_target^4)
/// where σ = Stefan-Boltzmann constant, ε = emissivity, A = area, F = view factor
/// blocking_factor: 0-1, reduction due to intervening non-burnable objects
pub fn calculate_radiation_flux_with_blocking(
    source: &FuelElement,
    target: &FuelElement,
    distance: f32,
    blocking_factor: f32,
) -> f32 {
    if !source.ignited || distance <= 0.0 {
        return 0.0;
    }

    // Convert temperatures to Kelvin
    let source_temp_k = source.temperature + 273.15;
    let target_temp_k = target.temperature + 273.15;

    // Emissivity for flames and burning materials (typical: 0.9-0.95)
    let emissivity = 0.95;

    // Calculate source radiating area based on fuel geometry
    let source_area = source.fuel.surface_area_to_volume * source.fuel_remaining.sqrt();

    // View factor calculation (geometric factor for radiation exchange)
    // F = A_source / (4 * π * r²) for point source approximation
    let view_factor = source_area / (4.0 * std::f32::consts::PI * distance * distance);
    let view_factor = view_factor.min(1.0);

    // Stefan-Boltzmann Law: radiant flux in W/m²
    // Use full T^4 formula, not simplified version
    let flux = STEFAN_BOLTZMANN
        * emissivity
        * view_factor
        * (source_temp_k.powi(4) - target_temp_k.powi(4));

    // Apply to target receiving area
    let target_area = target.fuel.surface_area_to_volume;

    // Convert W/m² to kJ/s (per second energy transfer)
    let heat_kj = flux * target_area * 0.001;

    // Apply blocking factor (heat reduced by non-burnable obstacles)
    (heat_kj * blocking_factor).max(0.0)
}

/// Calculate radiation heat flux (without blocking check, for compatibility)
pub fn calculate_radiation_flux(source: &FuelElement, target: &FuelElement, distance: f32) -> f32 {
    calculate_radiation_flux_with_blocking(source, target, distance, 1.0)
}

/// Calculate convection heat transfer (for elements directly above)
pub fn calculate_convection_heat(source: &FuelElement, target: &FuelElement, distance: f32) -> f32 {
    if !source.ignited {
        return 0.0;
    }

    // Only significant for elements above the source
    if target.position.z <= source.position.z {
        return 0.0;
    }

    let intensity = source.byram_fireline_intensity();
    intensity * 0.15 / (distance + 1.0)
}

/// Calculate wind radiation multiplier (CRITICAL: extreme directional spread)
pub fn wind_radiation_multiplier(from: Vec3, to: Vec3, wind: Vec3) -> f32 {
    // OPTIMIZATION: Early exit for calm conditions using squared magnitude
    let wind_mag_sq = wind.x * wind.x + wind.y * wind.y + wind.z * wind.z;
    if wind_mag_sq < 0.01 {
        return 1.0;
    }

    let direction = (to - from).normalize();
    let alignment = direction.dot(&wind.normalize());
    let wind_speed_ms = wind_mag_sq.sqrt();

    if alignment > 0.0 {
        // Downwind: 41x multiplier at high wind
        1.0 + alignment * wind_speed_ms * 2.5
    } else {
        // Upwind: exponential suppression to 5% minimum
        ((-alignment.abs() * wind_speed_ms * 0.35).exp()).max(0.05)
    }
}

/// Calculate wind diffusion multiplier (even stronger effect)
pub fn wind_diffusion_multiplier(from: Vec3, to: Vec3, wind: Vec3) -> f32 {
    if wind.magnitude() < 0.1 {
        return 1.0;
    }

    let direction = (to - from).normalize();
    let alignment = direction.dot(&wind.normalize());
    let wind_speed_ms = wind.magnitude();

    if alignment > 0.0 {
        // Downwind: 61x boost
        1.0 + alignment * wind_speed_ms * 3.0
    } else {
        // Upwind: 2% minimum
        ((-alignment.abs() * wind_speed_ms * 0.4).exp()).max(0.02)
    }
}

/// Calculate vertical spread factor (fire climbs naturally)
pub fn vertical_spread_factor(from: &FuelElement, to: &FuelElement) -> f32 {
    let height_diff = to.position.z - from.position.z;

    if height_diff > 0.0 {
        // Fire climbs (convection + radiation push flames upward)
        2.5 + (height_diff * 0.1)
    } else if height_diff < 0.0 {
        // Fire descends (radiation only, no convection assist)
        0.7 * (1.0 / (1.0 + height_diff.abs() * 0.2))
    } else {
        1.0 // Horizontal
    }
}

/// Calculate slope spread multiplier
pub fn slope_spread_multiplier(from: &FuelElement, to: &FuelElement) -> f32 {
    let horizontal = ((to.position.x - from.position.x).powi(2)
        + (to.position.y - from.position.y).powi(2))
    .sqrt();

    if horizontal < 0.1 {
        return 1.0;
    }

    let vertical = to.position.z - from.position.z;
    let slope_angle = (vertical / horizontal).atan().to_degrees();

    if slope_angle > 0.0 {
        // Uphill: flames tilt closer to fuel ahead (exponential effect)
        1.0 + (slope_angle / 10.0).powf(1.5) * 2.0
    } else {
        // Downhill: much slower
        (1.0 + slope_angle / 30.0).max(0.3)
    }
}

/// Calculate wind speed at a given height (logarithmic profile)
pub fn wind_at_height(wind_10m: f32, height: f32) -> f32 {
    // Logarithmic wind profile
    // v(h) = v_ref × [ln(h/z0) / ln(h_ref/z0)]
    // z0 = 0.5m (roughness length for forest)

    if height < 0.5 {
        return 0.0;
    }

    wind_10m * (height / 0.5).ln() / (10.0_f32 / 0.5).ln()
}

/// Check if crown fire transition should occur
pub fn check_crown_transition(element: &FuelElement, fire_intensity: f32, wind_speed: f32) -> bool {
    // Base threshold from fuel type
    let base_threshold = element.fuel.crown_fire_threshold;

    // CRITICAL: High ladder fuel factor dramatically lowers threshold
    let ladder_factor = element.fuel.bark_properties.ladder_fuel_factor;
    let threshold = if ladder_factor > 0.8 {
        // Extreme ladder fuels like Stringybark
        let bark_boost = element.fuel.bark_ladder_intensity; // 600-700 kW/m

        // Can cause crown fire at 30% normal intensity!
        if fire_intensity + bark_boost > 300.0 {
            return true; // GUARANTEED crown transition
        }
        base_threshold * (1.0 - ladder_factor * 0.7) // Up to 70% reduction
    } else {
        base_threshold * (1.0 - ladder_factor * 0.3) // Moderate reduction
    };

    // Wind increases crown fire probability
    let wind_factor = 1.0 + wind_speed * 0.05;

    fire_intensity * wind_factor > threshold
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::element::FuelPart;
    use crate::core_types::fuel::Fuel;

    #[test]
    fn test_wind_directionality() {
        let from = Vec3::new(0.0, 0.0, 0.0);
        let to_downwind = Vec3::new(10.0, 0.0, 0.0);
        let to_upwind = Vec3::new(-10.0, 0.0, 0.0);
        let wind = Vec3::new(10.0, 0.0, 0.0); // 10 m/s wind in +X direction

        let downwind_mult = wind_radiation_multiplier(from, to_downwind, wind);
        let upwind_mult = wind_radiation_multiplier(from, to_upwind, wind);

        println!(
            "Downwind multiplier: {:.2}, Upwind multiplier: {:.2}",
            downwind_mult, upwind_mult
        );

        // Downwind should be significantly faster than upwind
        // At 10 m/s: downwind = 1 + 1.0 * 10 * 2.5 = 26x
        // At 10 m/s: upwind = exp(-1.0 * 10 * 0.35) = 0.03x
        // Ratio should be ~26 / 0.05 = 520x (using the 0.05 minimum)
        assert!(downwind_mult > 20.0); // Should be ~26x
        assert!(upwind_mult < 0.1); // Should be heavily suppressed to 0.05
        assert!(downwind_mult > 5.0 * upwind_mult); // At least 5x difference (conservative)
    }

    #[test]
    fn test_vertical_spread() {
        let fuel = Fuel::dry_grass();
        let lower = FuelElement::new(
            1,
            Vec3::new(0.0, 0.0, 0.0),
            fuel.clone(),
            1.0,
            FuelPart::GroundVegetation,
            None,
        );
        let upper = FuelElement::new(
            2,
            Vec3::new(0.0, 0.0, 5.0),
            fuel,
            1.0,
            FuelPart::Crown,
            None,
        );

        let climb_factor = vertical_spread_factor(&lower, &upper);
        let descend_factor = vertical_spread_factor(&upper, &lower);

        // Fire should climb faster than it descends
        assert!(climb_factor > 2.0);
        assert!(descend_factor < 1.0);
        assert!(climb_factor > 2.0 * descend_factor);
    }

    #[test]
    fn test_slope_effect() {
        let fuel = Fuel::dry_grass();
        let lower = FuelElement::new(
            1,
            Vec3::new(0.0, 0.0, 0.0),
            fuel.clone(),
            1.0,
            FuelPart::GroundVegetation,
            None,
        );
        let upper = FuelElement::new(
            2,
            Vec3::new(10.0, 0.0, 5.0),
            fuel,
            1.0,
            FuelPart::GroundVegetation,
            None,
        );

        let uphill = slope_spread_multiplier(&lower, &upper);
        let downhill = slope_spread_multiplier(&upper, &lower);

        // Uphill should be faster
        assert!(uphill > 1.5);
        assert!(downhill < 1.0);
    }
}
