//! Grid-based simulation modules

pub mod element_grid_coupling;
pub mod simulation_grid;
pub mod terrain;

// Re-export main types
pub use element_grid_coupling::*;
pub use simulation_grid::*;
pub use terrain::*;
