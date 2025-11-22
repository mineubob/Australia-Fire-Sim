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
    pub co2_produced: f32,   // kg
    pub co_produced: f32,    // kg
    pub h2o_produced: f32,   // kg
    pub smoke_produced: f32, // kg
    pub heat_released: f32,  // kJ
    pub o2_consumed: f32,    // kg
}

pub(crate) fn calculate_combustion_products(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridCell;

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
}
