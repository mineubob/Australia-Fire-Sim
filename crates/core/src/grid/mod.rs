//! Grid-based simulation modules

pub mod fuel_loader;
pub(crate) mod simulation_grid;
pub(crate) mod terrain;
pub mod wind_field;

// Re-export only public types (not internal functions)
pub use simulation_grid::{GridCell, SimulationGrid};
pub use terrain::{TerrainCache, TerrainData};
pub use wind_field::{PlameSource, StabilityClass, WindField, WindFieldConfig};
