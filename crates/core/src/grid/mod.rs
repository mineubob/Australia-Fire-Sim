//! Grid-based simulation modules

pub mod element_grid_coupling;
pub mod simulation_grid;
pub mod terrain;

// Re-export only public types (not internal functions)
pub use simulation_grid::{GridCell, SimulationGrid};
pub use terrain::{TerrainCache, TerrainData};
