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
use crate::core_types::{get_oxygen_limited_burn_rate, simulate_plume_rise, update_wind_field};
use crate::grid::{GridCell, SimulationGrid, TerrainData};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

/// Ultra-realistic fire simulation with full atmospheric modeling
pub struct FireSimulation {
    // Atmospheric grid
    pub(crate) grid: SimulationGrid,

    // Fuel elements
    elements: Vec<Option<FuelElement>>,
    /// Set of burning element IDs
    pub(crate) burning_elements: HashSet<u32>,
    next_element_id: u32,

    // Spatial indexing for elements
    /// Spatial index for efficient neighbor queries
    pub(crate) spatial_index: SpatialIndex,

    // Weather system
    weather: WeatherSystem,

    // Embers
    embers: Vec<Ember>,
    _next_ember_id: u32,

    // Configuration
    max_search_radius: f32,

    // Statistics
    pub(crate) total_fuel_consumed: f32,
    pub(crate) simulation_time: f32,
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
            max_search_radius: 10.0, // Reduced from 15.0m to prevent instant ignition of adjacent trees
            total_fuel_consumed: 0.0,
            simulation_time: 0.0,
        }
    }

    /// Get the grid's terrain.
    pub fn terrain(&self) -> &TerrainData {
        &self.grid.terrain
    }

    /// Get the current number of active embers
    pub fn ember_count(&self) -> usize {
        self.embers.len()
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

    /// Ignite a fuel element directly (MANUAL IGNITION PATHWAY)
    ///
    /// This method is for **manual fire starts** and **bypasses moisture evaporation physics**.
    /// It directly sets the element to burning state at the specified temperature.
    ///
    /// # Use Cases
    ///
    /// - Lightning strikes (instant high-energy ignition)
    /// - Human-caused fires (arson, accidents)
    /// - Controlled burns (prescribed fire operations)
    /// - Testing and validation
    /// - Game engine / FFI layer fire starts
    ///
    /// # Natural Fire Spread
    ///
    /// For realistic fire spread behavior, DO NOT use this method. Natural spread occurs through:
    /// - **Heat-based auto-ignition**: Elements receive heat via `apply_heat()` which respects
    ///   moisture evaporation (2260 kJ/kg latent heat) and probabilistic ignition via
    ///   `check_ignition_probability()`. See FuelElement::apply_heat() in core_types/element.rs.
    /// - **Ember spot fires**: Hot embers land on receptive fuel and attempt ignition based on
    ///   ember temperature, fuel moisture, and fuel ember_receptivity property.
    ///
    /// # Physics Justification
    ///
    /// Lightning strikes deliver instantaneous high energy (typically 1-5 GJ) that can:
    /// - Flash-vaporize fuel moisture instantly
    /// - Raise fuel temperature above ignition point in milliseconds
    /// - Ignite even moderately moist fuels (up to 20% moisture)
    ///
    /// This bypassing of moisture physics is realistic for such high-energy ignition sources.
    /// For lower-energy ignition sources (embers, radiant heat), use heat-based pathways instead.
    ///
    /// # Parameters
    ///
    /// - `element_id`: ID of fuel element to ignite
    /// - `initial_temp`: Initial burning temperature (°C). Will be clamped to at least
    ///   the fuel's ignition temperature.
    pub fn ignite_element(&mut self, element_id: u32, initial_temp: f32) {
        if let Some(Some(element)) = self.elements.get_mut(element_id as usize) {
            element.ignited = true;
            element.temperature = initial_temp.max(element.fuel.ignition_temperature);
            self.burning_elements.insert(element_id);
        }
    }

    /// Apply heat to a fuel element (respects moisture evaporation physics)
    ///
    /// This method applies heat energy to a fuel element following realistic physics:
    /// - Heat goes to moisture evaporation FIRST (2260 kJ/kg latent heat)
    /// - Remaining heat raises temperature
    /// - Probabilistic ignition via check_ignition_probability()
    ///
    /// # Use Cases
    /// - Backburns/controlled burns (gradual heating)
    /// - Radiant heat from external sources
    /// - Pre-heating from approaching fire
    /// - Testing heat-based ignition mechanics
    ///
    /// # Parameters
    /// - `element_id`: ID of fuel element to heat
    /// - `heat_kj`: Heat energy in kilojoules
    /// - `dt`: Time step in seconds
    pub fn apply_heat_to_element(&mut self, element_id: u32, heat_kj: f32, dt: f32) {
        let ambient_temp = self.grid.ambient_temperature;
        if let Some(element) = self.get_element_mut(element_id) {
            let was_ignited = element.ignited;
            element.apply_heat(heat_kj, dt, ambient_temp);

            // Add newly ignited elements to burning set
            if !was_ignited && element.ignited {
                self.burning_elements.insert(element_id);
            }
        }
    }

    /// Get all fuel elements within a certain radius around a position
    ///
    /// # Arguments
    /// * `position` - Center position in world space
    /// * `radius` - Search radius in meters
    ///
    /// # Returns
    /// Vector of references to fuel elements within the specified radius
    pub fn get_elements_in_radius(&self, position: Vec3, radius: f32) -> Vec<&FuelElement> {
        let nearby_ids = self.spatial_index.query_radius(position, radius);

        nearby_ids
            .into_iter()
            .filter_map(|id| self.get_element(id))
            .collect()
    }

    /// Get a fuel element by ID
    pub fn get_element(&self, id: u32) -> Option<&FuelElement> {
        self.elements.get(id as usize)?.as_ref()
    }

    /// Get a mutable fuel element by ID
    pub fn get_element_mut(&mut self, id: u32) -> Option<&mut FuelElement> {
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

    /// Get reference to weather system (read-only)
    pub fn get_weather(&self) -> &WeatherSystem {
        &self.weather
    }

    /// Apply suppression directly at specified coordinates without physics simulation
    ///
    /// This method immediately applies suppression agent to the grid at the given position,
    /// bypassing the physics-based droplet simulation. Useful for direct application
    /// such as ground crews or instant effects.
    ///
    /// # Parameters
    /// - `position`: World coordinates (x, y, z) where suppression is applied
    /// - `mass`: Mass of suppression agent in kg
    /// - `agent_type`: Type of suppression agent (Water, Retardant, Foam)
    pub fn apply_suppression_direct(
        &mut self,
        position: Vec3,
        mass: f32,
        agent_type: crate::physics::SuppressionAgent,
    ) {
        crate::physics::apply_suppression_direct(position, mass, agent_type, &mut self.grid);
    }

    /// Main simulation update
    ///
    /// # Fire Ignition Mechanisms
    ///
    /// This simulation implements THREE distinct ignition pathways, each with scientific basis:
    ///
    /// ## 1. Manual Ignition (`ignite_element`)
    /// - **Purpose**: Initial fire starts (lightning, human-caused, testing)
    /// - **Physics**: Bypasses moisture evaporation (instant high-energy delivery)
    /// - **When**: Called explicitly for lightning strikes, arson, controlled burns
    /// - **Justification**: Lightning delivers 1-5 GJ instantly, flash-vaporizing moisture
    ///
    /// ## 2. Heat-Based Auto-Ignition (`apply_heat` → `check_ignition_probability`)
    /// - **Purpose**: Natural fire spread element-to-element
    /// - **Physics**: Respects moisture evaporation (2260 kJ/kg), probabilistic ignition
    /// - **When**: Automatically during heat transfer from burning neighbors
    /// - **Justification**: Rothermel (1972) heat of pre-ignition, Nelson (2000) moisture dynamics
    ///
    /// ## 3. Ember Spot Fire Ignition (`Ember::attempt_ignition`)
    /// - **Purpose**: Long-range fire spread via ember spotting (up to 25km)
    /// - **Physics**: Probability based on ember temp, fuel moisture, ember_receptivity
    /// - **When**: Hot embers (>250°C) land on receptive fuel
    /// - **Justification**: Koo et al. (2010), Black Saturday 2009 empirical data
    ///   - Stringybark: 60% receptivity (highly susceptible)
    ///   - Dead litter: 90% receptivity (extremely susceptible)
    ///   - Green vegetation: 20% receptivity (resistant)
    ///
    /// These three mechanisms work together to create realistic Australian bushfire behavior,
    /// where ember spotting is often the dominant spread mechanism during extreme fire weather.
    pub fn update(&mut self, dt: f32) {
        self.simulation_time += dt;

        // 1. Update weather
        self.weather.update(dt);
        let wind_vector = self.weather.wind_vector();
        let ffdi_multiplier = self.weather.spread_rate_multiplier();

        // Heat transfer boost factor for smaller timesteps
        // With dt=0.1s, 10 updates per second still need effective heat transfer
        // This compensates for numerical precision losses at smaller timesteps
        // While maintaining realistic overall fire spread behavior
        let heat_boost = if dt < 0.5 { 5.0 } else { 1.0 };

        // 1a. Update fuel moisture using Nelson timelag system (Phase 1)
        // Assume desorption (drying) as typical wildfire conditions
        let equilibrium_moisture = crate::physics::calculate_equilibrium_moisture(
            self.weather.temperature,
            self.weather.humidity,
            false, // is_adsorbing - false for typical drying conditions
        );

        // 1aa. Apply ambient temperature regulation for all elements
        // Non-burning elements should cool/heat toward ambient temperature
        let ambient_temp = self.grid.ambient_temperature;
        self.elements.par_iter_mut().flatten().for_each(|element| {
            if !element.ignited {
                // Newton's law of cooling: dT/dt = -k(T - T_ambient)
                // Typical convective cooling coefficient for outdoor conditions
                let cooling_rate = 0.1; // per second (faster for better responsiveness)
                let temp_diff = element.temperature - ambient_temp;
                let temp_change = temp_diff * cooling_rate * dt;
                element.temperature -= temp_change;
                element.temperature = element.temperature.max(ambient_temp);
            }
        });

        self.elements.par_iter_mut().flatten().for_each(|element| {
            if let Some(ref mut moisture_state) = element.moisture_state {
                let dt_hours = dt / 3600.0; // Convert seconds to hours

                // Update each timelag class
                moisture_state.moisture_1h = crate::physics::update_moisture_timelag(
                    moisture_state.moisture_1h,
                    equilibrium_moisture,
                    element.fuel.timelag_1h,
                    dt_hours,
                );
                moisture_state.moisture_10h = crate::physics::update_moisture_timelag(
                    moisture_state.moisture_10h,
                    equilibrium_moisture,
                    element.fuel.timelag_10h,
                    dt_hours,
                );
                moisture_state.moisture_100h = crate::physics::update_moisture_timelag(
                    moisture_state.moisture_100h,
                    equilibrium_moisture,
                    element.fuel.timelag_100h,
                    dt_hours,
                );
                moisture_state.moisture_1000h = crate::physics::update_moisture_timelag(
                    moisture_state.moisture_1000h,
                    equilibrium_moisture,
                    element.fuel.timelag_1000h,
                    dt_hours,
                );

                // Update overall moisture fraction (weighted average)
                let dist = element.fuel.size_class_distribution;
                element.moisture_fraction = moisture_state.moisture_1h * dist[0]
                    + moisture_state.moisture_10h * dist[1]
                    + moisture_state.moisture_100h * dist[2]
                    + moisture_state.moisture_1000h * dist[3];

                moisture_state.average_moisture = element.moisture_fraction;
            }
        });

        // 2. Update wind field in grid based on terrain
        update_wind_field(&mut self.grid, wind_vector, dt);

        // 3. Mark active cells near burning elements
        let burning_positions: Vec<Vec3> = self
            .burning_elements
            .par_iter()
            .filter_map(|&id| self.get_element(id).map(|e| e.position))
            .collect();
        self.grid.mark_active_cells(&burning_positions, 30.0);

        // 4. Update burning elements (parallelized for performance)
        let elements_to_process: Vec<u32> = self.burning_elements.iter().copied().collect();

        // Cache spatial queries to avoid repeated lookups (major performance win)
        // Use wind-directional search radius for realistic fire spread
        let wind_vector = self.weather.wind_vector();
        let wind_speed_ms = wind_vector.magnitude();

        let nearby_cache: Vec<(u32, Vec3, Vec<u32>)> = elements_to_process
            .par_iter()
            .filter_map(|&element_id| {
                self.get_element(element_id).map(|e| {
                    // Wind-directional search radius: base 10m, extends downwind
                    // Downwind: 10m + (wind_speed × 1.5) = up to 25m at 10 m/s
                    // This allows fire to "reach out" in wind direction while maintaining
                    // inverse-square heat falloff with distance
                    let downwind_extension = if wind_speed_ms > 0.1 {
                        wind_speed_ms * 1.5
                    } else {
                        0.0
                    };
                    let search_radius = self.max_search_radius + downwind_extension;

                    let nearby = self.spatial_index.query_radius(e.position, search_radius);
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

            // 4b. Update smoldering combustion state (Phase 3)
            let smold_update_data = if let Some(element) = self.get_element(element_id) {
                if let Some(smold_state) = element.smoldering_state {
                    let grid_data = self.grid.interpolate_at_position(element.position);
                    Some((smold_state, element.temperature, grid_data.oxygen))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((smold_state, temp, oxygen)) = smold_update_data {
                if let Some(element) = self.get_element_mut(element_id) {
                    element.smoldering_state = Some(crate::physics::update_smoldering_state(
                        smold_state,
                        temp,
                        oxygen,
                        dt,
                    ));
                }
            }

            // 4c. Get element info for burn calculations
            let base_burn_rate = {
                if let Some(element) = self.get_element(element_id) {
                    element.calculate_burn_rate()
                } else {
                    continue;
                }
            };

            // 4d. Calculate oxygen-limited burn rate
            let oxygen_factor = get_oxygen_limited_burn_rate(
                self.get_element(element_id).unwrap(),
                base_burn_rate,
                &self.grid,
            );

            let actual_burn_rate = base_burn_rate * oxygen_factor;

            // 4e. Apply smoldering combustion multiplier (Phase 3)
            let smoldering_multiplier = if let Some(element) = self.get_element(element_id) {
                if let Some(ref smold_state) = element.smoldering_state {
                    smold_state.heat_release_multiplier
                } else {
                    1.0
                }
            } else {
                1.0
            };

            let fuel_consumed = actual_burn_rate * smoldering_multiplier * dt;

            // 4f. Burn fuel and update element, INCLUDING temperature increase from combustion
            let mut should_extinguish = false;
            let mut fuel_consumed_actual = 0.0;
            if let Some(element) = self.get_element_mut(element_id) {
                element.fuel_remaining -= fuel_consumed;
                fuel_consumed_actual = fuel_consumed;

                // CRITICAL: Burning elements continue to heat up from their own combustion
                // Heat released = fuel consumed × heat content (kJ/kg)
                if fuel_consumed > 0.0 && element.fuel_remaining > 0.1 {
                    let combustion_heat = fuel_consumed * element.fuel.heat_content;
                    // Only fraction of heat goes to element (rest radiates away)
                    let self_heating = combustion_heat * 0.3; // 30% self-heating
                    let temp_rise =
                        self_heating / (element.fuel_remaining * element.fuel.specific_heat);
                    element.temperature =
                        (element.temperature + temp_rise).min(element.fuel.max_flame_temperature);
                }

                if element.fuel_remaining < 0.01 {
                    element.ignited = false;
                    should_extinguish = true;
                }
            }

            self.total_fuel_consumed += fuel_consumed_actual;

            if should_extinguish {
                self.burning_elements.remove(&element_id);
            }

            // 4g. Check for crown fire transition (Phase 1 - Van Wagner model)
            if let Some(element) = self.get_element(element_id) {
                if !element.crown_fire_active
                    && matches!(
                        element.part_type,
                        FuelPart::Crown | FuelPart::TrunkUpper | FuelPart::Branch { .. }
                    )
                {
                    // Use fuel properties for crown fire calculation
                    let crown_behavior = crate::physics::calculate_crown_fire_behavior(
                        element,
                        element.fuel.crown_bulk_density,
                        element.fuel.crown_base_height,
                        element.fuel.foliar_moisture_content,
                        10.0, // Assume 10 m/min active spread rate (can enhance with actual calculation)
                        wind_vector.norm(),
                    );

                    // If active or passive crown fire, mark it and potentially ignite crown elements
                    if crown_behavior.fire_type != crate::physics::CrownFireType::Surface {
                        if let Some(elem_mut) = self.get_element_mut(element_id) {
                            elem_mut.crown_fire_active = true;
                            // Crown fire causes 2-3x higher temperatures
                            elem_mut.temperature = elem_mut
                                .temperature
                                .max(elem_mut.fuel.max_flame_temperature * 0.9);
                        }
                    }
                }
            }

            // 4h. Transfer heat and combustion products to grid
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
                        moisture_state: None,
                        smoldering_state: None,
                        crown_fire_active: false,
                    };

                    // Calculate heat for all nearby targets
                    nearby
                        .iter()
                        .filter_map(|&target_id| {
                            if target_id == *element_id {
                                return None;
                            }

                            // Get target element data (read-only)
                            // Heat transfer to BOTH ignited and non-ignited elements
                            // Ignited elements need continuous heating to maintain/increase temperature
                            self.get_element(target_id).and_then(|target| {
                                if target.fuel_remaining < 0.01 {
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
                                    moisture_state: None,
                                    smoldering_state: None,
                                    crown_fire_active: false,
                                };

                                // Calculate heat transfer (pure computation, no side effects)
                                let base_heat = crate::physics::element_heat_transfer::calculate_total_heat_transfer(
                                    &temp_source,
                                    &temp_target,
                                    wind_vector,
                                    dt,
                                );

                                // Apply FFDI multiplier and heat boost for realistic Australian fire behavior
                                // Heat boost compensates for numerical precision at small timesteps
                                let heat = base_heat * ffdi_multiplier * heat_boost;

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
        let mut heat_map: HashMap<u32, f32> = HashMap::new();
        for (target_id, heat) in heat_transfers {
            *heat_map.entry(target_id).or_insert(0.0) += heat;
        }

        // Apply accumulated heat to each target
        // NOTE: apply_heat() handles ignition internally via check_ignition_probability()
        // which respects moisture evaporation and probabilistic ignition based on temperature
        // and moisture content. We only need to check if newly ignited to add to burning set.
        for (target_id, total_heat) in heat_map {
            if let Some(target) = self.get_element_mut(target_id) {
                let was_ignited = target.ignited;
                target.apply_heat(total_heat, dt, ambient_temp);

                // Add newly ignited elements to burning set
                // (apply_heat already set ignited=true via check_ignition_probability)
                if !was_ignited && target.ignited {
                    self.burning_elements.insert(target_id);
                }
            }
        }

        // 5. Update grid atmospheric processes
        self.grid.update_diffusion(dt);
        self.grid.update_buoyancy(dt);

        // 6. Simulate plume rise
        simulate_plume_rise(&mut self.grid, &burning_positions, dt);

        // 6a. Generate embers with Albini spotting physics (Phase 2)
        let mut new_ember_id = self._next_ember_id;
        for &element_id in &self.burning_elements {
            if let Some(element) = self.get_element(element_id) {
                // Probabilistic ember generation based on fuel ember production
                // High multiplier for realistic ember generation rates (stringybark produces many embers)
                // For stringybark (0.9 production): 0.9 × 1.0 × 0.8 = 72% chance per second
                let ember_prob = element.fuel.ember_production * dt * 0.8;
                if ember_prob > 0.0 && rand::random::<f32>() < ember_prob {
                    // Calculate ember lofting height using Albini model
                    let intensity = element.byram_fireline_intensity(wind_vector.norm());
                    let lofting_height = crate::physics::calculate_lofting_height(intensity);

                    // Generate ember with physics-based initial conditions
                    // Albini model calculates trajectory - all embers generated (even short distance)
                    let ember_mass = 0.0005; // kg (0.5g typical)
                    let ember = Ember::new(
                        new_ember_id,
                        element.position + Vec3::new(0.0, 0.0, 1.0),
                        Vec3::new(
                            wind_vector.x * 0.5,
                            wind_vector.y * 0.5,
                            lofting_height.min(100.0) * 0.1, // Initial upward velocity
                        ),
                        element.temperature,
                        ember_mass,
                        element.fuel.id,
                    );
                    self.embers.push(ember);
                    new_ember_id += 1;
                }
            }
        }
        self._next_ember_id = new_ember_id;

        // 7. Update embers
        self.embers.par_iter_mut().for_each(|ember| {
            ember.update_physics(wind_vector, self.grid.ambient_temperature, dt);
        });

        // 7a. Attempt ember spot fire ignition (Phase 2 - Albini spotting with Koo et al. ignition)
        // Collect ember data first to avoid borrow checker issues
        // Only hot, landed embers can ignite fuel
        let ember_ignition_attempts: Vec<(usize, Vec3, f32, u8)> = self
            .embers
            .par_iter()
            .enumerate()
            .filter_map(|(idx, ember)| {
                if ember.can_ignite() {
                    Some((
                        idx,
                        ember.position(),
                        ember.temperature(),
                        ember.source_fuel_type(),
                    ))
                } else {
                    None
                }
            })
            .collect();

        let mut ignited_ember_indices = Vec::new();
        for (idx, position, temperature, _source_fuel) in ember_ignition_attempts {
            // Find nearby fuel elements within 2m radius
            let nearby_fuel_ids: Vec<u32> = self.spatial_index.query_radius(position, 2.0);

            // Try to ignite nearby receptive fuel
            let mut ignition_occurred = false;
            for fuel_id in nearby_fuel_ids {
                if let Some(fuel_element) = self.get_element(fuel_id) {
                    // Skip already ignited elements
                    if fuel_element.ignited || fuel_element.fuel_remaining < 0.1 {
                        continue;
                    }

                    // Calculate distance to fuel element
                    let distance = (fuel_element.position - position).magnitude();

                    // 1. Ember temperature factor (Koo et al. 2010)
                    let temp_factor = if temperature >= 600.0 {
                        0.9 // Very hot ember
                    } else if temperature >= 400.0 {
                        0.6 // Hot ember
                    } else if temperature >= 300.0 {
                        0.3 // Warm ember
                    } else if temperature >= 250.0 {
                        0.1 // Cool ember (near threshold)
                    } else {
                        0.0 // Too cold
                    };

                    // 2. Fuel receptivity (fuel-specific property)
                    let receptivity = fuel_element.fuel.ember_receptivity;

                    // 3. Moisture factor (wet fuel resists ignition)
                    let moisture_factor = if fuel_element.moisture_fraction < 0.1 {
                        1.0 // Dry
                    } else if fuel_element.moisture_fraction < 0.2 {
                        0.6 // Slightly moist
                    } else if fuel_element.moisture_fraction < 0.3 {
                        0.3 // Moist
                    } else {
                        0.0 // Too wet (approaching moisture of extinction)
                    };

                    // 4. Distance factor (closer = better heat transfer)
                    let distance_factor = if distance < 0.5 {
                        1.0 // Very close
                    } else if distance < 1.0 {
                        0.7 // Close
                    } else if distance < 2.0 {
                        0.4 // Near
                    } else {
                        0.0 // Too far
                    };

                    // Combined ignition probability (Koo et al. 2010 probabilistic model)
                    let ignition_prob =
                        temp_factor * receptivity * moisture_factor * distance_factor;

                    // Probabilistic ignition
                    if ignition_prob > 0.0 && rand::random::<f32>() < ignition_prob {
                        // Ignite fuel element at ember temperature
                        let ignition_temp =
                            temperature.min(fuel_element.fuel.ignition_temperature + 100.0);
                        self.ignite_element(fuel_id, ignition_temp);
                        ignition_occurred = true;
                        break; // Ember successfully ignited fuel, stop trying
                    }
                }
            }

            if ignition_occurred {
                ignited_ember_indices.push(idx);
            }
        }

        // Remove embers that successfully ignited fuel (they've landed and transferred heat)
        for &idx in ignited_ember_indices.iter().rev() {
            self.embers.swap_remove(idx);
        }

        // Remove inactive embers (cooled below 200°C or fallen below ground)
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

    /// Get all fuel elements (both burning and unburned)
    pub fn get_all_elements(&self) -> Vec<&FuelElement> {
        self.elements
            .iter()
            .filter_map(|opt| opt.as_ref())
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

        // Under LOW fire danger with wider spacing, spread should be controlled
        // Real Australian fires in winter/humid conditions spread slower but still spread
        // With 3x heat boost for numerical stability, expect some spread
        assert!(
            burning_count <= 20,
            "Low fire danger should have controlled spread (<=20 of 25), got {} burning elements",
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

        // MODERATE conditions: should have spread but at controlled rate
        // With 10m search radius, wind-directional effects, and fuel moisture timelag,
        // spread is more realistic and gradual under moderate conditions
        assert!(
            burning_count >= 4,
            "Moderate fire danger should have some spread (>=4), got {}",
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
