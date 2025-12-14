//! Chemistry-based combustion physics with Arrhenius kinetics
//!
//! Implements realistic combustion modeling including:
//! - Arrhenius ignition kinetics
//! - Oxygen-limited reaction rates
//! - Multi-band radiation (visible, IR, UV)
//! - Combustion product generation

use crate::grid::GridCell;

/// Stoichiometric coefficients for complete combustion
/// C6H10O5 (cellulose) + 6 O2 -> 6 CO2 + 5 H2O
const O2_PER_KG_FUEL: f32 = 1.33; // kg O2 per kg fuel
const CO2_PER_KG_FUEL: f32 = 1.47; // kg CO2 per kg fuel
const H2O_PER_KG_FUEL: f32 = 0.56; // kg H2O per kg fuel

/// Incomplete combustion produces CO (oxygen-starved conditions)
const CO_FRACTION_INCOMPLETE: f32 = 0.3; // 30% becomes CO instead of CO2

/// Smoke particle generation rate
const SMOKE_PER_KG_FUEL: f32 = 0.02; // kg particulates per kg fuel

/// Calculate oxygen-limited combustion rate
/// Returns fraction of maximum burn rate based on available O2
pub(crate) fn oxygen_limited_burn_rate(
    fuel_burn_rate: f32,
    cell: &GridCell,
    cell_volume: f32,
) -> f32 {
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
#[allow(dead_code)] // co_produced and heat_released fields
pub(crate) struct CombustionProducts {
    co2_produced: f32,   // kg
    co_produced: f32,    // kg
    h2o_produced: f32,   // kg
    smoke_produced: f32, // kg
    heat_released: f32,  // kJ
    o2_consumed: f32,    // kg
}

impl CombustionProducts {
    /// Get O2 consumed
    pub(crate) fn o2_consumed(&self) -> f32 {
        self.o2_consumed
    }

    /// Get CO2 produced
    pub(crate) fn co2_produced(&self) -> f32 {
        self.co2_produced
    }

    /// Get H2O produced
    pub(crate) fn h2o_produced(&self) -> f32 {
        self.h2o_produced
    }

    /// Get smoke produced
    pub(crate) fn smoke_produced(&self) -> f32 {
        self.smoke_produced
    }
}

pub(crate) fn calculate_combustion_products(
    fuel_consumed: f32,
    cell: &GridCell,
    fuel_heat_content: f32,
    cell_volume: f32,
) -> CombustionProducts {
    // Check oxygen availability by concentration
    let oxygen_completeness_by_concentration = if cell.oxygen > 0.195 {
        1.0 // Complete combustion
    } else if cell.oxygen > 0.1 {
        // Incomplete combustion (more CO, less CO2)
        (cell.oxygen - 0.1) / 0.095
    } else {
        0.0 // Smoldering only
    };

    // Calculate oxygen that would be consumed for this completeness level
    let o2_required = fuel_consumed * O2_PER_KG_FUEL * oxygen_completeness_by_concentration;

    // Available oxygen mass in the cell (kg)
    let o2_available = cell.oxygen * cell_volume;

    // Limit oxygen consumption to what's actually available
    // This prevents consuming more oxygen than physically exists in the cell
    let o2_consumed = o2_required.min(o2_available);

    // If we're limited by available oxygen mass (not concentration),
    // we need to recalculate completeness based on actual oxygen availability
    let actual_fuel_combusted;
    let oxygen_completeness;

    if o2_required > 0.0 && o2_consumed < o2_required {
        // Oxygen mass limited - reduce fuel combusted proportionally
        let fuel_fraction = o2_consumed / o2_required;
        actual_fuel_combusted = fuel_consumed * fuel_fraction;

        // Recalculate completeness: if we have less oxygen mass than needed,
        // the combustion becomes less complete regardless of concentration
        oxygen_completeness = oxygen_completeness_by_concentration * fuel_fraction;
    } else {
        // Not oxygen mass limited - use concentration-based completeness
        actual_fuel_combusted = fuel_consumed;
        oxygen_completeness = oxygen_completeness_by_concentration;
    }

    // Complete combustion products based on actual combustion
    let co2_complete = actual_fuel_combusted * CO2_PER_KG_FUEL * oxygen_completeness;
    let h2o_complete = actual_fuel_combusted * H2O_PER_KG_FUEL * oxygen_completeness;

    // Incomplete combustion adjustment
    let co_fraction = (1.0 - oxygen_completeness) * CO_FRACTION_INCOMPLETE;
    let co_produced = co2_complete * co_fraction;
    let co2_produced = co2_complete * (1.0 - co_fraction);

    // Smoke increases with incomplete combustion
    let smoke_base = actual_fuel_combusted * SMOKE_PER_KG_FUEL;
    let smoke_produced = smoke_base * (1.0 + 2.0 * (1.0 - oxygen_completeness));

    // Heat release (reduced for incomplete combustion)
    let combustion_efficiency = 0.6 + 0.4 * oxygen_completeness;
    let heat_released = actual_fuel_combusted * fuel_heat_content * combustion_efficiency;

    CombustionProducts {
        co2_produced,
        co_produced,
        h2o_produced: h2o_complete,
        smoke_produced,
        heat_released,
        o2_consumed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridCell;

    #[test]
    fn test_combustion_products() {
        let mut cell = GridCell::new(0.0);
        cell.oxygen = 0.273;
        let cell_volume = 100.0; // m³

        let products = calculate_combustion_products(1.0, &cell, 18000.0, cell_volume);

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
        let cell_volume = 100.0; // m³

        let products = calculate_combustion_products(1.0, &cell_low_o2, 18000.0, cell_volume);

        // Incomplete combustion produces more CO and smoke
        assert!(products.co_produced > 0.0);
        assert!(products.smoke_produced > SMOKE_PER_KG_FUEL);

        // Heat release is reduced
        assert!(products.heat_released < 18000.0);
    }

    #[test]
    fn test_oxygen_mass_limiting() {
        // Test case where oxygen mass (not concentration) is the limiting factor
        let mut cell = GridCell::new(0.0);
        cell.oxygen = 0.21; // Normal oxygen concentration (complete combustion)
        let cell_volume = 1.0; // Small cell volume: only 0.21 kg O2 available

        // High fuel consumption that would require ~13.3 kg O2 for complete combustion
        let fuel_consumed = 10.0;
        let products = calculate_combustion_products(fuel_consumed, &cell, 18000.0, cell_volume);

        // Oxygen consumption should be limited to available oxygen
        let o2_available = cell.oxygen * cell_volume;
        assert!((products.o2_consumed - o2_available).abs() < 1e-6);

        // All available oxygen should be consumed
        assert!(products.o2_consumed < fuel_consumed * O2_PER_KG_FUEL);

        // Fuel combusted should be reduced proportionally
        // o2_required ~= 10.0 * 1.33 = 13.3 kg, o2_available = 0.21 kg
        // fuel_fraction = 0.21 / 13.3 ~= 0.0158
        let expected_fuel_fraction = o2_available / (fuel_consumed * O2_PER_KG_FUEL);
        let expected_fuel_combusted = fuel_consumed * expected_fuel_fraction;
        assert!((products.o2_consumed / O2_PER_KG_FUEL - expected_fuel_combusted).abs() < 0.1);

        // Heat release should be proportionally reduced
        // Combustion efficiency with full oxygen is ~1.0, so heat ~= fuel * heat_content
        let expected_heat =
            expected_fuel_combusted * 18000.0 * (0.6 + 0.4 * expected_fuel_fraction);
        assert!((products.heat_released - expected_heat).abs() < 100.0);
    }
}
