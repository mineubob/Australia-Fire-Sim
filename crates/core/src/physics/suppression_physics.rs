//! Fire suppression direct application
//!
//! Implements direct suppression agent application including:
//! - Immediate water/retardant application at coordinates
//! - Realistic cooling and humidity effects
//! - Support for multiple agent types

use crate::core_types::element::Vec3;
use crate::grid::SimulationGrid;
use serde::{Deserialize, Serialize};

/// Types of suppression agents
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SuppressionAgent {
    Water,
    ShortTermRetardant,
    LongTermRetardant,
    Foam,
}

/// Physical constants for suppression
const WATER_LATENT_HEAT: f32 = 2260.0; // kJ/kg - latent heat of vaporization
const WATER_SPECIFIC_HEAT: f32 = 4.18; // kJ/(kg·K)
const WATER_BOILING_POINT: f32 = 100.0; // °C
const SHORT_TERM_RETARDANT_COOLING: f32 = 2500.0; // kJ/kg
const LONG_TERM_RETARDANT_COOLING: f32 = 1800.0; // kJ/kg - base cooling
const RETARDANT_COATING_BENEFIT: f32 = 500.0; // kJ/kg - additional benefit from coating
const FOAM_COOLING: f32 = 3000.0; // kJ/kg - high effectiveness due to coverage

/// Calculate cooling capacity for a suppression agent
///
/// This function computes the total cooling capacity of a suppression agent
/// based on its type and temperature. For water, it includes both latent heat
/// of vaporization and sensible heat capacity.
///
/// # Parameters
/// - `agent_type`: Type of suppression agent
/// - `temperature`: Current temperature of the agent in °C
///
/// # Returns
/// Cooling capacity in kJ per kg
fn calculate_cooling_capacity(agent_type: SuppressionAgent, temperature: f32) -> f32 {
    match agent_type {
        SuppressionAgent::Water => {
            // Latent heat of vaporization + sensible heat to reach boiling point
            let sensible = WATER_SPECIFIC_HEAT * (WATER_BOILING_POINT - temperature);
            WATER_LATENT_HEAT + sensible
        }
        SuppressionAgent::ShortTermRetardant => {
            // Similar to water but with fire-retarding chemicals
            SHORT_TERM_RETARDANT_COOLING
        }
        SuppressionAgent::LongTermRetardant => {
            // Long-term retardants form coating that provides additional benefit
            LONG_TERM_RETARDANT_COOLING + RETARDANT_COATING_BENEFIT
        }
        SuppressionAgent::Foam => {
            // Foam insulates and cools - high effectiveness due to coverage
            FOAM_COOLING
        }
    }
}

/// Apply suppression directly to grid at specified coordinates without physics simulation
///
/// This method immediately applies suppression agent to the grid at the given position,
/// bypassing the physics-based droplet simulation. Useful for direct application
/// such as ground crews or instant effects.
///
/// # Parameters
/// - `position`: World coordinates (x, y, z) where suppression is applied
/// - `mass`: Mass of suppression agent in kg
/// - `agent_type`: Type of suppression agent (Water, Retardant, Foam)
/// - `grid`: Mutable reference to the simulation grid
pub(crate) fn apply_suppression_direct(
    position: Vec3,
    mass: f32,
    agent_type: SuppressionAgent,
    grid: &mut SimulationGrid,
) {
    // Get values we need before borrowing mutably
    let cell_volume = grid.cell_size.powi(3);
    let ambient_temp = grid.ambient_temperature;

    if let Some(cell) = grid.cell_at_position_mut(position) {
        // Add suppression agent to cell
        let concentration_increase = mass / cell_volume;
        cell.suppression_agent += concentration_increase;

        // Cooling effect based on agent type
        // Typical suppression agent delivery temperature (°C)
        // Water from trucks: 15-25°C, aerial drops: 10-20°C
        let agent_temp = 20.0;
        let cooling_capacity = calculate_cooling_capacity(agent_type, agent_temp);

        let cooling_kj = mass * cooling_capacity;
        let air_mass = cell.air_density() * cell_volume;
        const SPECIFIC_HEAT_AIR: f32 = 1.005; // kJ/(kg·K) - physical constant
        let temp_drop = cooling_kj / (air_mass * SPECIFIC_HEAT_AIR);

        cell.temperature = (cell.temperature - f64::from(temp_drop)).max(*ambient_temp);

        // Increase humidity (water vapor)
        if matches!(agent_type, SuppressionAgent::Water | SuppressionAgent::Foam) {
            // Immediate evaporation fraction (varies by agent type and temperature)
            let evaporation_fraction = 0.1; // 10% for water at ambient conditions
            let vapor_added = mass * evaporation_fraction;
            cell.water_vapor += vapor_added / cell_volume;

            // Humidity increases
            // Max vapor capacity depends on temperature (Clausius-Clapeyron)
            // At 20°C: ~17 g/m³, at 30°C: ~30 g/m³, at 40°C: ~51 g/m³
            let temp_celsius = cell.temperature.max(0.0);
            let max_vapor = 0.017 * (1.0 + (temp_celsius - 20.0) * 0.07); // Temperature-dependent saturation
            let vapor_fraction = (cell.water_vapor / max_vapor).min(1.0);
            cell.humidity = cell.humidity.max(vapor_fraction);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::TerrainData;
    use approx::assert_relative_eq;

    #[test]
    fn test_direct_suppression_application() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let mut grid = SimulationGrid::new(100.0, 100.0, 50.0, 10.0, terrain);

        let position = Vec3::new(50.0, 50.0, 10.0);
        let mass = 10.0; // 10 kg

        let initial_temp = grid.cell_at_position(position).unwrap().temperature;

        // Apply water directly without physics simulation
        apply_suppression_direct(position, mass, SuppressionAgent::Water, &mut grid);

        // Should cool the cell
        let final_temp = grid.cell_at_position(position).unwrap().temperature;
        assert!(final_temp <= initial_temp);

        // Suppression agent should be present
        let agent = grid.cell_at_position(position).unwrap().suppression_agent;
        assert!(agent > 0.0);

        // Humidity should increase for water
        let humidity = grid.cell_at_position(position).unwrap().humidity;
        assert!(humidity > 0.0);
    }

    #[test]
    fn test_direct_suppression_different_agents() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);

        // Test water
        let mut grid_water = SimulationGrid::new(100.0, 100.0, 50.0, 10.0, terrain.clone());
        let position = Vec3::new(50.0, 50.0, 10.0);
        apply_suppression_direct(position, 5.0, SuppressionAgent::Water, &mut grid_water);
        let water_agent = grid_water
            .cell_at_position(position)
            .unwrap()
            .suppression_agent;

        // Test foam
        let mut grid_foam = SimulationGrid::new(100.0, 100.0, 50.0, 10.0, terrain.clone());
        apply_suppression_direct(position, 5.0, SuppressionAgent::Foam, &mut grid_foam);
        let foam_agent = grid_foam
            .cell_at_position(position)
            .unwrap()
            .suppression_agent;

        // Test retardant
        let mut grid_retardant = SimulationGrid::new(100.0, 100.0, 50.0, 10.0, terrain);
        apply_suppression_direct(
            position,
            5.0,
            SuppressionAgent::LongTermRetardant,
            &mut grid_retardant,
        );
        let retardant_agent = grid_retardant
            .cell_at_position(position)
            .unwrap()
            .suppression_agent;

        // All should have suppression agent applied
        assert!(water_agent > 0.0);
        assert!(foam_agent > 0.0);
        assert!(retardant_agent > 0.0);

        // Should be roughly equal since same mass was applied
        assert_relative_eq!(water_agent, foam_agent, epsilon = 0.001);
        assert_relative_eq!(water_agent, retardant_agent, epsilon = 0.001);
    }

    #[test]
    fn test_cooling_capacity() {
        // Test that different agents have appropriate cooling capacities
        let water_cooling = calculate_cooling_capacity(SuppressionAgent::Water, 20.0);
        let foam_cooling = calculate_cooling_capacity(SuppressionAgent::Foam, 20.0);
        let retardant_cooling =
            calculate_cooling_capacity(SuppressionAgent::LongTermRetardant, 20.0);

        // Water: ~2600 kJ/kg (2260 + 4.18*(100-20))
        assert!(water_cooling > 2500.0 && water_cooling < 2700.0);

        // Foam: 3000 kJ/kg (highest)
        assert_relative_eq!(foam_cooling, 3000.0, epsilon = 0.1);

        // Long-term retardant: 2300 kJ/kg (1800 + 500)
        assert_relative_eq!(retardant_cooling, 2300.0, epsilon = 0.1);

        // Foam should have highest cooling capacity
        assert!(foam_cooling > water_cooling);
        assert!(foam_cooling > retardant_cooling);
    }
}
