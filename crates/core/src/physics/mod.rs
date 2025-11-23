//! Physics modules for ultra-realistic fire simulation

pub mod combustion_physics;
pub mod element_heat_transfer;
pub mod rothermel;
pub mod suppression_physics;

// Re-export public functions and types
pub use rothermel::{
    calculate_spread_rate_with_environment, rothermel_spread_rate,
};
pub use suppression_physics::{apply_suppression_direct, SuppressionAgent};
