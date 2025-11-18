//! Chemistry-based combustion physics with Arrhenius kinetics
//!
//! Implements realistic combustion modeling including:
//! - Arrhenius ignition kinetics
//! - Oxygen-limited reaction rates
//! - Multi-band radiation (visible, IR, UV)
//! - Combustion product generation

use crate::core_types::element::FuelElement;
use crate::grid::GridCell;

/// Universal gas constant (J/(mol·K))
const R_UNIVERSAL: f32 = 8.314;

/// Arrhenius parameters for wood combustion
const ACTIVATION_ENERGY_WOOD: f32 = 140000.0; // J/mol (typical for cellulose)
const PRE_EXPONENTIAL_FACTOR: f32 = 1e8; // 1/s

/// Stoichiometric coefficients for complete combustion
/// C6H10O5 (cellulose) + 6 O2 -> 6 CO2 + 5 H2O
const O2_PER_KG_FUEL: f32 = 1.33; // kg O2 per kg fuel
const CO2_PER_KG_FUEL: f32 = 1.47; // kg CO2 per kg fuel
const H2O_PER_KG_FUEL: f32 = 0.56; // kg H2O per kg fuel

/// Incomplete combustion produces CO (oxygen-starved conditions)
const CO_FRACTION_INCOMPLETE: f32 = 0.3; // 30% becomes CO instead of CO2

/// Smoke particle generation rate
const SMOKE_PER_KG_FUEL: f32 = 0.02; // kg particulates per kg fuel

/// Calculate Arrhenius reaction rate
/// k = A × exp(-Ea / (R × T))
pub fn arrhenius_rate(temperature_celsius: f32, activation_energy: f32) -> f32 {
    let temp_kelvin = temperature_celsius + 273.15;

    if temp_kelvin <= 273.15 {
        return 0.0;
    }

    let exponent = -activation_energy / (R_UNIVERSAL * temp_kelvin);
    PRE_EXPONENTIAL_FACTOR * exponent.exp()
}

/// Calculate ignition probability using Arrhenius kinetics
/// Probabilistic ignition based on temperature and time
pub fn calculate_ignition_probability(element: &FuelElement, dt: f32) -> f32 {
    if element.temperature < element.fuel.ignition_temperature {
        return 0.0;
    }

    // Temperature excess above ignition point
    let temp_excess = element.temperature - element.fuel.ignition_temperature;

    // Arrhenius-based ignition rate
    let rate = arrhenius_rate(element.temperature, ACTIVATION_ENERGY_WOOD);

    // Moisture inhibits ignition
    let moisture_factor = (1.0 - element.moisture_fraction).max(0.0);

    // Probability increases with temperature and time
    let base_prob = rate * dt * moisture_factor * 0.001;

    // Boost for high temperature excesses
    let temp_boost = 1.0 + (temp_excess / 100.0).min(5.0);

    (base_prob * temp_boost).min(1.0)
}

/// Calculate oxygen-limited combustion rate
/// Returns fraction of maximum burn rate based on available O2
pub fn oxygen_limited_burn_rate(fuel_burn_rate: f32, cell: &GridCell, cell_volume: f32) -> f32 {
    // Oxygen required for stoichiometric combustion (kg/s)
    let o2_required = fuel_burn_rate * O2_PER_KG_FUEL;

    // Oxygen available in cell (kg)
    let o2_available = cell.oxygen * cell_volume;

    // Limit burn rate by oxygen availability
    if o2_required <= 0.0 {
        return 1.0;
    }

    let o2_ratio = o2_available / o2_required;

    // Smooth limiting function
    if o2_ratio >= 1.0 {
        1.0 // Sufficient oxygen
    } else if o2_ratio <= 0.15 {
        0.0 // Too little oxygen for combustion
    } else {
        // Partial combustion (oxygen-starved)
        (o2_ratio - 0.15) / 0.85
    }
}

/// Calculate combustion products for a burning fuel element
pub struct CombustionProducts {
    pub co2_produced: f32,   // kg
    pub co_produced: f32,    // kg
    pub h2o_produced: f32,   // kg
    pub smoke_produced: f32, // kg
    pub heat_released: f32,  // kJ
    pub o2_consumed: f32,    // kg
}

pub fn calculate_combustion_products(
    fuel_consumed: f32,
    cell: &GridCell,
    fuel_heat_content: f32,
) -> CombustionProducts {
    // Check oxygen availability
    let oxygen_completeness = if cell.oxygen > 0.195 {
        1.0 // Complete combustion
    } else if cell.oxygen > 0.1 {
        // Incomplete combustion (more CO, less CO2)
        (cell.oxygen - 0.1) / 0.095
    } else {
        0.0 // Smoldering only
    };

    // Oxygen consumed
    let o2_consumed = fuel_consumed * O2_PER_KG_FUEL * oxygen_completeness;

    // Complete combustion products
    let co2_complete = fuel_consumed * CO2_PER_KG_FUEL * oxygen_completeness;
    let h2o_complete = fuel_consumed * H2O_PER_KG_FUEL * oxygen_completeness;

    // Incomplete combustion adjustment
    let co_fraction = (1.0 - oxygen_completeness) * CO_FRACTION_INCOMPLETE;
    let co_produced = co2_complete * co_fraction;
    let co2_produced = co2_complete * (1.0 - co_fraction);

    // Smoke increases with incomplete combustion
    let smoke_base = fuel_consumed * SMOKE_PER_KG_FUEL;
    let smoke_produced = smoke_base * (1.0 + 2.0 * (1.0 - oxygen_completeness));

    // Heat release (reduced for incomplete combustion)
    let combustion_efficiency = 0.6 + 0.4 * oxygen_completeness;
    let heat_released = fuel_consumed * fuel_heat_content * combustion_efficiency;

    CombustionProducts {
        co2_produced,
        co_produced,
        h2o_produced: h2o_complete,
        smoke_produced,
        heat_released,
        o2_consumed,
    }
}

/// Multi-band radiation calculation
/// Splits thermal radiation into visible, IR, and UV bands
pub struct RadiationBands {
    pub visible: f32,     // W/m² (400-700 nm)
    pub infrared: f32,    // W/m² (700 nm - 1 mm)
    pub ultraviolet: f32, // W/m² (10-400 nm)
}

pub fn calculate_radiation_bands(temperature_celsius: f32, emissivity: f32) -> RadiationBands {
    const STEFAN_BOLTZMANN: f32 = 5.67e-8;

    let temp_kelvin = temperature_celsius + 273.15;
    let total_radiance = STEFAN_BOLTZMANN * emissivity * temp_kelvin.powi(4);

    // Wien's displacement law: λ_max = 2.898×10^-3 / T
    let wavelength_max_m = 2.898e-3 / temp_kelvin;
    let _wavelength_max_nm = wavelength_max_m * 1e9; // For reference/future use

    // Approximate band fractions using Planck's law simplification
    // These are empirically derived fractions for different temperature ranges
    let (visible_frac, uv_frac) = if temperature_celsius > 1500.0 {
        // Very hot (white hot) - significant visible and UV
        (0.25, 0.05)
    } else if temperature_celsius > 1000.0 {
        // Hot (yellow-orange) - mostly visible
        (0.15, 0.01)
    } else if temperature_celsius > 600.0 {
        // Medium (red hot) - mostly IR with some visible
        (0.05, 0.001)
    } else {
        // Cool - all IR
        (0.0, 0.0)
    };

    let infrared_frac = 1.0 - visible_frac - uv_frac;

    RadiationBands {
        visible: total_radiance * visible_frac,
        infrared: total_radiance * infrared_frac,
        ultraviolet: total_radiance * uv_frac,
    }
}

/// Calculate flame color based on temperature
pub fn flame_color_temperature(temperature_celsius: f32) -> [f32; 3] {
    // RGB color approximation based on blackbody radiation

    if temperature_celsius < 500.0 {
        [0.0, 0.0, 0.0] // No visible flame
    } else if temperature_celsius < 700.0 {
        // Dark red
        let intensity = (temperature_celsius - 500.0) / 200.0;
        [intensity * 0.5, 0.0, 0.0]
    } else if temperature_celsius < 900.0 {
        // Red to orange
        let t = (temperature_celsius - 700.0) / 200.0;
        [0.8, t * 0.3, 0.0]
    } else if temperature_celsius < 1100.0 {
        // Orange to yellow
        let t = (temperature_celsius - 900.0) / 200.0;
        [1.0, 0.5 + t * 0.5, 0.0]
    } else if temperature_celsius < 1300.0 {
        // Yellow to white
        let t = (temperature_celsius - 1100.0) / 200.0;
        [1.0, 1.0, t * 0.7]
    } else {
        // White hot
        [1.0, 1.0, 0.9]
    }
}

/// Calculate convective heat transfer coefficient (W/(m²·K))
/// Based on natural convection for vertical surfaces
pub fn natural_convection_coefficient(temp_diff: f32, height: f32) -> f32 {
    if temp_diff <= 0.0 || height <= 0.0 {
        return 5.0; // Minimum forced convection
    }

    // Grashof number (dimensionless) - simplified
    let g = 9.81; // m/s²
    let beta = 1.0 / 300.0; // Thermal expansion coefficient (1/K)
    let nu = 1.5e-5; // Kinematic viscosity of air (m²/s)

    let gr = (g * beta * temp_diff * height.powi(3)) / (nu * nu);

    // Rayleigh number
    let pr = 0.7; // Prandtl number for air
    let ra = gr * pr;

    // Nusselt number correlation for natural convection
    let nu_num = if ra < 1e9 {
        0.59 * ra.powf(0.25)
    } else {
        0.1 * ra.powf(0.33)
    };

    // Heat transfer coefficient
    let k_air = 0.026; // Thermal conductivity of air (W/(m·K))
    (nu_num * k_air / height).max(5.0)
}

/// Calculate fire intensity using modified Byram's equation
/// Accounts for oxygen limitation
pub fn calculate_fire_intensity(
    fuel_consumed_rate: f32,
    heat_content: f32,
    oxygen_factor: f32,
) -> f32 {
    // Byram's intensity: I = H × w × r
    // where H = heat content, w = fuel consumed per unit area, r = rate of spread
    // Simplified: I ≈ fuel_rate × heat_content

    let base_intensity = fuel_consumed_rate * heat_content;

    // Apply oxygen limitation
    base_intensity * oxygen_factor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::element::{FuelElement, FuelPart, Vec3};
    use crate::core_types::fuel::Fuel;
    use crate::grid::GridCell;

    #[test]
    fn test_arrhenius_rate() {
        // Rate should increase exponentially with temperature
        let rate_300 = arrhenius_rate(300.0, ACTIVATION_ENERGY_WOOD);
        let rate_500 = arrhenius_rate(500.0, ACTIVATION_ENERGY_WOOD);
        let rate_700 = arrhenius_rate(700.0, ACTIVATION_ENERGY_WOOD);

        assert!(rate_500 > rate_300);
        assert!(rate_700 > rate_500);
        assert!(rate_700 > rate_300 * 10.0);
    }

    #[test]
    fn test_ignition_probability() {
        let fuel = Fuel::dry_grass();
        let element = FuelElement::new(
            1,
            Vec3::zeros(),
            fuel,
            1.0,
            FuelPart::GroundVegetation,
            None,
        );

        let mut elem_cold = element.clone();
        elem_cold.temperature = 200.0;
        let prob_cold = calculate_ignition_probability(&elem_cold, 1.0);
        assert_eq!(prob_cold, 0.0); // Below ignition temp

        let mut elem_hot = element.clone();
        elem_hot.temperature = 600.0;
        elem_hot.moisture_fraction = 0.0;
        let prob_hot = calculate_ignition_probability(&elem_hot, 1.0);
        assert!(prob_hot > 0.0);
    }

    #[test]
    fn test_combustion_products() {
        let mut cell = GridCell::new(0.0);
        cell.oxygen = 0.273;

        let products = calculate_combustion_products(1.0, &cell, 18000.0);

        // Should produce CO2 and consume O2
        assert!(products.co2_produced > 1.0);
        assert!(products.o2_consumed > 1.0);
        assert!(products.heat_released > 10000.0);

        // Water vapor should be produced
        assert!(products.h2o_produced > 0.0);
    }

    #[test]
    fn test_incomplete_combustion() {
        let mut cell_low_o2 = GridCell::new(0.0);
        cell_low_o2.oxygen = 0.15; // Low oxygen

        let products = calculate_combustion_products(1.0, &cell_low_o2, 18000.0);

        // Incomplete combustion produces more CO and smoke
        assert!(products.co_produced > 0.0);
        assert!(products.smoke_produced > SMOKE_PER_KG_FUEL);

        // Heat release is reduced
        assert!(products.heat_released < 18000.0);
    }

    #[test]
    fn test_radiation_bands() {
        let bands_hot = calculate_radiation_bands(1200.0, 0.95);

        // Hot flames have significant visible light
        assert!(bands_hot.visible > 0.0);
        assert!(bands_hot.infrared > bands_hot.visible);

        let bands_cool = calculate_radiation_bands(400.0, 0.95);

        // Cooler fires are mostly IR
        assert_eq!(bands_cool.visible, 0.0);
        assert!(bands_cool.infrared > 0.0);
    }

    #[test]
    fn test_flame_color() {
        let color_red = flame_color_temperature(700.0);
        assert!(color_red[0] > 0.5); // Red component
        assert!(color_red[1] < 0.1); // Little green

        let color_white = flame_color_temperature(1400.0);
        assert!(color_white[0] > 0.9); // All RGB high
        assert!(color_white[1] > 0.9);
        assert!(color_white[2] > 0.8);
    }

    #[test]
    fn test_natural_convection() {
        let h_small_diff = natural_convection_coefficient(10.0, 1.0);
        let h_large_diff = natural_convection_coefficient(200.0, 1.0);

        // Larger temperature difference = stronger convection
        assert!(h_large_diff > h_small_diff);
    }
}
