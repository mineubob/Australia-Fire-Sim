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

// Legacy modules (kept for compatibility with existing code)
pub mod fuel;
pub mod element;
pub mod spatial;
pub mod legacy_physics;
pub mod weather;
pub mod ember;
pub mod australian;
pub mod pyrocumulonimbus;
pub mod simulation;

// Ultra-realistic simulation modules (organized in subfolders)
pub mod physics;
pub mod grid;

// Re-export main types
pub use fuel::{Fuel, BarkProperties};
pub use element::{FuelElement, FuelPart, Vec3};
pub use weather::{WeatherSystem, WeatherPreset, ClimatePattern};
pub use ember::Ember;
pub use pyrocumulonimbus::{PyroCb, PyroCbSystem, LightningStrike, Downdraft};
pub use simulation::FireSimulation;

// Re-export ultra-realistic types
pub use grid::{TerrainData, SimulationGrid, GridCell};
pub use physics::{SuppressionAgent, SuppressionDroplet, AircraftDrop, GroundSuppression};
