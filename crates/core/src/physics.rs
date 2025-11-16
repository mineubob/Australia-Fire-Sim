use crate::element::{FuelElement, Vec3};

/// Stefan-Boltzmann constant (W/(m²·K⁴))
const STEFAN_BOLTZMANN: f32 = 5.67e-8;

/// Calculate radiation heat flux from source to target
pub fn calculate_radiation_flux(source: &FuelElement, target: &FuelElement, distance: f32) -> f32 {
    if !source.ignited || distance <= 0.0 {
        return 0.0;
    }
    
    // Convert temperature to Kelvin
    let temp_k = source.temperature + 273.15;
    
    // View factor (simplified - assumes reasonable line of sight)
    let source_area = source.fuel.surface_area_to_volume * source.fuel_remaining.sqrt();
    let view_factor = source_area / (4.0 * std::f32::consts::PI * distance * distance);
    let view_factor = view_factor.min(1.0);
    
    // Radiant flux (simplified (T/1000)^4 for performance)
    let flux = STEFAN_BOLTZMANN * (temp_k / 1000.0).powi(4) * view_factor * 10000.0;
    
    // Convert W/m² to kJ delivered over target area
    let target_area = target.fuel.surface_area_to_volume;
    let heat_kj = flux * target_area * 0.001; // per second
    
    heat_kj
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
    if wind.magnitude() < 0.1 {
        return 1.0;
    }
    
    let direction = (to - from).normalize();
    let alignment = direction.dot(&wind.normalize());
    let wind_speed_ms = wind.magnitude();
    
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
                    + (to.position.y - from.position.y).powi(2)).sqrt();
    
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
    use crate::fuel::BarkType;
    
    // Base threshold from fuel type
    let base_threshold = element.fuel.crown_fire_threshold;
    
    // CRITICAL: Stringybark dramatically lowers threshold
    let threshold = if matches!(element.fuel.bark_type, BarkType::Stringybark) {
        let bark_boost = element.fuel.bark_ladder_intensity; // 600-700 kW/m
        
        // Can cause crown fire at 30% normal intensity!
        if fire_intensity + bark_boost > 300.0 {
            return true; // GUARANTEED crown transition
        }
        base_threshold * 0.3
    } else {
        base_threshold
    };
    
    // Wind increases crown fire probability
    let wind_factor = 1.0 + wind_speed * 0.05;
    
    fire_intensity * wind_factor > threshold
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuel::Fuel;
    use crate::element::FuelPart;
    
    #[test]
    fn test_wind_directionality() {
        let from = Vec3::new(0.0, 0.0, 0.0);
        let to_downwind = Vec3::new(10.0, 0.0, 0.0);
        let to_upwind = Vec3::new(-10.0, 0.0, 0.0);
        let wind = Vec3::new(10.0, 0.0, 0.0); // 10 m/s wind in +X direction
        
        let downwind_mult = wind_radiation_multiplier(from, to_downwind, wind);
        let upwind_mult = wind_radiation_multiplier(from, to_upwind, wind);
        
        println!("Downwind multiplier: {:.2}, Upwind multiplier: {:.2}", downwind_mult, upwind_mult);
        
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
        let lower = FuelElement::new(1, Vec3::new(0.0, 0.0, 0.0), fuel.clone(), 1.0, FuelPart::GroundVegetation, None);
        let upper = FuelElement::new(2, Vec3::new(0.0, 0.0, 5.0), fuel, 1.0, FuelPart::Crown, None);
        
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
        let lower = FuelElement::new(1, Vec3::new(0.0, 0.0, 0.0), fuel.clone(), 1.0, FuelPart::GroundVegetation, None);
        let upper = FuelElement::new(2, Vec3::new(10.0, 0.0, 5.0), fuel, 1.0, FuelPart::GroundVegetation, None);
        
        let uphill = slope_spread_multiplier(&lower, &upper);
        let downhill = slope_spread_multiplier(&upper, &lower);
        
        // Uphill should be faster
        assert!(uphill > 1.5);
        assert!(downhill < 1.0);
    }
}
