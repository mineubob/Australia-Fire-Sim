//! Physics modules for ultra-realistic fire simulation

pub mod combustion_physics;
pub mod suppression_physics;

// Re-export main types
pub use combustion_physics::*;
pub use suppression_physics::*;
