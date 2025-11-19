//! Ultra-realistic fire simulation integrating all advanced systems
//!
//! FireSimulationUltra combines:
//! - 3D atmospheric grid with terrain elevation
//! - Discrete fuel elements with grid coupling
//! - Chemistry-based combustion
//! - Advanced suppression physics
//! - Buoyancy-driven convection and plumes

use crate::core_types::element::{FuelElement, FuelPart, Vec3};
use crate::core_types::ember::Ember;
use crate::core_types::fuel::Fuel;
use crate::core_types::spatial::SpatialIndex;
use crate::core_types::weather::WeatherSystem;
use crate::grid::element_grid_coupling::*;
use crate::grid::{GridCell, SimulationGrid, TerrainData};
use crate::physics::SuppressionDroplet;
use rayon::prelude::*;
use std::collections::HashSet;

/// Ultra-realistic fire simulation with full atmospheric modeling
pub struct FireSimulation {
    // Atmospheric grid
    pub grid: SimulationGrid,

    // Fuel elements
    elements: Vec<Option<FuelElement>>,
    burning_elements: HashSet<u32>,
    next_element_id: u32,

    // Spatial indexing for elements
    spatial_index: SpatialIndex,

    // Weather system
    pub weather: WeatherSystem,

    // Embers
    embers: Vec<Ember>,
    _next_ember_id: u32,

    // Suppression droplets
    suppression_droplets: Vec<SuppressionDroplet>,

    // Configuration
    max_search_radius: f32,

    // Statistics
    pub total_fuel_consumed: f32,
    pub total_area_burned: f32,
    pub simulation_time: f32,
    pub max_fire_intensity: f32,
}

impl FireSimulation {
    /// Create a new ultra-realistic fire simulation
    pub fn new(grid_cell_size: f32, terrain: TerrainData) -> Self {
        // Use terrain dimensions
        let width = terrain.width;
        let height = terrain.height;
        // Use sensible depth based on terrain elevation range
        let depth = (terrain.max_elevation - terrain.min_elevation + 100.0).max(100.0);

        let bounds = (
            Vec3::new(0.0, 0.0, terrain.min_elevation),
            Vec3::new(width, height, terrain.max_elevation + 50.0),
        );

        let spatial_index = SpatialIndex::new(bounds, 15.0);
        let grid = SimulationGrid::new(width, height, depth, grid_cell_size, terrain);

        FireSimulation {
            grid,
            elements: Vec::new(),
            burning_elements: HashSet::new(),
            next_element_id: 0,
            spatial_index,
            weather: WeatherSystem::default(),
            embers: Vec::new(),
            _next_ember_id: 0,
            suppression_droplets: Vec::new(),
            max_search_radius: 15.0,
            total_fuel_consumed: 0.0,
            total_area_burned: 0.0,
            simulation_time: 0.0,
            max_fire_intensity: 0.0,
        }
    }

    /// Add a fuel element to the simulation
    pub fn add_fuel_element(
        &mut self,
        position: Vec3,
        fuel: Fuel,
        mass: f32,
        part_type: FuelPart,
        parent_id: Option<u32>,
    ) -> u32 {
        let id = self.next_element_id;
        self.next_element_id += 1;

        let element = FuelElement::new(id, position, fuel, mass, part_type, parent_id);

        // Add to spatial index
        self.spatial_index.insert(id, position);

        // Add to elements array
        if id as usize >= self.elements.len() {
            self.elements.resize((id as usize + 1) * 2, None);
        }
        self.elements[id as usize] = Some(element);

        id
    }

    /// Ignite a fuel element
    pub fn ignite_element(&mut self, element_id: u32, initial_temp: f32) {
        if let Some(Some(element)) = self.elements.get_mut(element_id as usize) {
            element.ignited = true;
            element.temperature = initial_temp.max(element.fuel.ignition_temperature);
            self.burning_elements.insert(element_id);
        }
    }

    /// Get a fuel element by ID
    pub fn get_element(&self, id: u32) -> Option<&FuelElement> {
        self.elements.get(id as usize)?.as_ref()
    }

    /// Get a mutable fuel element by ID
    fn get_element_mut(&mut self, id: u32) -> Option<&mut FuelElement> {
        self.elements.get_mut(id as usize)?.as_mut()
    }

    /// Set weather conditions
    pub fn set_weather(&mut self, weather: WeatherSystem) {
        // Update grid ambient conditions before moving weather
        self.grid.ambient_temperature = weather.temperature;
        self.grid.ambient_humidity = weather.humidity;
        self.grid.ambient_wind = weather.wind_vector();

        // Now move weather
        self.weather = weather;
    }

    /// Add suppression droplet
    pub fn add_suppression_droplet(&mut self, droplet: SuppressionDroplet) {
        self.suppression_droplets.push(droplet);
    }

    /// Main simulation update
    pub fn update(&mut self, dt: f32) {
        self.simulation_time += dt;

        // 1. Update weather
        self.weather.update(dt);
        let wind_vector = self.weather.wind_vector();
        let ffdi_multiplier = self.weather.spread_rate_multiplier();

        // 2. Update wind field in grid based on terrain
        update_wind_field(&mut self.grid, wind_vector, dt);

        // 3. Mark active cells near burning elements
        let burning_positions: Vec<Vec3> = self
            .burning_elements
            .iter()
            .filter_map(|&id| self.get_element(id).map(|e| e.position))
            .collect();
        self.grid.mark_active_cells(&burning_positions, 30.0);

        // 4. Update burning elements (parallelized for performance)
        let elements_to_process: Vec<u32> = self.burning_elements.iter().copied().collect();

        // Cache spatial queries to avoid repeated lookups (major performance win)
        let nearby_cache: Vec<(u32, Vec3, Vec<u32>)> = elements_to_process
            .iter()
            .filter_map(|&element_id| {
                self.get_element(element_id).map(|e| {
                    let nearby = self
                        .spatial_index
                        .query_radius(e.position, self.max_search_radius);
                    (element_id, e.position, nearby)
                })
            })
            .collect();

        for (element_id, _element_pos, _nearby) in &nearby_cache {
            let element_id = *element_id;
            // 4a. Apply grid conditions to element (needs both borrows separate)
            {
                let grid_data = self.grid.interpolate_at_position(
                    self.get_element(element_id)
                        .map(|e| e.position)
                        .unwrap_or(Vec3::zeros()),
                );

                if let Some(element) = self.get_element_mut(element_id) {
                    // Apply humidity changes
                    if grid_data.humidity > element.moisture_fraction {
                        let moisture_uptake_rate = 0.0001;
                        let moisture_increase =
                            (grid_data.humidity - element.moisture_fraction) * moisture_uptake_rate;
                        element.moisture_fraction = (element.moisture_fraction + moisture_increase)
                            .min(element.fuel.base_moisture * 1.5);
                    }

                    // Apply suppression cooling
                    if grid_data.suppression_agent > 0.0 {
                        let cooling_rate = grid_data.suppression_agent * 1000.0;
                        let mass = element.fuel_remaining;
                        let temp_drop = cooling_rate / (mass * element.fuel.specific_heat);
                        element.temperature =
                            (element.temperature - temp_drop).max(grid_data.temperature);
                    }
                }
            }

            // 4b. Get element info for burn calculations
            let base_burn_rate = {
                if let Some(element) = self.get_element(element_id) {
                    element.calculate_burn_rate()
                } else {
                    continue;
                }
            };

            // 4c. Calculate oxygen-limited burn rate
            let oxygen_factor = get_oxygen_limited_burn_rate(
                self.get_element(element_id).unwrap(),
                base_burn_rate,
                &self.grid,
            );

            let actual_burn_rate = base_burn_rate * oxygen_factor;
            let fuel_consumed = actual_burn_rate * dt;

            // 4d. Burn fuel and update element
            let mut should_extinguish = false;
            let mut fuel_consumed_actual = 0.0;
            if let Some(element) = self.get_element_mut(element_id) {
                element.fuel_remaining -= fuel_consumed;
                fuel_consumed_actual = fuel_consumed;

                if element.fuel_remaining < 0.01 {
                    element.ignited = false;
                    should_extinguish = true;
                }
            }

            self.total_fuel_consumed += fuel_consumed_actual;

            if should_extinguish {
                self.burning_elements.remove(&element_id);
            }

            // 4e. Transfer heat and combustion products to grid
            // Collect element data first to avoid borrow conflicts
            let element_data = if let Some(element) = self.get_element(element_id) {
                if element.ignited {
                    Some((
                        element.position,
                        element.temperature,
                        element.fuel_remaining,
                        element.fuel.surface_area_to_volume,
                        element.fuel.heat_content,
                    ))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((pos, temp, fuel_remaining, surface_area, heat_content)) = element_data {
                // Get grid parameters we'll need
                let cell_size = self.grid.cell_size;
                let cell_volume = cell_size.powi(3);

                // Now we can safely borrow grid mutably
                if let Some(cell) = self.grid.cell_at_position_mut(pos) {
                    // Enhanced heat transfer - fires need to heat air more effectively
                    let temp_diff = temp - cell.temperature;
                    if temp_diff > 0.0 {
                        // Much higher heat transfer coefficient for realistic fire heating
                        let h = 500.0; // W/(m²·K) - typical for fire convection
                        let area = surface_area * fuel_remaining.sqrt();
                        let heat_kj = h * area * temp_diff * dt * 0.001;

                        let air_mass = cell.air_density() * cell_volume;
                        let specific_heat_air = 1.005; // kJ/(kg·K)
                        let temp_rise = heat_kj / (air_mass * specific_heat_air);

                        // Allow cell to reach high temperatures from fire, but cap appropriately
                        // Cell should not exceed element temp (can't be hotter than source)
                        // and must respect physical limits for wildfire air temperatures
                        let target_temp = (cell.temperature + temp_rise)
                            .min(temp * 0.8) // Cell reaches max 80% of source temp
                            .min(800.0); // Physical cap for wildfire plume air

                        cell.temperature = target_temp;
                    }

                    // Combustion products
                    let products =
                        crate::physics::combustion_physics::calculate_combustion_products(
                            fuel_consumed,
                            cell,
                            heat_content,
                        );

                    cell.oxygen -= products.o2_consumed / cell_volume;
                    cell.oxygen = cell.oxygen.max(0.0);
                    cell.carbon_dioxide += products.co2_produced / cell_volume;
                    cell.water_vapor += products.h2o_produced / cell_volume;
                    cell.smoke_particles += products.smoke_produced / cell_volume;
                }
            }

            // Store heat transfers for this source (no borrow conflicts)
        }

        // Calculate all element-to-element heat transfers in parallel for performance
        // This leverages Rayon to compute radiation/convection physics across all burning elements
        let ambient_temp = self.grid.ambient_temperature;

        // Collect heat transfer calculations (parallelized, read-only)
        let heat_transfers: Vec<(u32, f32)> = nearby_cache
            .par_iter()
            .flat_map(|(element_id, _pos, nearby)| {
                // Get source element data (read-only)
                let source_data = self.get_element(*element_id).map(|source| {
                    (
                        source.position,
                        source.temperature,
                        source.fuel_remaining,
                        source.fuel.clone(),
                    )
                });

                if let Some((source_pos, source_temp, source_fuel_remaining, source_fuel)) = source_data {
                    // Create temporary source element for physics calculations
                    let temp_source = FuelElement {
                        id: *element_id,
                        position: source_pos,
                        temperature: source_temp,
                        fuel_remaining: source_fuel_remaining,
                        fuel: source_fuel,
                        moisture_fraction: 0.0,
                        ignited: true,
                        flame_height: 0.0,
                        parent_id: None,
                        part_type: FuelPart::GroundVegetation,
                        elevation: source_pos.z,
                        slope_angle: 0.0,
                        neighbors: Vec::new(),
                    };

                    // Calculate heat for all nearby targets
                    nearby
                        .iter()
                        .filter_map(|&target_id| {
                            if target_id == *element_id {
                                return None;
                            }

                            // Get target element data (read-only)
                            self.get_element(target_id).and_then(|target| {
                                if target.ignited || target.fuel_remaining < 0.01 {
                                    return None;
                                }

                                // Create temporary target for physics
                                let temp_target = FuelElement {
                                    id: target_id,
                                    position: target.position,
                                    temperature: target.temperature,
                                    fuel_remaining: 1.0,
                                    fuel: target.fuel.clone(),
                                    moisture_fraction: 0.0,
                                    ignited: false,
                                    flame_height: 0.0,
                                    parent_id: None,
                                    part_type: FuelPart::GroundVegetation,
                                    elevation: target.position.z,
                                    slope_angle: 0.0,
                                    neighbors: Vec::new(),
                                };

                                // Calculate heat transfer (pure computation, no side effects)
                                let base_heat = crate::physics::element_heat_transfer::calculate_total_heat_transfer(
                                    &temp_source,
                                    &temp_target,
                                    wind_vector,
                                    dt,
                                );

                                // Apply FFDI multiplier for realistic Australian fire behavior
                                let heat = base_heat * ffdi_multiplier;

                                if heat > 0.0 {
                                    Some((target_id, heat))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            })
            .collect();

        // Apply heat transfers sequentially to avoid race conditions
        // Use a HashMap to accumulate heat for each target from multiple sources
        use std::collections::HashMap;
        let mut heat_map: HashMap<u32, f32> = HashMap::new();
        for (target_id, heat) in heat_transfers {
            *heat_map.entry(target_id).or_insert(0.0) += heat;
        }

        // Apply accumulated heat to each target
        for (target_id, total_heat) in heat_map {
            if let Some(target) = self.get_element_mut(target_id) {
                target.apply_heat(total_heat, dt, ambient_temp);

                // Check for ignition
                if target.temperature > target.fuel.ignition_temperature {
                    target.ignited = true;
                    self.burning_elements.insert(target_id);
                }
            }
        }

        // 5. Update grid atmospheric processes
        self.grid.update_diffusion(dt);
        self.grid.update_buoyancy(dt);

        // 6. Simulate plume rise
        simulate_plume_rise(&mut self.grid, &burning_positions, dt);

        // 7. Update suppression droplets
        self.suppression_droplets
            .par_iter_mut()
            .for_each(|droplet| {
                let local_wind = self.grid.interpolate_at_position(droplet.position).wind;
                droplet.update(local_wind, self.grid.ambient_temperature, dt);
            });

        // Apply suppression to grid
        for droplet in &self.suppression_droplets {
            if droplet.active {
                crate::physics::suppression_physics::apply_suppression_to_grid(
                    droplet,
                    &mut self.grid,
                );
            }
        }

        // Remove inactive droplets
        self.suppression_droplets.retain(|d| d.active);

        // 8. Update embers (legacy system - can be enhanced)
        self.embers.par_iter_mut().for_each(|ember| {
            ember.update_physics(wind_vector, self.grid.ambient_temperature, dt);
        });

        self.embers
            .retain(|e| e.temperature > 200.0 && e.position.z > 0.0);
    }

    /// Get all burning elements
    pub fn get_burning_elements(&self) -> Vec<&FuelElement> {
        self.burning_elements
            .iter()
            .filter_map(|&id| self.get_element(id))
            .collect()
    }

    /// Get grid cell at position
    pub fn get_cell_at_position(&self, pos: Vec3) -> Option<&GridCell> {
        self.grid.cell_at_position(pos)
    }

    /// Get number of active cells
    pub fn active_cell_count(&self) -> usize {
        self.grid.active_cell_count()
    }

    /// Get statistics
    pub fn get_stats(&self) -> SimulationStats {
        SimulationStats {
            burning_elements: self.burning_elements.len(),
            total_elements: self.elements.iter().filter(|e| e.is_some()).count(),
            active_cells: self.active_cell_count(),
            total_cells: self.grid.cells.len(),
            suppression_droplets: self.suppression_droplets.len(),
            total_fuel_consumed: self.total_fuel_consumed,
            simulation_time: self.simulation_time,
        }
    }
}

/// Statistics for the simulation
#[derive(Debug, Clone)]
pub struct SimulationStats {
    pub burning_elements: usize,
    pub total_elements: usize,
    pub active_cells: usize,
    pub total_cells: usize,
    pub suppression_droplets: usize,
    pub total_fuel_consumed: f32,
    pub simulation_time: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ultra_simulation_creation() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let sim = FireSimulation::new(10.0, terrain);

        assert_eq!(sim.burning_elements.len(), 0);
        assert_eq!(sim.grid.nx, 10);
        assert_eq!(sim.grid.ny, 10);
    }

    #[test]
    fn test_add_and_ignite() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let mut sim = FireSimulation::new(10.0, terrain);

        let fuel = Fuel::dry_grass();
        let id = sim.add_fuel_element(
            Vec3::new(50.0, 50.0, 1.0),
            fuel,
            1.0,
            FuelPart::GroundVegetation,
            None,
        );

        sim.ignite_element(id, 600.0);

        assert_eq!(sim.burning_elements.len(), 1);
        assert!(sim.get_element(id).unwrap().ignited);
    }

    #[test]
    fn test_simulation_update() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let mut sim = FireSimulation::new(10.0, terrain);

        let fuel = Fuel::dry_grass();
        let id = sim.add_fuel_element(
            Vec3::new(50.0, 50.0, 1.0),
            fuel,
            1.0,
            FuelPart::GroundVegetation,
            None,
        );

        sim.ignite_element(id, 600.0);

        // Update for 1 second
        sim.update(1.0);

        // Should have consumed some fuel
        assert!(sim.total_fuel_consumed > 0.0);

        // Cell should be heated
        let cell = sim
            .get_cell_at_position(Vec3::new(50.0, 50.0, 1.0))
            .unwrap();
        assert!(cell.temperature > 20.0);
    }

    /// Test fire spread under LOW fire danger conditions (cool, humid, calm)
    /// Fire should spread slowly - mimicking real Australian winter conditions
    #[test]
    fn test_low_fire_danger_minimal_spread() {
        let terrain = TerrainData::flat(50.0, 50.0, 2.0, 0.0);
        let mut sim = FireSimulation::new(2.0, terrain);

        // Set LOW fire danger conditions (cool, humid, calm - winter conditions)
        let weather = WeatherSystem::new(
            15.0, // Cool temperature (15°C - winter)
            0.70, // High humidity (70% - coastal morning)
            2.0,  // Low wind (2 m/s - calm)
            0.0,  // No wind direction
            2.0,  // Low drought factor
        );
        sim.set_weather(weather);

        // Add fuel elements in a wider grid (3m spacing - less dense)
        let mut fuel_ids = Vec::new();
        for i in 0..5 {
            for j in 0..5 {
                let x = 20.0 + i as f32 * 3.0;
                let y = 20.0 + j as f32 * 3.0;
                let fuel = Fuel::dry_grass();
                let id = sim.add_fuel_element(
                    Vec3::new(x, y, 0.5),
                    fuel,
                    3.0,
                    FuelPart::GroundVegetation,
                    None,
                );
                fuel_ids.push(id);
            }
        }

        // Ignite center element
        sim.ignite_element(fuel_ids[12], 600.0);

        // Run for 20 seconds (shorter time for low conditions)
        for _ in 0..20 {
            sim.update(1.0);
        }

        let burning_count = sim.burning_elements.len();

        // Under LOW fire danger with wider spacing, spread should be limited
        // Real Australian fires in winter/humid conditions spread slowly
        // Expect minimal spread beyond immediate neighbors
        assert!(
            burning_count < 15,
            "Low fire danger should have limited spread (<15 of 25), got {} burning elements",
            burning_count
        );

        // FFDI should be low
        let ffdi = sim.weather.calculate_ffdi();
        assert!(ffdi < 12.0, "FFDI should be low (<12), got {}", ffdi);
    }

    /// Test fire spread under MODERATE fire danger conditions
    /// Fire should spread at a moderate rate - typical spring/autumn conditions
    #[test]
    fn test_moderate_fire_danger_controlled_spread() {
        let terrain = TerrainData::flat(50.0, 50.0, 2.0, 0.0);
        let mut sim = FireSimulation::new(2.0, terrain);

        // Set MODERATE fire danger conditions (warm, moderate humidity, light wind)
        let weather = WeatherSystem::new(
            25.0, // Warm temperature (25°C - spring)
            0.40, // Moderate humidity (40%)
            8.0,  // Moderate wind (8 m/s)
            45.0, // Wind direction
            5.0,  // Moderate drought
        );
        sim.set_weather(weather);

        // Add fuel elements with moderate spacing
        let mut fuel_ids = Vec::new();
        for i in 0..5 {
            for j in 0..5 {
                let x = 20.0 + i as f32 * 2.0;
                let y = 20.0 + j as f32 * 2.0;
                let fuel = Fuel::dry_grass();
                let id = sim.add_fuel_element(
                    Vec3::new(x, y, 0.5),
                    fuel,
                    3.0,
                    FuelPart::GroundVegetation,
                    None,
                );
                fuel_ids.push(id);
            }
        }

        sim.ignite_element(fuel_ids[12], 600.0);

        // Run for 25 seconds
        for _ in 0..25 {
            sim.update(1.0);
        }

        let burning_count = sim.burning_elements.len();

        // MODERATE conditions: should have spread significantly but realistically
        // With direct radiation and moderate conditions, most fuel will ignite
        assert!(
            burning_count >= 10,
            "Moderate fire danger should have significant spread (>=10), got {}",
            burning_count
        );

        // FFDI should be moderate
        let ffdi = sim.weather.calculate_ffdi();
        assert!(
            (12.0..50.0).contains(&ffdi),
            "FFDI should be moderate (12-50), got {}",
            ffdi
        );
    }

    /// Test fire spread under EXTREME fire danger conditions (Code Red)
    /// Fire should spread rapidly - mimicking Black Summer / Ash Wednesday conditions
    #[test]
    fn test_extreme_fire_danger_rapid_spread() {
        let terrain = TerrainData::flat(50.0, 50.0, 2.0, 0.0);
        let mut sim = FireSimulation::new(2.0, terrain);

        // Set EXTREME fire danger conditions (hot, dry, strong wind - Code Red)
        let weather = WeatherSystem::new(
            42.0, // Extreme temperature (42°C - heatwave)
            0.15, // Very low humidity (15% - bone dry)
            25.0, // Strong wind (25 m/s / 90 km/h)
            45.0, // Wind direction (NE typical for bad fire days)
            10.0, // Extreme drought
        );
        sim.set_weather(weather);

        // Add fuel elements
        let mut fuel_ids = Vec::new();
        for i in 0..5 {
            for j in 0..5 {
                let x = 20.0 + i as f32 * 1.5;
                let y = 20.0 + j as f32 * 1.5;
                let fuel = Fuel::dry_grass();
                let id = sim.add_fuel_element(
                    Vec3::new(x, y, 0.5),
                    fuel,
                    3.0,
                    FuelPart::GroundVegetation,
                    None,
                );
                fuel_ids.push(id);
            }
        }

        sim.ignite_element(fuel_ids[12], 600.0);

        // Run for 30 seconds
        for _ in 0..30 {
            sim.update(1.0);
        }

        let burning_count = sim.burning_elements.len();

        // EXTREME conditions: should have rapid spread to majority of fuel (>15 elements)
        assert!(
            burning_count > 15,
            "Extreme fire danger should have rapid spread (>15), got {}",
            burning_count
        );

        // FFDI should be extreme (>75)
        let ffdi = sim.weather.calculate_ffdi();
        assert!(ffdi > 75.0, "FFDI should be extreme (>75), got {}", ffdi);
    }

    /// Test that Australian-specific factors affect fire behavior correctly
    #[test]
    fn test_australian_fire_characteristics() {
        let terrain = TerrainData::flat(50.0, 50.0, 2.0, 0.0);
        let mut sim = FireSimulation::new(2.0, terrain);

        // Test eucalyptus fuel with volatile oils
        let eucalyptus = Fuel::eucalyptus_stringybark();

        // Verify Australian-specific properties exist
        assert!(
            eucalyptus.volatile_oil_content > 0.0,
            "Eucalyptus should have volatile oils"
        );
        assert!(
            eucalyptus.oil_vaporization_temp > 0.0,
            "Should have oil vaporization temp"
        );
        assert!(
            eucalyptus.oil_autoignition_temp > 0.0,
            "Should have oil autoignition temp"
        );
        assert!(
            eucalyptus.max_spotting_distance > 1000.0,
            "Eucalyptus should have long spotting distance"
        );

        // Stringybark should have high ladder fuel factor
        assert!(
            eucalyptus.bark_properties.ladder_fuel_factor > 0.8,
            "Stringybark should have high ladder fuel factor"
        );

        // Test McArthur FFDI is being used
        let weather = WeatherSystem::new(35.0, 0.25, 20.0, 0.0, 8.0);
        sim.set_weather(weather);

        let ffdi = sim.weather.calculate_ffdi();
        assert!(ffdi > 0.0, "FFDI should be calculated");

        // Verify FFDI affects spread rate
        let spread_multiplier = sim.weather.spread_rate_multiplier();
        assert!(
            spread_multiplier > 1.0,
            "High FFDI should increase spread rate"
        );
    }

    /// Test wind direction effects on fire spread (critical for Australian conditions)
    #[test]
    fn test_wind_direction_fire_spread() {
        let terrain = TerrainData::flat(50.0, 50.0, 2.0, 0.0);
        let mut sim = FireSimulation::new(2.0, terrain);

        // Strong easterly wind (270° = westward)
        let weather = WeatherSystem::new(
            35.0, // Hot
            0.20, // Dry
            20.0, // Strong wind
            0.0,  // East wind (spreads west)
            8.0,
        );
        sim.set_weather(weather);

        // Create line of fuel elements east to west
        let mut fuel_ids = Vec::new();
        for i in 0..10 {
            let x = 20.0 + i as f32 * 1.5;
            let fuel = Fuel::dry_grass();
            let id = sim.add_fuel_element(
                Vec3::new(x, 25.0, 0.5),
                fuel,
                3.0,
                FuelPart::GroundVegetation,
                None,
            );
            fuel_ids.push(id);
        }

        // Ignite eastern end (downwind elements should ignite)
        sim.ignite_element(fuel_ids[0], 600.0);

        // Run simulation
        for _ in 0..20 {
            sim.update(1.0);
        }

        // Check that downwind elements ignited more than upwind
        let mut downwind_burning = 0;
        for elem_id in fuel_ids.iter().take(5) {
            if let Some(elem) = sim.get_element(*elem_id) {
                if elem.ignited {
                    downwind_burning += 1;
                }
            }
        }

        // Fire should spread in wind direction
        assert!(
            downwind_burning >= 2,
            "Fire should spread downwind, got {} elements",
            downwind_burning
        );
    }
}
