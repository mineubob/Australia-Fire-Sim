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
pub mod grid;
pub mod physics;
pub mod simulation;
pub mod suppression;
pub mod weather;

// Re-export core types
pub use core_types::{BarkProperties, Fuel, FuelElement, FuelPart, Vec3};
pub use core_types::{ClimatePattern, WeatherPreset, WeatherSystem};
pub use core_types::{Ember, SpatialIndex};

// Re-export ultra-realistic types
pub use grid::{GridCell, SimulationGrid, TerrainData};
pub use physics::{apply_suppression_direct, SuppressionAgent};
pub use simulation::{ActionQueue, FireSimulation, PlayerAction, PlayerActionType, SimulationStats};

// Re-export suppression types (Phase 1)
pub use suppression::{SuppressionAgentProperties, SuppressionAgentType, SuppressionCoverage};

// Re-export weather types (Phase 2)
pub use weather::{AtmosphericProfile, PyrocumulusCloud};
