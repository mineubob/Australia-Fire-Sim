//! Grid-based simulation modules

pub mod terrain;
pub mod simulation_grid;
pub mod element_grid_coupling;

// Re-export main types
pub use terrain::*;
pub use simulation_grid::*;
pub use element_grid_coupling::*;
