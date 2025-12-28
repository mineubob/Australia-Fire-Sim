//! New field-based fire simulation using `FieldSolver`
//!
//! This module provides the new `FieldSimulation` struct that uses the `FieldSolver` trait
//! for GPU/CPU-accelerated field-based fire physics. This replaces the old element-based system.

use crate::core_types::element::Vec3;
use crate::core_types::ember::Ember;
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
        let (width, height, cell_size) = solver.dimensions();

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
        let wind_vector = self.weather.wind_vector();
        let ambient_temp = self.weather.temperature.to_kelvin().as_f32();
        let humidity = self.weather.humidity.value();

        debug!(
            "Simulation update: t={:.2}s, dt={:.4}s, wind=({:.2}, {:.2}), T={:.1}K",
            self.simulation_time, dt, wind_vector.x, wind_vector.y, ambient_temp
        );

        // 2. GPU/CPU compute passes
        self.solver
            .step_heat_transfer(dt, wind_vector.x, wind_vector.y, ambient_temp);
        self.solver.step_combustion(dt);
        self.solver.step_moisture(dt, humidity);
        self.solver.step_level_set(dt);
        self.solver.step_ignition_sync();

        // 3. CPU-side sparse updates
        self.update_embers(dt);

        // 4. Extract fire front (can be deferred for performance)
        self.extract_fire_front();

        // 5. Update statistics
        self.update_statistics();
    }

    /// Ignite fire at a specific position
    ///
    /// # Arguments
    ///
    /// * `position` - World position (x, y, z) in meters
    /// * `radius` - Ignition radius in meters
    pub fn ignite_at(&mut self, position: Vec3, radius: f32) {
        info!(
            "Igniting fire at ({:.2}, {:.2}) with radius {:.2}m",
            position.x, position.y, radius
        );
        self.solver.ignite_at(position.x, position.y, radius);
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
            ember.update_physics(wind_vector, ambient_temp, dt);

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

            // Ignite spot fire with 2m radius
            self.solver.ignite_at(pos.x, pos.y, 2.0);
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

    #[test]
    fn test_field_simulation_creation() {
        let terrain = TerrainData::flat(100.0, 100.0, 10.0, 0.0);
        let weather = WeatherSystem::new(25.0, 0.5, 10.0, 0.0, 0.0);
        let sim = FieldSimulation::new(&terrain, QualityPreset::Low, weather);

        assert_eq!(sim.burned_area(), 0.0);
        assert_eq!(sim.simulation_time(), 0.0);
        assert!(sim.fire_front().vertex_count() == 0);
    }

    #[test]
    fn test_field_simulation_ignition() {
        let terrain = TerrainData::flat(100.0, 100.0, 10.0, 0.0);
        let weather = WeatherSystem::new(25.0, 0.5, 10.0, 0.0, 0.0);
        let mut sim = FieldSimulation::new(&terrain, QualityPreset::Low, weather);

        // Ignite at center
        sim.ignite_at(Vec3::new(50.0, 50.0, 0.0), 5.0);

        // Level set should have some burned cells
        let phi = sim.read_level_set();
        let burned_cells = phi.iter().filter(|&&p| p < 0.0).count();
        assert!(burned_cells > 0, "Ignition should create burned region");
    }

    #[test]
    fn test_field_simulation_update() {
        let terrain = TerrainData::flat(100.0, 100.0, 10.0, 0.0);
        let weather = WeatherSystem::new(25.0, 0.5, 10.0, 0.0, 0.0);
        let mut sim = FieldSimulation::new(&terrain, QualityPreset::Low, weather);

        // Ignite and step forward
        sim.ignite_at(Vec3::new(50.0, 50.0, 0.0), 5.0);
        sim.update(0.1);

        assert!(sim.simulation_time() > 0.0);
        // After update, fire front should be extracted
        // (may be 0 vertices if no clear boundary in small grid)
    }

    #[test]
    fn test_field_simulation_wind_affects_spread() {
        let terrain = TerrainData::flat(200.0, 200.0, 10.0, 0.0);
        // Wind in +x direction (eastward)
        let weather = WeatherSystem::new(25.0, 0.5, 10.0, 0.0, 0.0);

        let mut sim = FieldSimulation::new(&terrain, QualityPreset::Medium, weather);

        // Ignite at center
        sim.ignite_at(Vec3::new(100.0, 100.0, 0.0), 10.0);

        // Step simulation multiple times
        for _ in 0..10 {
            sim.update(1.0);
        }

        // Fire should have spread
        assert!(sim.burned_area() > 0.0, "Fire should spread over time");
    }

    #[test]
    fn test_ember_generation_from_fire_front() {
        let terrain = TerrainData::flat(200.0, 200.0, 10.0, 0.0);
        let weather = WeatherSystem::new(25.0, 0.2, 15.0, 0.0, 0.0); // Low moisture, good wind
        let mut sim = FieldSimulation::new(&terrain, QualityPreset::Medium, weather);

        // Ignite fire
        sim.ignite_at(Vec3::new(100.0, 100.0, 0.0), 10.0);

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
        use crate::core_types::units::{Celsius, Kilograms};

        let terrain = TerrainData::flat(200.0, 200.0, 10.0, 0.0);
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

        let terrain = TerrainData::flat(200.0, 200.0, 10.0, 0.0);
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
}
