//! Fire simulation integrating advanced physics systems
//!
//! This module provides the field-based `FieldSimulation` system for GPU/CPU-accelerated
//! realistic bushfire simulation.
//!
//! # Fire Simulation System
//!
//! - **`FieldSimulation`**: GPU/CPU-accelerated field-based physics with level set fire fronts
//!
//! ## `FieldSimulation`
//!
//! `FieldSimulation` uses continuous 2D fields for:
//! - Temperature distribution (Stefan-Boltzmann radiation, thermal diffusion)
//! - Fuel consumption and combustion
//! - Fire front tracking via level sets with curvature-dependent spread
//! - Ember generation and spot fire ignition
//! - Automatic GPU acceleration with CPU fallback
//!
//! ### Example
//!
//! ```rust
//! use fire_sim_core::simulation::FieldSimulation;
//! use fire_sim_core::solver::QualityPreset;
//! use fire_sim_core::{TerrainData, WeatherSystem};
//! use fire_sim_core::Vec3;
//!
//! // Create field-based simulation (auto-selects GPU or CPU)
//! let terrain = TerrainData::flat(1000.0, 1000.0, 10.0, 0.0);
//! let weather = WeatherSystem::new(25.0, 0.5, 10.0, 0.0, 0.0);
//! let mut sim = FieldSimulation::new(&terrain, QualityPreset::High, weather);
//!
//! // Ignite fire
//! sim.ignite_at(Vec3::new(500.0, 500.0, 0.0), 10.0);
//!
//! // Run simulation (includes ember generation and spot fires)
//! for _ in 0..100 {
//!     sim.update(0.1);  // 0.1s timestep
//! }
//!
//! // Query results
//! let burned_area = sim.burned_area();
//! let fire_front = sim.fire_front();
//! ```

pub mod action_queue;
pub mod field_simulation;

// Re-export public types from action_queue
pub use action_queue::{PlayerAction, PlayerActionType};
// Keep ActionQueue internal (for future multiplayer support)
#[expect(unused_imports)]
pub(crate) use action_queue::ActionQueue;

// Re-export field-based simulation
pub use field_simulation::FieldSimulation;
