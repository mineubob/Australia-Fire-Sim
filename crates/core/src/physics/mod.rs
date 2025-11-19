//! Physics modules for ultra-realistic fire simulation

pub mod combustion_physics;
pub mod suppression_physics;
pub mod element_heat_transfer;

// Re-export main types
pub use combustion_physics::*;
pub use suppression_physics::*;
pub use element_heat_transfer::*;
