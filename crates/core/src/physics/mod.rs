//! Physics modules for ultra-realistic fire simulation

pub mod combustion_physics;
pub mod element_heat_transfer;
pub mod suppression_physics;

// Re-export main types
pub use combustion_physics::*;
pub use element_heat_transfer::*;
pub use suppression_physics::*;
