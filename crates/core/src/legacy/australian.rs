use crate::core_types::element::FuelElement;

/// Update oil vaporization and check for explosive ignition
pub fn update_oil_vaporization(element: &mut FuelElement, _dt: f32) -> Option<ExplosionEvent> {
    if element.fuel.volatile_oil_content <= 0.0 {
        return None;
    }

    // Oil vaporizes at 170°C
    if element.temperature > element.fuel.oil_vaporization_temp {
        let vapor_mass = element.fuel.volatile_oil_content * 0.01 * element.fuel_remaining;

        // Autoignition at 232°C
        if element.temperature > element.fuel.oil_autoignition_temp {
            // EXPLOSIVE ignition (43 MJ/kg for eucalyptus oil)
            let explosion_energy = vapor_mass * 43000.0; // kJ
            let blast_radius = (explosion_energy / 1000.0).sqrt();

            return Some(ExplosionEvent {
                position: element.position,
                energy: explosion_energy,
                blast_radius,
                temperature: element.temperature + 200.0, // Additional heat from explosion
            });
        }
    }

    None
}

/// Explosion event data
#[derive(Debug, Clone)]
pub struct ExplosionEvent {
    pub position: crate::core_types::element::Vec3,
    pub energy: f32,       // kJ
    pub blast_radius: f32, // meters
    pub temperature: f32,  // °C
}

/// Calculate crown fire transition probability
pub fn calculate_crown_transition(
    element: &FuelElement,
    fire_intensity: f32,
    wind_speed: f32,
    vertical_neighbors: usize,
) -> bool {
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

    // Check vertical fuel continuity
    let vertical_continuity = (vertical_neighbors as f32) / 10.0;
    let continuity_factor = 1.0 - vertical_continuity * 0.5;

    // Wind increases crown fire probability
    let wind_factor = 1.0 + wind_speed * 0.05;

    fire_intensity * wind_factor > threshold * continuity_factor
}

/// Calculate bark ladder fuel contribution
pub fn bark_ladder_contribution(element: &FuelElement) -> f32 {
    // Use bark properties for more nuanced calculations
    let base_contribution = element.fuel.bark_ladder_intensity;
    let ladder_factor = element.fuel.bark_properties.ladder_fuel_factor;
    let flammability = element.fuel.bark_properties.flammability;

    // Contribution scaled by both ladder factor and flammability
    base_contribution * ladder_factor * flammability
}

/// Calculate spotting distance based on fuel type and conditions
pub fn calculate_spotting_distance(
    element: &FuelElement,
    wind_speed_ms: f32,
    fire_intensity: f32,
) -> f32 {
    // Base spotting distance from fuel type
    let base_distance = element.fuel.max_spotting_distance;

    // Wind dramatically increases spotting distance
    let wind_factor = 1.0 + (wind_speed_ms / 10.0).powf(1.5);

    // Fire intensity affects ember loft height
    let intensity_factor = (fire_intensity / 1000.0).sqrt().min(2.0);

    // Bark shedding rate affects ember generation
    let shedding_factor = 1.0 + element.fuel.bark_properties.shedding_rate * 0.5;

    base_distance * wind_factor * intensity_factor * shedding_factor
}

/// Check if fuel moisture is low enough for ignition
pub fn can_ignite_with_moisture(moisture_fraction: f32, moisture_of_extinction: f32) -> bool {
    moisture_fraction < moisture_of_extinction
}

/// Calculate effective heat release for eucalyptus fuels
pub fn eucalyptus_heat_release(
    base_heat: f32,
    oil_content: f32,
    temperature: f32,
    oil_vaporization_temp: f32,
) -> f32 {
    if temperature > oil_vaporization_temp && oil_content > 0.0 {
        // Oil combustion adds significant heat
        let oil_contribution = oil_content * 43000.0; // kJ/kg for eucalyptus oil
        base_heat + oil_contribution * 0.1 // 10% of oil heat added per second
    } else {
        base_heat
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::element::{FuelPart, Vec3};
    use crate::core_types::fuel::Fuel;

    #[test]
    fn test_stringybark_crown_transition() {
        let fuel = Fuel::eucalyptus_stringybark();
        let element = FuelElement::new(
            1,
            Vec3::new(0.0, 0.0, 10.0),
            fuel,
            5.0,
            FuelPart::TrunkUpper,
            None,
        );

        // Stringybark should transition at lower intensity
        let should_transition = calculate_crown_transition(&element, 400.0, 10.0, 5);
        assert!(should_transition);
    }

    #[test]
    fn test_oil_vaporization() {
        let fuel = Fuel::eucalyptus_stringybark();
        let mut element = FuelElement::new(
            1,
            Vec3::new(0.0, 0.0, 5.0),
            fuel,
            5.0,
            FuelPart::TrunkMiddle,
            None,
        );

        // Below vaporization temp - no explosion
        element.temperature = 150.0;
        let result = update_oil_vaporization(&mut element, 0.1);
        assert!(result.is_none());

        // Above autoignition temp - explosion!
        element.temperature = 250.0;
        let result = update_oil_vaporization(&mut element, 0.1);
        assert!(result.is_some());
    }

    #[test]
    fn test_bark_ladder_contribution() {
        let stringybark = Fuel::eucalyptus_stringybark();
        let smooth = Fuel::eucalyptus_smooth_bark();

        let element_stringy = FuelElement::new(
            1,
            Vec3::new(0.0, 0.0, 5.0),
            stringybark,
            5.0,
            FuelPart::TrunkMiddle,
            None,
        );

        let element_smooth = FuelElement::new(
            2,
            Vec3::new(10.0, 0.0, 5.0),
            smooth,
            5.0,
            FuelPart::TrunkMiddle,
            None,
        );

        let stringy_ladder = bark_ladder_contribution(&element_stringy);
        let smooth_ladder = bark_ladder_contribution(&element_smooth);

        // Stringybark should have much higher ladder fuel contribution
        assert!(stringy_ladder > smooth_ladder);
        assert!(stringy_ladder > 500.0);
    }

    #[test]
    fn test_spotting_distance() {
        let fuel = Fuel::eucalyptus_stringybark();
        let element = FuelElement::new(
            1,
            Vec3::new(0.0, 0.0, 5.0),
            fuel,
            5.0,
            FuelPart::Crown,
            None,
        );

        // High wind should dramatically increase spotting
        let distance_low_wind = calculate_spotting_distance(&element, 5.0, 500.0);
        let distance_high_wind = calculate_spotting_distance(&element, 20.0, 500.0);

        assert!(distance_high_wind > distance_low_wind * 2.0);
    }
}
