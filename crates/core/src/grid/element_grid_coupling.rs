//! Coupling between discrete fuel elements and atmospheric grid
//!
//! Handles heat/mass/gas exchange between burning fuel elements and grid cells,
//! enabling realistic fire-atmosphere interaction.

use crate::core_types::element::{FuelElement, Vec3};
use crate::grid::SimulationGrid;
use crate::physics::combustion_physics::{calculate_combustion_products, oxygen_limited_burn_rate};
use rayon::prelude::*;

/// Transfer heat from burning element to surrounding grid cells
pub(crate) fn transfer_heat_to_grid(element: &FuelElement, grid: &mut SimulationGrid, dt: f32) {
    if !element.ignited || element.fuel_remaining <= 0.0 {
        return;
    }

    let element_pos = element.position;
    let cell_volume = grid.cell_size.powi(3);

    // Get element's cell
    if let Some(cell) = grid.cell_at_position_mut(element_pos) {
        // Calculate heat transfer based on temperature difference
        let temp_diff = element.temperature - cell.temperature;

        if temp_diff > 0.0 {
            // Convective heat transfer coefficient
            let h = 50.0; // W/(m²·K) - typical for fire convection
            let surface_area = element.fuel.surface_area_to_volume * element.fuel_remaining.sqrt();

            // Heat transfer rate (W)
            let heat_transfer_w = h * surface_area * temp_diff;

            // Energy transferred (kJ)
            let heat_kj = heat_transfer_w * dt * 0.001;

            // Update cell temperature
            let air_mass = cell.air_density() * cell_volume;
            let specific_heat_air = 1.005; // kJ/(kg·K)
            let temp_rise = heat_kj / (air_mass * specific_heat_air);

            cell.temperature += temp_rise;
            cell.radiation_flux += heat_transfer_w;
        }
    }

    // Also heat neighboring cells (radiation)
    let radiation_radius = (element.temperature / 100.0).min(2.0) * grid.cell_size;
    let cells_radius = (radiation_radius / grid.cell_size).ceil() as i32;

    let cx = (element_pos.x / grid.cell_size) as i32;
    let cy = (element_pos.y / grid.cell_size) as i32;
    let cz = (element_pos.z / grid.cell_size) as i32;

    for dz in -cells_radius..=cells_radius {
        for dy in -cells_radius..=cells_radius {
            for dx in -cells_radius..=cells_radius {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue; // Skip center cell (already handled)
                }

                let ix = cx + dx;
                let iy = cy + dy;
                let iz = cz + dz;

                if ix >= 0
                    && ix < grid.nx as i32
                    && iy >= 0
                    && iy < grid.ny as i32
                    && iz >= 0
                    && iz < grid.nz as i32
                {
                    let cell_center = Vec3::new(
                        ix as f32 * grid.cell_size + grid.cell_size / 2.0,
                        iy as f32 * grid.cell_size + grid.cell_size / 2.0,
                        iz as f32 * grid.cell_size + grid.cell_size / 2.0,
                    );

                    let distance = (cell_center - element_pos).magnitude();
                    if distance < radiation_radius {
                        // Radiative heat transfer (simplified)
                        let radiation_factor = 1.0 - (distance / radiation_radius).powi(2);
                        let heat_flux = element.temperature * 0.1 * radiation_factor;

                        if let Some(cell) = grid.cell_at_mut(ix as usize, iy as usize, iz as usize)
                        {
                            cell.radiation_flux += heat_flux;
                        }
                    }
                }
            }
        }
    }
}

/// Transfer combustion products from element to grid
pub(crate) fn transfer_combustion_products_to_grid(
    element: &FuelElement,
    fuel_consumed: f32,
    grid: &mut SimulationGrid,
) {
    if fuel_consumed <= 0.0 {
        return;
    }

    let element_pos = element.position;
    let cell_volume = grid.cell_size.powi(3);

    if let Some(cell) = grid.cell_at_position_mut(element_pos) {
        // Calculate combustion products
        let products =
            calculate_combustion_products(fuel_consumed, cell, element.fuel.heat_content);

        // Update cell composition
        cell.oxygen -= products.o2_consumed / cell_volume;
        cell.oxygen = cell.oxygen.max(0.0);

        cell.carbon_dioxide += products.co2_produced / cell_volume;
        cell.carbon_monoxide += products.co_produced / cell_volume;
        cell.water_vapor += products.h2o_produced / cell_volume;
        cell.smoke_particles += products.smoke_produced / cell_volume;

        // Heat release increases temperature
        let air_mass = cell.air_density() * cell_volume;
        let specific_heat_air = 1.005; // kJ/(kg·K)
        let temp_rise = products.heat_released / (air_mass * specific_heat_air);
        cell.temperature += temp_rise;
    }
}

/// Apply grid conditions to fuel element (wind, humidity, oxygen)
pub(crate) fn apply_grid_to_element(element: &mut FuelElement, grid: &SimulationGrid) {
    let interpolated = grid.interpolate_at_position(element.position);

    // Update element's local conditions
    // Wind affects ember trajectory (handled elsewhere)
    // Humidity affects moisture content
    if interpolated.humidity > element.moisture_fraction {
        // Fuel absorbs moisture from air (slow process)
        let moisture_uptake_rate = 0.0001; // kg/(kg·s) per humidity difference
        let moisture_increase =
            (interpolated.humidity - element.moisture_fraction) * moisture_uptake_rate;
        element.moisture_fraction =
            (element.moisture_fraction + moisture_increase).min(element.fuel.base_moisture * 1.5);
    }

    // Suppression agent cools element
    if interpolated.suppression_agent > 0.0 {
        let cooling_rate = interpolated.suppression_agent * 1000.0; // kJ per kg agent
        let mass = element.fuel_remaining;
        let temp_drop = cooling_rate / (mass * element.fuel.specific_heat);
        element.temperature = (element.temperature - temp_drop).max(interpolated.temperature);
    }
}

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

/// Calculate buoyancy force on element from grid temperature gradient
pub(crate) fn calculate_buoyancy_on_element(element: &FuelElement, grid: &SimulationGrid) -> Vec3 {
    let pos = element.position;
    let cell_size = grid.cell_size;

    // Sample cells above and below
    let pos_above = pos + Vec3::new(0.0, 0.0, cell_size);
    let pos_below = pos - Vec3::new(0.0, 0.0, cell_size);

    let cell_center = grid.interpolate_at_position(pos);
    let cell_above = grid.interpolate_at_position(pos_above);
    let cell_below = grid.interpolate_at_position(pos_below);

    // Temperature gradient in vertical direction (for reference/future use)
    let _temp_gradient = (cell_above.temperature - cell_below.temperature) / (2.0 * cell_size);

    // Buoyancy force (simplified)
    let buoyancy_strength = cell_center.buoyancy_force(grid.ambient_temperature);

    Vec3::new(0.0, 0.0, buoyancy_strength * 0.1) // Vertical force
}

/// Interpolate wind velocity at element position
pub(crate) fn get_wind_at_element(element: &FuelElement, grid: &SimulationGrid) -> Vec3 {
    let interpolated = grid.interpolate_at_position(element.position);
    interpolated.wind
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::element::FuelPart;
    use crate::core_types::fuel::Fuel;
    use crate::grid::TerrainData;

    #[test]
    fn test_heat_transfer_to_grid() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let mut grid = SimulationGrid::new(100.0, 100.0, 50.0, 10.0, terrain);

        let fuel = Fuel::dry_grass();
        let mut element = FuelElement::new(
            1,
            Vec3::new(50.0, 50.0, 10.0),
            fuel,
            1.0,
            FuelPart::GroundVegetation,
            None,
        );
        element.ignited = true;
        element.temperature = 800.0;

        // Transfer heat
        transfer_heat_to_grid(&element, &mut grid, 1.0);

        // Cell should be heated
        let cell = grid.cell_at_position(element.position).unwrap();
        assert!(cell.temperature > 20.0);
    }

    #[test]
    fn test_combustion_products_transfer() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let mut grid = SimulationGrid::new(100.0, 100.0, 50.0, 10.0, terrain);

        let fuel = Fuel::dry_grass();
        let element = FuelElement::new(
            1,
            Vec3::new(50.0, 50.0, 10.0),
            fuel,
            1.0,
            FuelPart::GroundVegetation,
            None,
        );

        let initial_o2 = grid.cell_at_position(element.position).unwrap().oxygen;

        // Burn a significant amount of fuel
        transfer_combustion_products_to_grid(&element, 1.0, &mut grid);

        // Oxygen should be depleted
        let final_o2 = grid.cell_at_position(element.position).unwrap().oxygen;
        assert!(final_o2 < initial_o2);

        // CO2 should increase significantly
        let co2 = grid
            .cell_at_position(element.position)
            .unwrap()
            .carbon_dioxide;
        assert!(co2 > 0.002); // Should be well above ambient
    }

    #[test]
    fn test_wind_interpolation() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let mut grid = SimulationGrid::new(100.0, 100.0, 50.0, 10.0, terrain);

        // Set base wind
        let base_wind = Vec3::new(5.0, 0.0, 0.0);
        update_wind_field(&mut grid, base_wind, 1.0);

        let fuel = Fuel::dry_grass();
        let element = FuelElement::new(
            1,
            Vec3::new(50.0, 50.0, 20.0),
            fuel,
            1.0,
            FuelPart::GroundVegetation,
            None,
        );

        let wind = get_wind_at_element(&element, &grid);

        // Wind should be non-zero
        assert!(wind.magnitude() > 0.0);
    }

    #[test]
    fn test_suppression_cooling() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let mut grid = SimulationGrid::new(100.0, 100.0, 50.0, 10.0, terrain);

        let fuel = Fuel::dry_grass();
        let mut element = FuelElement::new(
            1,
            Vec3::new(50.0, 50.0, 10.0),
            fuel,
            1.0,
            FuelPart::GroundVegetation,
            None,
        );
        element.temperature = 500.0;

        // Add suppression agent
        if let Some(cell) = grid.cell_at_position_mut(element.position) {
            cell.suppression_agent = 0.5;
        }

        let initial_temp = element.temperature;
        apply_grid_to_element(&mut element, &grid);

        // Temperature should decrease
        assert!(element.temperature < initial_temp);
    }
}
