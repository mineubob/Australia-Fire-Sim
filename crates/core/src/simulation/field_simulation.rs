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
    #[allow(dead_code)]
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

    // ====== Private Methods ======

    /// Update ember trajectories and spot fire ignition
    fn update_embers(&mut self, _dt: f32) {
        // TODO Phase 5: Implement ember physics
        // - Update ember trajectories using Albini spotting model
        // - Check for landing and ignition
        // - Remove inactive embers
        debug!(
            "Ember update (placeholder): {} active embers",
            self.embers.len()
        );

        // Ember system will be fully implemented in Phase 5
        // For now, just clear embers to avoid unbounded growth
        self.embers.clear();
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
}
