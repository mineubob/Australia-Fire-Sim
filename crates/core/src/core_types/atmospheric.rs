//! Coupling between discrete fuel elements and atmospheric grid
//!
//! Handles heat/mass/gas exchange between burning fuel elements and grid cells,
//! enabling realistic fire-atmosphere interaction.

use crate::core_types::element::{FuelElement, Vec3};
use crate::grid::SimulationGrid;
use crate::physics::combustion_physics::oxygen_limited_burn_rate;
// no parallel helper required (previously used by update_wind_field)

/// Calculate oxygen-limited burn rate for element based on cell oxygen
pub(crate) fn get_oxygen_limited_burn_rate(
    element: &FuelElement,
    base_burn_rate: f32,
    grid: &SimulationGrid,
) -> f32 {
    if let Some(cell) = grid.cell_at_position(element.position) {
        let cell_volume = grid.cell_size.powi(3);
        oxygen_limited_burn_rate(base_burn_rate, cell, cell_volume)
    } else {
        1.0 // No limitation if outside grid
    }
}

// NOTE: previously we provided a simple terrain-modulated update helper (`update_wind_field`) used
// as a fallback when an advanced mass-consistent wind field was disabled. The simulation now
// always contains an active mass-consistent `WindField`, so this helper is no longer needed and
// has been removed.

/// Simulate smoke/heat plume rising from fire
pub(crate) fn simulate_plume_rise(grid: &mut SimulationGrid, source_positions: &[Vec3], dt: f32) {
    // For each burning element position, create upward transport of heat and smoke
    for pos in source_positions {
        if let Some(source_cell) = grid.cell_at_position(*pos) {
            let source_temp = source_cell.temperature;
            let source_smoke = source_cell.smoke_particles;

            if source_temp > *grid.ambient_temperature + 50.0 {
                // Calculate plume rise velocity
                let temp_excess = source_temp - *grid.ambient_temperature;
                let buoyancy_vel = (2.0 * 9.81 * temp_excess / *grid.ambient_temperature).sqrt();

                // Transport to cells above
                let rise_distance = buoyancy_vel * dt;
                let cells_to_rise = (rise_distance / grid.cell_size).floor() as usize;

                let cx = (pos.x / grid.cell_size) as i32;
                let cy = (pos.y / grid.cell_size) as i32;
                let cz = (pos.z / grid.cell_size) as i32;

                for dz in 1..=(cells_to_rise as i32).min(5) {
                    let target_z = cz + dz;

                    if target_z >= 0 && target_z < grid.nz as i32 {
                        // Spread plume horizontally as it rises
                        let spread_radius = dz / 2;

                        for dy in -spread_radius..=spread_radius {
                            for dx in -spread_radius..=spread_radius {
                                let tx = cx + dx;
                                let ty = cy + dy;

                                if tx >= 0 && tx < grid.nx as i32 && ty >= 0 && ty < grid.ny as i32
                                {
                                    if let Some(target_cell) = grid.cell_at_mut(
                                        tx as usize,
                                        ty as usize,
                                        target_z as usize,
                                    ) {
                                        // Dilution with height
                                        let dilution = 1.0 / (dz as f32 * dz as f32);

                                        let temp_transfer = temp_excess * 0.1 * dilution;
                                        target_cell.temperature += temp_transfer;

                                        let smoke_transfer = source_smoke * 0.1 * dilution;
                                        target_cell.smoke_particles += smoke_transfer;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
