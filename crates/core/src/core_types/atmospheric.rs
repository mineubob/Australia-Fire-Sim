//! Coupling between discrete fuel elements and atmospheric grid
//!
//! Handles heat/mass/gas exchange between burning fuel elements and grid cells,
//! enabling realistic fire-atmosphere interaction.

use crate::core_types::element::{FuelElement, Vec3};
use crate::grid::SimulationGrid;
use crate::physics::combustion_physics::oxygen_limited_burn_rate;
use rayon::prelude::*;

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

/// Update grid wind field based on terrain and base wind
/// Uses precomputed terrain cache for performance
/// Parallelized for multi-core systems
pub(crate) fn update_wind_field(grid: &mut SimulationGrid, base_wind: Vec3, _dt: f32) {
    // Skip update if wind hasn't changed significantly
    let wind_delta = (base_wind - grid.last_base_wind).norm();
    if wind_delta < 0.1 {
        return; // Wind barely changed, skip expensive update
    }
    grid.last_base_wind = base_wind;

    // Wind modification by terrain (channeling, blocking, acceleration)
    // Process in parallel for performance on large grids
    let nx = grid.nx;
    let ny = grid.ny;
    let _nz = grid.nz;
    let cell_size = grid.cell_size;
    let _ambient_temp = grid.ambient_temperature;
    let terrain_cache = &grid.terrain_cache;

    // Parallel processing of grid cells
    grid.cells
        .par_chunks_mut(nx)
        .enumerate()
        .for_each(|(chunk_idx, chunk)| {
            let iz = chunk_idx / ny;
            let iy = chunk_idx % ny;

            for (ix, cell) in chunk.iter_mut().enumerate() {
                // Use cached terrain properties - eliminates expensive slope_at/aspect_at calls
                let terrain_slope = terrain_cache.slope_at_grid(ix, iy);
                let terrain_aspect = terrain_cache.aspect_at_grid(ix, iy);

                // Wind speed increases with height above terrain
                let height_above_terrain = (iz as f32 * cell_size) - cell.elevation;
                let height_factor = if height_above_terrain > 0.0 {
                    1.0 + (height_above_terrain / 10.0).min(0.5)
                } else {
                    0.5 // Below terrain, reduced wind
                };

                // Terrain channeling effect
                let wind_direction = base_wind.xy().normalize();
                let terrain_aspect_rad = terrain_aspect.to_radians();
                let aspect_vec = Vec3::new(terrain_aspect_rad.sin(), terrain_aspect_rad.cos(), 0.0);

                let alignment = wind_direction.dot(&aspect_vec.xy());
                let channeling_factor = if terrain_slope > 15.0 {
                    1.0 + alignment.abs() * 0.3 // Channeling in valleys
                } else {
                    1.0
                };

                // Apply factors
                cell.wind = base_wind * height_factor * channeling_factor;
            }
        });
}

/// Simulate smoke/heat plume rising from fire
pub(crate) fn simulate_plume_rise(grid: &mut SimulationGrid, source_positions: &[Vec3], dt: f32) {
    // For each burning element position, create upward transport of heat and smoke
    for pos in source_positions {
        if let Some(source_cell) = grid.cell_at_position(*pos) {
            let source_temp = source_cell.temperature;
            let source_smoke = source_cell.smoke_particles;

            if source_temp > grid.ambient_temperature + 50.0 {
                // Calculate plume rise velocity
                let temp_excess = source_temp - grid.ambient_temperature;
                let buoyancy_vel = (2.0 * 9.81 * temp_excess / grid.ambient_temperature).sqrt();

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
