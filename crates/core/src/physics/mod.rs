//! Physics modules for ultra-realistic fire simulation

pub(crate) mod albini_spotting;
pub(crate) mod canopy_layers;
pub(crate) mod combustion_physics;
pub(crate) mod crown_fire;
pub(crate) mod element_heat_transfer;
pub(crate) mod fuel_moisture;
pub(crate) mod rothermel;
pub(crate) mod smoldering;
pub mod suppression_physics; // Made pub for FFI access to SuppressionAgent
pub(crate) mod terrain_physics;

// Re-export internal functions and types for crate-internal use only
pub(crate) use albini_spotting::{calculate_ember_trajectory, calculate_lofting_height};
pub(crate) use canopy_layers::{
    calculate_layer_transition_probability, CanopyLayer, CanopyStructure,
};
pub(crate) use crown_fire::{calculate_crown_fire_behavior, CrownFireType};
pub(crate) use fuel_moisture::{calculate_equilibrium_moisture, FuelMoistureState};
pub(crate) use smoldering::update_smoldering_state;
// Re-export smoldering types publicly for integration tests
pub use smoldering::{CombustionPhase, SmolderingState};
pub(crate) use suppression_physics::apply_suppression_direct;
// Re-export SuppressionAgent publicly for FFI
pub use suppression_physics::SuppressionAgent;
pub(crate) use terrain_physics::terrain_spread_multiplier;

// ============================================================================
// PUBLIC RE-EXPORTS FOR VALIDATION TESTING
// ============================================================================
// These functions are exported publicly so integration tests can validate
// the scientific accuracy of the physics models. According to the
// implementation plan, validation tests need access to internal physics
// functions to verify they match peer-reviewed research.

/// Public re-exports of validation test functions from albini_spotting module
pub mod albini_spotting_validation {
    pub use super::albini_spotting::{
        calculate_ember_trajectory, calculate_lofting_height, calculate_maximum_spotting_distance,
    };
}

/// Public re-exports of validation test functions from canopy_layers module
pub mod canopy_layers_validation {
    pub use super::canopy_layers::{
        calculate_layer_transition_probability, CanopyLayer, CanopyStructure,
    };
}

/// Public re-exports of validation test functions from crown_fire module
pub mod crown_fire_validation {
    pub use super::crown_fire::{
        calculate_critical_crown_spread_rate, calculate_critical_surface_intensity,
    };
}

/// Public re-exports of validation test functions from rothermel module
pub mod rothermel_validation {
    pub use super::rothermel::rothermel_spread_rate;
}
