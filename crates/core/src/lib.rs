//! Fire Simulation Core Library
//!
//! A scientifically accurate wildfire simulation system with GPU/CPU-accelerated field-based physics.
//!
//! ## Field-Based Fire Simulation
//!
//! This simulation system includes:
//! - GPU/CPU-accelerated field solver with automatic backend selection
//! - Stefan-Boltzmann radiation with full T⁴ formula (no simplifications)
//! - Thermal diffusion and wind advection
//! - Level set fire front tracking with curvature-dependent spread
//! - Moisture evaporation physics (2260 kJ/kg latent heat FIRST)
//! - Fuel consumption and combustion chemistry
//! - Ember generation and spot fire ignition (Albini model)
//! - Fire front extraction via marching squares
//! - Realistic physics never simplified (per project requirements)

// Core types and utilities
pub mod core_types;

// Ultra-realistic simulation modules (organized in subfolders)
pub(crate) mod grid;
pub mod physics; // Made pub for FFI access to SuppressionAgent
pub mod simulation;
pub mod solver; // New GPU/CPU field solver abstraction
pub mod suppression; // Made pub for FFI access to SuppressionAgentType
pub(crate) mod weather;

// Re-export core types (public API)
pub use core_types::Ember;
pub use core_types::{BarkProperties, Fuel, FuelElement, FuelPart, Vec3};
pub use core_types::{ClimatePattern, WeatherPreset, WeatherSystem};

/// Re-export FFDI ranges for validation and testing
pub use core_types::weather::ffdi_ranges;

// Re-export simulation types (public API)
pub use grid::{GridCell, SimulationGrid, TerrainData};
pub use grid::{PlameSource, StabilityClass, WindField, WindFieldConfig};
pub use simulation::FieldSimulation;

// Re-export suppression types (for FFI)
pub use physics::SuppressionAgent;
pub use suppression::SuppressionAgentType;

// Re-export physics types for integration tests
pub use physics::CombustionPhase;

// Re-export multiplayer types (for FFI)
pub use simulation::{PlayerAction, PlayerActionType};
#[cfg(test)]
mod test_tracing {
    // Initialize tracing for all tests in this crate so `tracing` logs are available
    // when running `cargo test`. This runs only in test builds and executes once
    // at test binary startup using the `ctor` crate.
    use ctor::ctor;

    #[ctor]
    fn init_tracing() {
        // Initialize tracing subscriber; respect RUST_LOG / default env filter.
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }
}
