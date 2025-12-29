//! Field-based fire simulation solver module
//!
//! This module provides a unified GPU/CPU abstraction layer for field-based fire physics.
//! The core abstraction is the `FieldSolver` trait, which has both CPU and GPU implementations.
//!
//! # Feature Flags
//!
//! - `gpu` (default): Enables GPU acceleration via wgpu. Disable with `--no-default-features`
//!   for environments without GPU access.
//!
//! # Backend Selection
//!
//! The system automatically selects the best available backend:
//! 1. Try GPU (if `gpu` feature enabled and hardware available)
//! 2. Fall back to CPU (always available)
//!
//! # Example
//!
//! ```rust,ignore
//! use fire_sim_core::solver::{create_field_solver, QualityPreset};
//! use fire_sim_core::TerrainData;
//!
//! let terrain = TerrainData::flat(1000.0, 1000.0, 10.0, 0.0);
//! let solver = create_field_solver(&terrain, QualityPreset::Medium);
//! ```

mod combustion;
mod context;
mod cpu;
pub mod crown_fire;
mod fields;
pub mod fuel_layers;
pub mod fuel_variation;
mod heat_transfer;
mod level_set;
pub mod marching_squares;
pub mod noise;
pub mod profiler;
mod quality;
pub mod terrain_slope;
#[allow(clippy::module_name_repetitions)]
mod r#trait;
pub mod vertical_heat_transfer;

#[cfg(feature = "gpu")]
mod gpu;

// Re-exports
pub use context::GpuInitResult;
pub use cpu::CpuFieldSolver;
pub use crown_fire::{CanopyProperties, CrownFirePhysics, CrownFireState};
pub use fields::FieldData;
pub use fuel_layers::{FuelLayer, LayerState, LayeredFuelCell};
pub use fuel_variation::{
    apply_fuel_heterogeneity, apply_heterogeneity_single, calculate_aspect_moisture_factor,
    HeterogeneityConfig,
};
pub use marching_squares::{extract_fire_front, FireFront};
pub use noise::{NoiseGenerator, NoiseOctave};
pub use profiler::{FrameTimer, ProfilerScope};
pub use quality::QualityPreset;
pub use r#trait::FieldSolver;
pub use terrain_slope::{calculate_effective_slope, calculate_slope_factor, TerrainFields};
pub use vertical_heat_transfer::{
    FluxParams, VerticalHeatTransfer, LATENT_HEAT_WATER, STEFAN_BOLTZMANN,
};

#[cfg(feature = "gpu")]
pub use context::GpuContext;
#[cfg(feature = "gpu")]
pub use gpu::GpuFieldSolver;

use crate::TerrainData;
use tracing::info;

#[cfg(feature = "gpu")]
use tracing::warn;

/// Create a field solver with automatic backend selection
///
/// This function tries to use GPU acceleration if available, falling back to CPU otherwise.
/// The selection process is:
/// 1. If `gpu` feature is enabled, try to initialize GPU
/// 2. If GPU initialization fails or feature is disabled, use CPU
///
/// # Arguments
///
/// * `terrain` - Terrain data for the simulation
/// * `quality` - Quality preset determining grid resolution
///
/// # Returns
///
/// A boxed `FieldSolver` trait object using the best available backend
pub fn create_field_solver(terrain: &TerrainData, quality: QualityPreset) -> Box<dyn FieldSolver> {
    #[cfg(feature = "gpu")]
    {
        use context::GpuContext;

        match GpuContext::new() {
            GpuInitResult::Success(gpu_context) => {
                let (width, height, _cell_size) = quality.grid_dimensions(terrain);
                if gpu_context.can_allocate(width, height) {
                    info!(
                        "Using GPU backend: {} ({}x{} grid)",
                        gpu_context.adapter_name(),
                        width,
                        height
                    );
                    return Box::new(GpuFieldSolver::new(gpu_context, terrain, quality));
                }
                warn!(
                    "GPU has insufficient memory for {}x{} grid, falling back to CPU",
                    width, height
                );
            }
            GpuInitResult::NoGpuFound => {
                info!("No GPU found, using CPU backend");
            }
            GpuInitResult::InitFailed {
                adapter_name,
                error,
            } => {
                warn!(
                    "GPU '{}' found but failed to initialize: {}. Falling back to CPU.",
                    adapter_name, error
                );
            }
        }
    }

    #[cfg(not(feature = "gpu"))]
    info!("GPU feature disabled, using CPU backend");

    Box::new(CpuFieldSolver::new(terrain, quality))
}
