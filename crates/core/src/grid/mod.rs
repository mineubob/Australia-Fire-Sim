//! Grid-based simulation modules

pub(crate) mod simulation_grid;
pub(crate) mod terrain;

// Re-export only public types (not internal functions)
pub use simulation_grid::{GridCell, SimulationGrid};
pub use terrain::{TerrainCache, TerrainData};
