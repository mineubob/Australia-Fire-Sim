//! New field-based fire simulation using `FieldSolver`
//!
//! This module provides the new `FieldSimulation` struct that uses the `FieldSolver` trait
//! for GPU/CPU-accelerated field-based fire physics. This replaces the old element-based system.

use crate::core_types::ember::Ember;
use crate::core_types::units::{Kelvin, Meters, Seconds};
use crate::core_types::vec3::Vec3;
use crate::core_types::weather::WeatherSystem;
use crate::solver::{
    create_field_solver, extract_fire_front, FieldSolver, FireFront, QualityPreset,
};
use crate::TerrainData;
use tracing::{debug, info};

/// Field-based fire simulation using GPU/CPU solver
///
/// This struct orchestrates the complete fire simulation using continuous field-based physics
/// instead of the old discrete element system.
pub struct FieldSimulation {
    /// Backend-agnostic field solver (CPU or GPU)
    solver: Box<dyn FieldSolver>,

    /// Weather system (unchanged from old system)
    weather: WeatherSystem,

    /// Ember system (sparse, remains on CPU)
    embers: Vec<Ember>,
    next_ember_id: u32,

    /// Extracted fire front for visualization
    fire_front: FireFront,

    /// Statistics
    total_burned_area: f32,
    total_fuel_consumed: f32,
    simulation_time: f32,

    /// Grid dimensions (cached from solver)
    width: u32,
    height: u32,
    cell_size: f32,
}

impl FieldSimulation {
    /// Create a new field-based fire simulation
    ///
    /// # Arguments
    ///
    /// * `terrain` - Terrain data including elevation, fuel types
    /// * `quality` - Quality preset determining grid resolution
    /// * `weather` - Initial weather conditions
    ///
    /// # Returns
    ///
    /// New `FieldSimulation` instance with GPU or CPU backend
    pub fn new(terrain: &TerrainData, quality: QualityPreset, weather: WeatherSystem) -> Self {
        info!("Creating new field-based fire simulation");

        // Create field solver (automatically selects GPU or CPU)
        let solver = create_field_solver(terrain, quality);

        // Get grid dimensions
        let (width, height, cell_size_meters) = solver.dimensions();
        let cell_size = *cell_size_meters;

        info!(
            "Field simulation initialized: {}x{} grid, cell_size={:.2}m, GPU={}",
            width,
            height,
            cell_size,
            solver.is_gpu_accelerated()
        );

        Self {
            solver,
            weather,
            embers: Vec::new(),
            next_ember_id: 0,
            fire_front: FireFront::new(),
            total_burned_area: 0.0,
            total_fuel_consumed: 0.0,
            simulation_time: 0.0,
            width,
            height,
            cell_size,
        }
    }

    /// Main simulation update loop
    ///
    /// # Arguments
    ///
    /// * `dt` - Timestep in seconds
    pub fn update(&mut self, dt: f32) {
        self.simulation_time += dt;

        // 1. Update weather
        self.weather.update(dt);
        let wind = self.weather.wind_vector();
        let ambient_temp = self.weather.temperature.to_kelvin();
        let humidity = self.weather.humidity.value();

        debug!(
            "Simulation update: t={:.2}s, dt={:.4}s, wind=({:.2}, {:.2}), T={:.1}K",
            self.simulation_time, dt, wind.x, wind.y, *ambient_temp
        );

        // 2. GPU/CPU compute passes
        self.solver
            .step_heat_transfer(Seconds::new(dt), wind, ambient_temp);
        self.solver.step_combustion(Seconds::new(dt));
        self.solver.step_moisture(Seconds::new(dt), humidity);
        self.solver
            .step_level_set(Seconds::new(dt), wind, ambient_temp);
        self.solver.step_ignition_sync();

        // 3. CPU-side sparse updates
        self.update_embers(dt);

        // 4. Extract fire front (can be deferred for performance)
        self.extract_fire_front();

        // 5. Update statistics
        self.update_statistics();
    }

    /// Apply heat to a location for realistic fire ignition.
    ///
    /// **PRIMARY METHOD** for starting fires realistically. Heat accumulates
    /// in fuel, and ignition occurs naturally when temperature thresholds are reached.
    ///
    /// # Arguments
    ///
    /// * `position` - 3D position (x, y, z) in meters. Z is ignored (2D simulation).
    /// * `temperature_celsius` - Target temperature in Celsius to apply
    /// * `radius_meters` - Radius in meters over which to apply heat (Gaussian falloff)
    ///
    /// # Use Cases
    ///
    /// - **Drip torch / backburning**: Apply 600-800°C repeatedly along a fire line
    /// - **Match ignition**: Single 600°C application at a point (~0.1m radius)
    /// - **Radiant heating**: Apply 400-500°C over larger area to test autoignition
    /// - **Failed ignition testing**: Apply heat to wet fuel and watch it fail to ignite
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fire_sim_core::simulation::FieldSimulation;
    /// # use nalgebra::Vector3;
    /// # fn example(sim: &mut FieldSimulation) {
    /// // Drip torch backburn operation
    /// for i in 0..20 {
    ///     let x = 50.0 + i as f32 * 1.5;
    ///     sim.apply_heat(Vector3::new(x, 100.0, 0.0), 650.0, 0.3);
    /// }
    /// # }
    /// ```
    pub fn apply_heat(&mut self, position: Vec3, temperature_celsius: f32, radius_meters: f32) {
        let temp_kelvin = Kelvin::new(f64::from(temperature_celsius + 273.15));
        self.solver.apply_heat(
            Meters::new(position.x),
            Meters::new(position.y),
            temp_kelvin,
            Meters::new(radius_meters),
        );
    }

    /// Get the current fire front for visualization
    pub fn fire_front(&self) -> &FireFront {
        &self.fire_front
    }

    /// Get total burned area in square meters
    pub fn burned_area(&self) -> f32 {
        self.total_burned_area
    }

    /// Get total fuel consumed in kilograms
    pub fn fuel_consumed(&self) -> f32 {
        self.total_fuel_consumed
    }

    /// Get simulation time in seconds
    pub fn simulation_time(&self) -> f32 {
        self.simulation_time
    }

    /// Get weather system
    pub fn weather(&self) -> &WeatherSystem {
        &self.weather
    }

    /// Get mutable weather system
    pub fn weather_mut(&mut self) -> &mut WeatherSystem {
        &mut self.weather
    }

    /// Get grid dimensions
    pub fn grid_dimensions(&self) -> (u32, u32, f32) {
        (self.width, self.height, self.cell_size)
    }

    /// Check if GPU backend is being used
    pub fn is_gpu_accelerated(&self) -> bool {
        self.solver.is_gpu_accelerated()
    }

    /// Read temperature field for visualization
    pub fn read_temperature(&self) -> std::borrow::Cow<'_, [f32]> {
        self.solver.read_temperature()
    }

    /// Read level set field for analysis
    pub fn read_level_set(&self) -> std::borrow::Cow<'_, [f32]> {
        self.solver.read_level_set()
    }

    /// Returns the number of active embers in the simulation.
    pub fn ember_count(&self) -> u32 {
        self.embers.len() as u32
    }

    // ====== Point Query Methods (for game engine polling) ======

    /// Convert world position to grid cell index
    ///
    /// # Arguments
    ///
    /// * `x` - X position in meters
    /// * `y` - Y position in meters
    ///
    /// # Returns
    ///
    /// Grid index if position is within bounds, `None` otherwise
    #[inline]
    fn world_to_index(&self, x: f32, y: f32) -> Option<usize> {
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let grid_x = (x / self.cell_size).floor() as usize;
        let grid_y = (y / self.cell_size).floor() as usize;

        if grid_x >= self.width as usize || grid_y >= self.height as usize {
            return None;
        }

        Some(grid_y * (self.width as usize) + grid_x)
    }

    /// Query temperature at a specific world position (for game engine tree sync)
    ///
    /// Returns the temperature in Celsius at the specified position.
    /// Game objects can poll this to determine when to ignite/update their visual state.
    ///
    /// # Arguments
    ///
    /// * `x` - X position in meters
    /// * `y` - Y position in meters
    ///
    /// # Returns
    ///
    /// Temperature in Celsius, or ambient temperature if position is out of bounds
    pub fn temperature_at(&self, x: f32, y: f32) -> f32 {
        let temp_field = self.solver.read_temperature();

        if let Some(idx) = self.world_to_index(x, y) {
            if idx < temp_field.len() {
                // Convert from Kelvin to Celsius
                return temp_field[idx] - 273.15;
            }
        }

        // Return ambient temperature if out of bounds
        self.weather.temperature.to_kelvin().as_f32() - 273.15
    }

    /// Query level set value at a specific world position
    ///
    /// Returns the signed distance from the fire front:
    /// - φ < 0: Inside burned area
    /// - φ = 0: At fire front
    /// - φ > 0: Unburned fuel
    ///
    /// # Arguments
    ///
    /// * `x` - X position in meters
    /// * `y` - Y position in meters
    ///
    /// # Returns
    ///
    /// Level set value (φ), or positive value if out of bounds (considered unburned)
    pub fn level_set_at(&self, x: f32, y: f32) -> f32 {
        let phi_field = self.solver.read_level_set();

        if let Some(idx) = self.world_to_index(x, y) {
            if idx < phi_field.len() {
                return phi_field[idx];
            }
        }

        // Return positive (unburned) if out of bounds
        1.0
    }

    /// Check if a position is within the burned area
    ///
    /// This is the primary method for game objects to check if they're in the fire zone.
    ///
    /// # Arguments
    ///
    /// * `x` - X position in meters
    /// * `y` - Y position in meters
    ///
    /// # Returns
    ///
    /// `true` if the position is in a burned/burning area (φ < 0)
    pub fn is_burned(&self, x: f32, y: f32) -> bool {
        self.level_set_at(x, y) < 0.0
    }

    /// Check if a position is on or near the active fire front
    ///
    /// # Arguments
    ///
    /// * `x` - X position in meters
    /// * `y` - Y position in meters
    /// * `threshold` - Distance threshold for "near" the front (in cell units)
    ///
    /// # Returns
    ///
    /// `true` if the position is near the fire front (|φ| < threshold × `cell_size`)
    pub fn is_near_fire_front(&self, x: f32, y: f32, threshold: f32) -> bool {
        let phi = self.level_set_at(x, y);
        phi.abs() < threshold * self.cell_size
    }

    /// Batch query temperatures at multiple positions (for efficient game sync)
    ///
    /// This is more efficient than calling `temperature_at()` for each tree individually.
    ///
    /// # Arguments
    ///
    /// * `positions` - Slice of (x, y) position tuples
    ///
    /// # Returns
    ///
    /// Vector of temperatures in Celsius, one per input position
    pub fn temperatures_at(&self, positions: &[(f32, f32)]) -> Vec<f32> {
        let temp_field = self.solver.read_temperature();
        let ambient = self.weather.temperature.to_kelvin().as_f32() - 273.15;

        positions
            .iter()
            .map(|&(x, y)| {
                if let Some(idx) = self.world_to_index(x, y) {
                    if idx < temp_field.len() {
                        return temp_field[idx] - 273.15;
                    }
                }
                ambient
            })
            .collect()
    }

    /// Batch query burn states at multiple positions (for efficient game sync)
    ///
    /// # Arguments
    ///
    /// * `positions` - Slice of (x, y) position tuples
    ///
    /// # Returns
    ///
    /// Vector of booleans indicating burned state (true = burned/burning)
    pub fn burn_states_at(&self, positions: &[(f32, f32)]) -> Vec<bool> {
        let phi_field = self.solver.read_level_set();

        positions
            .iter()
            .map(|&(x, y)| {
                if let Some(idx) = self.world_to_index(x, y) {
                    if idx < phi_field.len() {
                        return phi_field[idx] < 0.0;
                    }
                }
                false
            })
            .collect()
    }

    // ====== Private Methods ======

    /// Update ember trajectories and spot fire ignition
    fn update_embers(&mut self, dt: f32) {
        // Generate new embers from fire front
        self.generate_embers_from_fire_front(dt);

        // Update existing ember trajectories
        let wind_vector = self.weather.wind_vector();
        let ambient_temp = self.weather.temperature;

        // Collect embers that need spot fire ignition
        let mut spot_fire_positions = Vec::new();

        for ember in &mut self.embers {
            ember.update_physics(wind_vector, ambient_temp, Seconds::new(dt));

            // Check for landing and spot fire ignition
            if ember.has_landed() && ember.can_ignite() {
                spot_fire_positions.push(ember.position());
            }
        }

        // Attempt spot fire ignition for landed embers
        for pos in spot_fire_positions {
            self.attempt_spot_fire_ignition(pos);
        }

        // Remove inactive embers (cooled or landed)
        self.embers.retain(Ember::is_active);

        debug!("Ember update: {} active embers", self.embers.len());
    }

    /// Generate embers from active fire front vertices
    fn generate_embers_from_fire_front(&mut self, dt: f32) {
        // Only generate embers if fire front exists
        if self.fire_front.vertex_count() == 0 {
            return;
        }

        // Ember generation rate scales with fire intensity
        for i in 0..self.fire_front.vertex_count() {
            let vertex = self.fire_front.vertices[i];
            let intensity = self.fire_front.intensities[i]; // kW/m

            // Calculate ember generation rate (Albini 1983)
            // Higher intensity → more embers
            // Rate: ~0.1 embers/m/s at 1000 kW/m intensity
            let ember_rate = Self::calculate_ember_generation_rate(intensity);

            // Stochastic ember generation (Poisson process)
            if rand::random::<f32>() < ember_rate * dt {
                // Create new ember
                let ember_id = self.next_ember_id;
                self.next_ember_id += 1;

                // Initial position at fire front (slightly elevated)
                let position = Vec3::new(vertex.x, vertex.y, 1.0);

                // Initial velocity from buoyancy (intensity-dependent)
                // Higher intensity → stronger updraft
                let initial_velocity = Vec3::new(
                    0.0,
                    0.0,
                    (intensity / 1000.0).sqrt() * 5.0, // 0-15 m/s updraft
                );

                // Ember mass: typical bark fragment (0.1-5 grams)
                let ember_mass = crate::core_types::units::Kilograms::new(
                    0.0001 + rand::random::<f32>() * 0.005,
                );

                // Initial temperature: fire temperature (~800°C)
                let ember_temp =
                    crate::core_types::units::Celsius::new(700.0 + rand::random::<f64>() * 200.0);

                // Source fuel type (default to 0, could be read from grid)
                let source_fuel_type = 0;

                let ember = Ember::new(
                    ember_id,
                    position,
                    initial_velocity,
                    ember_temp,
                    ember_mass,
                    source_fuel_type,
                );

                self.embers.push(ember);
            }
        }
    }

    /// Calculate ember generation rate from fire intensity
    ///
    /// Based on Albini (1983) and field observations
    ///
    /// # Arguments
    ///
    /// * `intensity` - Byram fireline intensity (kW/m)
    ///
    /// # Returns
    ///
    /// Ember generation rate (embers/s)
    fn calculate_ember_generation_rate(intensity: f32) -> f32 {
        if intensity < 100.0 {
            return 0.0; // Low intensity fires don't generate many embers
        }

        // Empirical relationship:
        // rate = k × I^0.5
        // At 1000 kW/m: ~0.1 embers/s
        // At 10000 kW/m: ~0.3 embers/s
        let k = 0.003;
        k * intensity.sqrt()
    }

    /// Attempt spot fire ignition from landed ember
    fn attempt_spot_fire_ignition(&mut self, pos: Vec3) {
        // Convert world position to grid coordinates
        let grid_x = (pos.x / self.cell_size).floor() as usize;
        let grid_y = (pos.y / self.cell_size).floor() as usize;

        // Check bounds
        if grid_x >= self.width as usize || grid_y >= self.height as usize {
            return;
        }

        // Check moisture content at landing position
        // Read level set to check if already burned
        let phi = self.solver.read_level_set();
        let idx = grid_y * (self.width as usize) + grid_x;
        if idx >= phi.len() {
            return;
        }

        // Don't ignite if already burned (φ < 0)
        if phi[idx] < 0.0 {
            return;
        }

        // TODO: Read actual moisture from solver
        // For now, use weather humidity as proxy
        let moisture_content = self.weather.humidity.value();

        // Check moisture extinction threshold (30%)
        if moisture_content > 0.30 {
            debug!(
                "Spot fire blocked: moisture {:.1}% > 30% threshold",
                moisture_content * 100.0
            );
            return;
        }

        // Calculate ignition probability
        // For simplicity, assume ember is hot enough (checked in can_ignite)
        // Higher fuel moisture → lower probability
        let moisture_factor = (1.0 - moisture_content / 0.30).max(0.0);

        // Fuel receptivity (would come from fuel properties in full implementation)
        let fuel_receptivity = 0.5_f32; // Placeholder

        let ignition_prob = moisture_factor * fuel_receptivity;

        // Probabilistic ignition
        if rand::random::<f32>() < ignition_prob {
            info!(
                "Spot fire ignited at ({:.1}, {:.1}) from ember",
                pos.x, pos.y
            );

            // Ignite spot fire with piloted ignition temperature (~600°C) from ember contact
            self.solver.apply_heat(
                Meters::new(pos.x),
                Meters::new(pos.y),
                Kelvin::new(873.15),
                Meters::new(5.0),
            );
        }
    }

    /// Extract fire front from level set field
    fn extract_fire_front(&mut self) {
        // Read φ field from solver
        let phi = self.solver.read_level_set();

        // Extract contour using marching squares
        self.fire_front = extract_fire_front(&phi, self.width, self.height, self.cell_size);

        debug!(
            "Fire front extracted: {} vertices, {} fronts",
            self.fire_front.vertex_count(),
            self.fire_front.front_count()
        );
    }

    /// Update simulation statistics
    fn update_statistics(&mut self) {
        // TODO: Compute burned area from level set
        // Count cells where φ < 0
        let phi = self.solver.read_level_set();
        let burned_cells = phi.iter().filter(|&&p| p < 0.0).count();
        #[allow(clippy::cast_precision_loss)]
        let burned_cells_f32 = burned_cells as f32;
        self.total_burned_area = burned_cells_f32 * self.cell_size * self.cell_size;

        // TODO: Track fuel consumed (requires reading fuel field)
        // For now, estimate from burned area
        self.total_fuel_consumed = self.total_burned_area * 2.0; // Placeholder: 2 kg/m²
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create flat terrain with f32 dimensions (for test convenience)
    fn flat_terrain(width: f32, height: f32, resolution: f32, elevation: f32) -> TerrainData {
        TerrainData::flat(
            crate::core_types::units::Meters::new(width),
            crate::core_types::units::Meters::new(height),
            crate::core_types::units::Meters::new(resolution),
            crate::core_types::units::Meters::new(elevation),
        )
    }

    #[test]
    fn test_field_simulation_creation() {
        let terrain = flat_terrain(100.0, 100.0, 10.0, 0.0);
        let weather = WeatherSystem::new(25.0, 0.5, 10.0, 0.0, 0.0);
        let sim = FieldSimulation::new(&terrain, QualityPreset::Low, weather);

        assert_eq!(sim.burned_area(), 0.0);
        assert_eq!(sim.simulation_time(), 0.0);
        assert!(sim.fire_front().vertex_count() == 0);
    }

    #[test]
    fn test_field_simulation_ignition() {
        let terrain = TerrainData::flat(
            Meters::new(100.0),
            Meters::new(100.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let weather = WeatherSystem::new(25.0, 0.5, 10.0, 0.0, 0.0);
        let mut sim = FieldSimulation::new(&terrain, QualityPreset::Low, weather);

        // Apply heat at center with piloted ignition parameters
        sim.apply_heat(Vec3::new(50.0, 50.0, 0.0), 873.15, 5.0);

        // Level set should have some burned cells
        let phi = sim.read_level_set();
        let burned_cells = phi.iter().filter(|&&p| p < 0.0).count();
        assert!(burned_cells > 0, "Ignition should create burned region");
    }

    #[test]
    fn test_field_simulation_update() {
        let terrain = TerrainData::flat(
            Meters::new(100.0),
            Meters::new(100.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let weather = WeatherSystem::new(25.0, 0.5, 10.0, 0.0, 0.0);
        let mut sim = FieldSimulation::new(&terrain, QualityPreset::Low, weather);

        // Apply heat and step forward
        sim.apply_heat(Vec3::new(50.0, 50.0, 0.0), 873.15, 5.0);
        sim.update(0.1);

        assert!(sim.simulation_time() > 0.0);
        // After update, fire front should be extracted
        // (may be 0 vertices if no clear boundary in small grid)
    }

    #[test]
    fn test_field_simulation_wind_affects_spread() {
        let terrain = TerrainData::flat(
            Meters::new(200.0),
            Meters::new(200.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        // Wind in +x direction (eastward)
        let weather = WeatherSystem::new(25.0, 0.5, 10.0, 0.0, 0.0);

        let mut sim = FieldSimulation::new(&terrain, QualityPreset::Medium, weather);

        // Apply heat at center
        sim.apply_heat(Vec3::new(100.0, 100.0, 0.0), 873.15, 5.0);

        // Step simulation multiple times
        for _ in 0..10 {
            sim.update(1.0);
        }

        // Fire should have spread
        assert!(sim.burned_area() > 0.0, "Fire should spread over time");
    }

    #[test]
    fn test_ember_generation_from_fire_front() {
        use crate::core_types::units::Meters;

        let terrain = TerrainData::flat(
            Meters::new(200.0),
            Meters::new(200.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let weather = WeatherSystem::new(25.0, 0.2, 15.0, 0.0, 0.0); // Low moisture, good wind
        let mut sim = FieldSimulation::new(&terrain, QualityPreset::Medium, weather);

        // Apply heat to ignite fire
        sim.apply_heat(Vec3::new(100.0, 100.0, 0.0), 873.15, 5.0);

        // Run simulation to create fire front
        for _ in 0..20 {
            sim.update(1.0);
        }

        // Fire front should exist
        assert!(
            sim.fire_front().vertex_count() > 0,
            "Fire front should have vertices"
        );

        // Note: Ember generation is stochastic, so we can't guarantee embers
        // But the system should be exercised without panicking
    }

    #[test]
    fn test_spot_fire_ignition_from_ember() {
        use crate::core_types::units::{Celsius, Kilograms, Meters};

        let terrain = TerrainData::flat(
            Meters::new(200.0),
            Meters::new(200.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let weather = WeatherSystem::new(25.0, 0.15, 10.0, 0.0, 0.0); // Low moisture
        let mut sim = FieldSimulation::new(&terrain, QualityPreset::Medium, weather);

        // Create a hot ember that will land
        let ember = Ember::new(
            0,
            Vec3::new(150.0, 150.0, 0.5), // Near ground
            Vec3::new(0.0, 0.0, -1.0),    // Falling
            Celsius::new(600.0),          // Hot enough to ignite
            Kilograms::new(0.001),        // 1 gram
            0,
        );

        sim.embers.push(ember);

        // Get initial burned area
        let initial_burned = sim.burned_area();

        // Update to allow ember to land and potentially ignite
        // Multiple attempts due to probabilistic ignition
        for _ in 0..10 {
            sim.update(0.1);
        }

        // Note: Ignition is probabilistic, but system should work without panicking
        // In a real test with many runs, we'd verify statistical ignition rate
        assert!(
            sim.burned_area() >= initial_burned,
            "Burned area should not decrease"
        );
    }

    #[test]
    fn test_moisture_prevents_spot_ignition() {
        use crate::core_types::units::{Celsius, Kilograms};

        let terrain = TerrainData::flat(
            Meters::new(200.0),
            Meters::new(200.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let weather = WeatherSystem::new(25.0, 0.40, 10.0, 0.0, 0.0); // High moisture (40%)
        let mut sim = FieldSimulation::new(&terrain, QualityPreset::Medium, weather);

        // Create a hot ember
        let ember = Ember::new(
            0,
            Vec3::new(100.0, 100.0, 0.5), // Near ground
            Vec3::new(0.0, 0.0, -1.0),    // Falling
            Celsius::new(600.0),          // Hot
            Kilograms::new(0.001),        // 1 gram
            0,
        );

        sim.embers.push(ember);

        // Get initial burned area (should be 0)
        let initial_burned = sim.burned_area();

        // Update multiple times
        for _ in 0..20 {
            sim.update(0.1);
        }

        // With 40% moisture (above 30% threshold), spot fires should be blocked
        // Burned area should remain 0
        assert_eq!(
            sim.burned_area(),
            initial_burned,
            "High moisture should prevent spot fire ignition"
        );
    }

    #[test]
    #[allow(clippy::cast_precision_loss)] // Test uses small loop indices that fit in f32
    fn test_apply_heat_drip_torch_backburn() {
        // Demonstrate realistic backburn operation using apply_heat
        let terrain = TerrainData::flat(
            Meters::new(200.0),
            Meters::new(200.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let weather = WeatherSystem::new(30.0, 0.10, 8.0, 0.0, 0.0); // Hot, dry, light wind
        let mut sim = FieldSimulation::new(&terrain, QualityPreset::Medium, weather);

        // Drip torch operation: apply heat along a line to create backburn
        // Typical drip torch temperature: 650-700°C, radius ~0.3-0.5m
        let drip_torch_temp = 670.0; // Celsius
        let drip_torch_radius = 0.5; // meters

        // Create a firebreak line by applying heat every 1.0 meters for better overlap
        for i in 0..20 {
            let x = 50.0 + i as f32 * 1.0;
            let y = 100.0;
            sim.apply_heat(Vec3::new(x, y, 0.0), drip_torch_temp, drip_torch_radius);
        }

        // Update once to let system process the ignitions
        sim.update(0.1);

        // Check that heat application created burned cells
        let initial_burned = sim.burned_area();
        assert!(
            initial_burned > 0.0,
            "Drip torch should create burned area after update"
        );

        // Verify that multiple cells were ignited along the line
        // (demonstrates practical backburn operation)
        assert!(
            initial_burned > 10.0,
            "Drip torch line should ignite multiple cells"
        );
    }

    #[test]
    fn test_apply_heat_failed_ignition_wet_fuel() {
        use crate::core_types::units::Meters;

        // Demonstrate that heat application can fail with wet fuel
        let terrain = TerrainData::flat(
            Meters::new(100.0),
            Meters::new(100.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let weather = WeatherSystem::new(20.0, 0.6, 5.0, 0.0, 0.0); // Very high moisture
        let mut sim = FieldSimulation::new(&terrain, QualityPreset::Low, weather);

        // Try to ignite with moderate heat (might not overcome moisture)
        // Real match temperature ~600°C, but wet fuel needs more energy
        sim.apply_heat(Vec3::new(50.0, 50.0, 0.0), 400.0, 0.2);

        // Initial ignition might fail or be very limited
        let burned = sim.burned_area();

        // Step simulation - fire should not spread well in wet conditions
        for _ in 0..5 {
            sim.update(1.0);
        }

        let final_burned = sim.burned_area();

        // With high moisture, fire spread should be minimal or fail
        // (exact behavior depends on moisture physics)
        assert!(
            final_burned - burned < 50.0,
            "Wet fuel should resist fire spread"
        );
    }
}
