//! Fire Simulation Core Library
//!
//! An ultra-realistic wildfire simulation system with scientifically accurate physics.
//!
//! ## Ultra-Realistic Fire Simulation
//!
//! This simulation system includes:
//! - 3D atmospheric grid with terrain elevation support
//! - Chemistry-based combustion with Arrhenius kinetics
//! - Element-grid coupling for realistic fire-atmosphere interaction
//! - Advanced suppression physics (water, retardant, foam)
//! - Buoyancy-driven convection and plume dynamics
//! - Multi-band radiation transfer
//! - Fuel element suppression coverage tracking
//! - Ember spot fire ignition with suppression blocking
//! - Advanced weather phenomena (pyrocumulus, atmospheric instability)
//! - Terrain-based fire spread physics (Phase 3)
//! - Multiplayer action queue system (Phase 5)

// Core types and utilities
pub mod core_types;

// Ultra-realistic simulation modules (organized in subfolders)
pub(crate) mod grid;
pub mod physics; // Made pub for FFI access to SuppressionAgent
pub mod simulation;
pub mod suppression; // Made pub for FFI access to SuppressionAgentType
pub(crate) mod weather;

// Re-export core types (public API)
pub use core_types::Ember;
pub use core_types::{BarkProperties, Fuel, FuelElement, FuelPart, Vec3};
pub use core_types::{ClimatePattern, WeatherPreset, WeatherSystem};

// Re-export simulation types (public API)
pub use grid::{GridCell, SimulationGrid, TerrainData};
pub use simulation::{FireSimulation, SimulationStats};

// Re-export suppression types (for FFI)
pub use physics::SuppressionAgent;
pub use suppression::SuppressionAgentType;

// Re-export physics types for integration tests
pub use physics::CombustionPhase;

// Re-export multiplayer types (for FFI)
pub use simulation::{PlayerAction, PlayerActionType};
