//! Physics modules for ultra-realistic fire simulation

pub mod albini_spotting;
pub mod canopy_layers;
pub mod combustion_physics;
pub mod crown_fire;
pub mod element_heat_transfer;
pub mod fuel_moisture;
pub mod rothermel;
pub mod smoldering;
pub mod suppression_physics;

// Re-export public functions and types
pub use albini_spotting::{calculate_maximum_spotting_distance, calculate_lofting_height};
pub use canopy_layers::{CanopyLayer, CanopyStructure, calculate_layer_transition_probability};
pub use crown_fire::{calculate_crown_fire_behavior, CrownFireBehavior, CrownFireType};
pub use fuel_moisture::{FuelMoistureState, calculate_equilibrium_moisture, update_moisture_timelag};
pub use rothermel::{calculate_spread_rate_with_environment, rothermel_spread_rate};
pub use smoldering::{CombustionPhase, SmolderingState, update_smoldering_state};
pub use suppression_physics::{apply_suppression_direct, SuppressionAgent};
