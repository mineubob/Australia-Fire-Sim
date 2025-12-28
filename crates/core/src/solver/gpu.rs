//! GPU-based field solver implementation
//!
//! This module provides a GPU implementation of the `FieldSolver` trait using
//! wgpu compute shaders and textures. This backend is only available when the
//! `gpu` feature is enabled.

#![cfg(feature = "gpu")]

use super::context::GpuContext;
use super::quality::QualityPreset;
use super::FieldSolver;
use crate::TerrainData;
use std::borrow::Cow;
use tracing::debug;

/// GPU-based field solver using wgpu compute shaders
///
/// This solver stores all field data as GPU textures and uses compute shaders
/// for parallel computation. It implements the same physics as the CPU solver
/// but runs on the GPU for much better performance.
pub struct GpuFieldSolver {
    context: GpuContext,

    // Grid dimensions
    width: u32,
    height: u32,
    cell_size: f32,

    // Staging buffers for CPU readback (allocated lazily)
    temperature_staging: Option<Vec<f32>>,
    level_set_staging: Option<Vec<f32>>,
}

impl GpuFieldSolver {
    /// Create a new GPU field solver
    ///
    /// Initializes all GPU textures and compute pipelines based on terrain data
    /// and quality preset.
    ///
    /// # Arguments
    ///
    /// * `context` - GPU context with device and queue
    /// * `terrain` - Terrain data for initialization
    /// * `quality` - Quality preset determining grid resolution
    ///
    /// # Returns
    ///
    /// New GPU field solver instance
    #[must_use]
    pub fn new(context: GpuContext, terrain: &TerrainData, quality: QualityPreset) -> Self {
        let (width, height, cell_size) = quality.grid_dimensions(terrain);

        debug!(
            "Creating GPU field solver: {}x{} grid, {:.2}m cells",
            width, height, cell_size
        );

        // TODO Phase 2: Create GPU textures for fields
        // TODO Phase 2: Create compute pipelines
        // TODO Phase 2: Initialize textures from terrain

        Self {
            context,
            width,
            height,
            cell_size,
            temperature_staging: None,
            level_set_staging: None,
        }
    }
}

impl FieldSolver for GpuFieldSolver {
    fn step_heat_transfer(&mut self, _dt: f32, _wind_x: f32, _wind_y: f32, _ambient_temp: f32) {
        // TODO Phase 2: Dispatch heat transfer compute shader
        debug!("GPU heat transfer pass (placeholder)");
    }

    fn step_combustion(&mut self, _dt: f32) {
        // TODO Phase 2: Dispatch combustion compute shader
        debug!("GPU combustion pass (placeholder)");
    }

    fn step_moisture(&mut self, _dt: f32, _humidity: f32) {
        // TODO Phase 2: Dispatch moisture compute shader
        debug!("GPU moisture pass (placeholder)");
    }

    fn step_level_set(&mut self, _dt: f32) {
        // TODO Phase 3: Dispatch level set compute shader
        debug!("GPU level set pass (placeholder)");
    }

    fn step_ignition_sync(&mut self) {
        // TODO Phase 3: Dispatch ignition sync compute shader
        debug!("GPU ignition sync pass (placeholder)");
    }

    fn read_temperature(&self) -> Cow<[f32]> {
        // TODO Phase 2: Read temperature texture from GPU
        // For now, return empty vec (will be implemented in Phase 2)
        let size = (self.width * self.height) as usize;
        Cow::Owned(vec![293.15; size]) // Placeholder: ambient temperature
    }

    fn read_level_set(&self) -> Cow<[f32]> {
        // TODO Phase 3: Read level set texture from GPU
        // For now, return empty vec (will be implemented in Phase 3)
        let size = (self.width * self.height) as usize;
        Cow::Owned(vec![f32::MAX; size]) // Placeholder: all unburned
    }

    fn ignite_at(&mut self, _x: f32, _y: f32, _radius: f32) {
        // TODO Phase 3: Upload ignition region to GPU
        debug!("GPU ignite_at (placeholder)");
    }

    fn dimensions(&self) -> (u32, u32, f32) {
        (self.width, self.height, self.cell_size)
    }

    fn is_gpu_accelerated(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::context::GpuInitResult;

    #[test]
    fn test_gpu_solver_creation() {
        // Only run if GPU is available
        if let GpuInitResult::Success(context) = GpuContext::new() {
            let terrain = TerrainData::flat(1000.0, 1000.0, 10.0, 0.0);
            let solver = GpuFieldSolver::new(context, &terrain, QualityPreset::Medium);

            let (width, height, cell_size) = solver.dimensions();
            assert_eq!(width, 100);
            assert_eq!(height, 100);
            assert_eq!(cell_size, 10.0);
            assert!(solver.is_gpu_accelerated());
        }
    }

    #[test]
    fn test_gpu_solver_read_temperature() {
        // Only run if GPU is available
        if let GpuInitResult::Success(context) = GpuContext::new() {
            let terrain = TerrainData::flat(100.0, 100.0, 10.0, 0.0);
            let solver = GpuFieldSolver::new(context, &terrain, QualityPreset::Low);

            let temp = solver.read_temperature();
            assert!(!temp.is_empty());
            // Placeholder returns ambient temperature
            assert!(temp.iter().all(|&t| (t - 293.15).abs() < 0.1));
        }
    }

    #[test]
    fn test_gpu_solver_dimensions() {
        // Only run if GPU is available
        if let GpuInitResult::Success(context) = GpuContext::new() {
            let terrain = TerrainData::flat(500.0, 300.0, 10.0, 0.0);
            let solver = GpuFieldSolver::new(context, &terrain, QualityPreset::High);

            let (width, height, _cell_size) = solver.dimensions();
            // 500m / 5m per cell = 100, 300m / 5m per cell = 60
            assert_eq!(width, 100);
            assert_eq!(height, 60);
        }
    }
}
