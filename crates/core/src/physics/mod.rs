//! Physics modules for ultra-realistic fire simulation

pub mod combustion_physics;
pub mod element_heat_transfer;
pub mod suppression_physics;

// Re-export only public types (not internal functions)
pub use suppression_physics::{
    AircraftDrop, GroundSuppression, SuppressionAgent, SuppressionDroplet,
};
