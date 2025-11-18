//! Fire Simulation Core Library
//! 
//! A scientifically accurate wildfire simulation system based on Australian bushfire research.
//! Implements physics-based fire spread, ember generation, and Australian-specific behaviors
//! like eucalyptus oil explosions and stringybark ladder fuels.
//! 
//! ## Ultra-Realistic Fire Simulation
//! 
//! The new ultra-realistic simulation system includes:
//! - 3D atmospheric grid with terrain elevation support
//! - Chemistry-based combustion with Arrhenius kinetics
//! - Element-grid coupling for realistic fire-atmosphere interaction
//! - Advanced suppression physics (water, retardant, foam)

// Core types and utilities
pub mod core_types;

// Legacy simulation modules (kept for compatibility)
pub mod legacy;

// Ultra-realistic simulation modules (organized in subfolders)
pub mod physics;
pub mod grid;
pub mod ultra;

// Re-export core types
pub use core_types::{Fuel, BarkProperties, FuelElement, FuelPart, Vec3};
pub use core_types::{WeatherSystem, WeatherPreset, ClimatePattern};
pub use core_types::{Ember, SpatialIndex};

// Re-export legacy types
pub use legacy::{FireSimulation, PyroCb, PyroCbSystem, LightningStrike, Downdraft};

// Re-export ultra-realistic types
pub use grid::{TerrainData, SimulationGrid, GridCell};
pub use physics::{SuppressionAgent, SuppressionDroplet, AircraftDrop, GroundSuppression};
pub use ultra::{FireSimulationUltra, SimulationStats};

