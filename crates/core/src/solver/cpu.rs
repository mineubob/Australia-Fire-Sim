//! CPU-based field solver implementation
//!
//! This module provides a CPU implementation of the `FieldSolver` trait using
//! `Vec<f32>` arrays and Rayon for parallelism. This backend is always available
//! and serves as a fallback when GPU acceleration is not available.

use super::combustion::{step_combustion_cpu, CombustionParams};
use super::fields::FieldData;
use super::heat_transfer::{step_heat_transfer_cpu, HeatTransferParams};
use super::quality::QualityPreset;
use super::FieldSolver;
use crate::TerrainData;
use std::borrow::Cow;

/// CPU-based field solver using Rayon for parallelism
///
/// This solver stores all field data as `Vec<f32>` arrays and uses Rayon's
/// parallel iterators for multi-threaded computation. It implements the same
/// physics as the GPU solver but runs on the CPU.
pub struct CpuFieldSolver {
    // Ping-pong buffers for each field (read from one, write to other, then swap)
    temperature: FieldData,
    temperature_back: FieldData,

    // Additional fields (used in Phase 2)
    fuel_load: FieldData,
    moisture: FieldData,
    level_set: FieldData,
    #[allow(dead_code)]
    level_set_back: FieldData,
    oxygen: FieldData,

    // Static fields (don't change during simulation)
    #[allow(dead_code)]
    fuel_type: Vec<u8>,
    #[allow(dead_code)]
    terrain_height: Vec<f32>,

    // Grid dimensions
    width: usize,
    height: usize,
    cell_size: f32,
}

impl CpuFieldSolver {
    /// Create a new CPU field solver
    ///
    /// Initializes all fields based on terrain data and quality preset.
    ///
    /// # Arguments
    ///
    /// * `terrain` - Terrain data for initialization
    /// * `quality` - Quality preset determining grid resolution
    ///
    /// # Returns
    ///
    /// New CPU field solver instance
    #[must_use]
    pub fn new(terrain: &TerrainData, quality: QualityPreset) -> Self {
        let (width_u32, height_u32, cell_size) = quality.grid_dimensions(terrain);
        let width = width_u32 as usize;
        let height = height_u32 as usize;
        let num_cells = width * height;

        // Initialize fields
        let temperature = FieldData::with_value(width, height, 293.15); // ~20°C ambient
        let temperature_back = FieldData::new(width, height);
        let fuel_load = FieldData::with_value(width, height, 1.0); // 1 kg/m² default
        let moisture = FieldData::with_value(width, height, 0.1); // 10% moisture default
        let level_set = FieldData::with_value(width, height, f32::MAX); // All unburned initially
        let level_set_back = FieldData::new(width, height);
        let oxygen = FieldData::with_value(width, height, 0.21); // Atmospheric O₂ fraction

        // Initialize static fields
        let fuel_type = vec![0_u8; num_cells]; // Default fuel type
        let terrain_height = vec![0.0_f32; num_cells]; // Flat terrain default

        Self {
            temperature,
            temperature_back,
            fuel_load,
            moisture,
            level_set,
            level_set_back,
            oxygen,
            fuel_type,
            terrain_height,
            width,
            height,
            cell_size,
        }
    }
}

impl FieldSolver for CpuFieldSolver {
    fn step_heat_transfer(&mut self, dt: f32, wind_x: f32, wind_y: f32, ambient_temp: f32) {
        // Use Phase 2 heat transfer physics
        let params = HeatTransferParams {
            dt,
            wind_x,
            wind_y,
            ambient_temp,
            cell_size: self.cell_size,
        };

        step_heat_transfer_cpu(
            self.temperature.as_slice(),
            self.temperature_back.as_mut_slice(),
            self.level_set.as_slice(),
            self.fuel_load.as_slice(),
            self.width,
            self.height,
            params,
        );

        // Swap buffers
        std::mem::swap(&mut self.temperature, &mut self.temperature_back);
    }

    fn step_combustion(&mut self, dt: f32) {
        // Use Phase 2 combustion physics
        let params = CombustionParams {
            dt,
            cell_size: self.cell_size,
        };

        let _heat_release = step_combustion_cpu(
            self.temperature.as_slice(),
            self.fuel_load.as_mut_slice(),
            self.moisture.as_mut_slice(),
            self.oxygen.as_mut_slice(),
            self.level_set.as_slice(),
            self.width,
            self.height,
            params,
        );

        // TODO Phase 2: Add heat_release to temperature field in next heat transfer step
    }

    fn step_moisture(&mut self, _dt: f32, _humidity: f32) {
        // Placeholder - will implement in Phase 2
    }

    fn step_level_set(&mut self, _dt: f32) {
        // Placeholder - will implement in Phase 3
    }

    fn step_ignition_sync(&mut self) {
        // Placeholder - will implement in Phase 3
    }

    fn read_temperature(&self) -> Cow<'_, [f32]> {
        Cow::Borrowed(self.temperature.as_slice())
    }

    fn read_level_set(&self) -> Cow<'_, [f32]> {
        Cow::Borrowed(self.level_set.as_slice())
    }

    fn ignite_at(&mut self, x: f32, y: f32, radius: f32) {
        // Convert world coordinates to grid coordinates
        let grid_x = (x / self.cell_size) as i32;
        let grid_y = (y / self.cell_size) as i32;
        let grid_radius = (radius / self.cell_size) as i32;

        // Set φ < 0 in circular region and raise temperature
        for dy in -grid_radius..=grid_radius {
            for dx in -grid_radius..=grid_radius {
                let dist_sq = dx * dx + dy * dy;
                let radius_sq = grid_radius * grid_radius;

                if dist_sq <= radius_sq {
                    let gx = grid_x + dx;
                    let gy = grid_y + dy;

                    // Check bounds
                    if gx >= 0 && gx < self.width as i32 && gy >= 0 && gy < self.height as i32 {
                        let idx = (gy as usize) * self.width + (gx as usize);

                        // Set level set to negative (burning)
                        self.level_set.as_mut_slice()[idx] = -1.0;

                        // Set high temperature to initiate combustion
                        self.temperature.as_mut_slice()[idx] = 600.0; // ~327°C (ignition temp)
                    }
                }
            }
        }
    }

    fn dimensions(&self) -> (u32, u32, f32) {
        (self.width as u32, self.height as u32, self.cell_size)
    }

    fn is_gpu_accelerated(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_solver_creation() {
        let terrain = TerrainData::flat(1000.0, 1000.0, 10.0, 0.0);
        let solver = CpuFieldSolver::new(&terrain, QualityPreset::Medium);

        let (width, height, cell_size) = solver.dimensions();
        assert_eq!(width, 100);
        assert_eq!(height, 100);
        assert_eq!(cell_size, 10.0);
        assert!(!solver.is_gpu_accelerated());
    }

    #[test]
    fn test_cpu_solver_read_temperature() {
        let terrain = TerrainData::flat(100.0, 100.0, 10.0, 0.0);
        let solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        let temp = solver.read_temperature();
        assert!(!temp.is_empty());
        // Should be initialized to ambient temperature (~293.15 K)
        assert!(temp.iter().all(|&t| (t - 293.15).abs() < 0.1));
    }

    #[test]
    fn test_cpu_solver_read_level_set() {
        let terrain = TerrainData::flat(100.0, 100.0, 10.0, 0.0);
        let solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        let level_set = solver.read_level_set();
        assert!(!level_set.is_empty());
        // Should be initialized to MAX (all unburned)
        assert!(level_set.iter().all(|&phi| phi == f32::MAX));
    }

    #[test]
    fn test_cpu_solver_ignite_at() {
        let terrain = TerrainData::flat(1000.0, 1000.0, 10.0, 0.0);
        let mut solver = CpuFieldSolver::new(&terrain, QualityPreset::Medium);

        // Ignite at center with 50m radius
        solver.ignite_at(500.0, 500.0, 50.0);

        let level_set = solver.read_level_set();
        let temperature = solver.read_temperature();

        // Check that some cells are now burning (φ < 0)
        let burning_cells = level_set.iter().filter(|&&phi| phi < 0.0).count();
        assert!(burning_cells > 0, "No cells were ignited");

        // Check that some cells have elevated temperature
        let hot_cells = temperature.iter().filter(|&&t| t > 400.0).count();
        assert!(hot_cells > 0, "No cells were heated");
    }

    #[test]
    fn test_cpu_solver_heat_transfer() {
        let terrain = TerrainData::flat(100.0, 100.0, 10.0, 0.0);
        let mut solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        // Ignite a spot to create temperature gradient
        solver.ignite_at(50.0, 50.0, 10.0);

        let temp_before = solver.read_temperature().to_vec();

        // Run heat transfer step
        solver.step_heat_transfer(1.0, 0.0, 0.0, 293.15);

        let temp_after = solver.read_temperature();

        // Temperature field should have changed
        let changed = temp_before
            .iter()
            .zip(temp_after.iter())
            .any(|(&before, &after)| (before - after).abs() > 0.01);
        assert!(changed, "Temperature field did not change");
    }
}
