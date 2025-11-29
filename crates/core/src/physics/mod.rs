//! Physics modules for ultra-realistic fire simulation

pub(crate) mod albini_spotting;
pub(crate) mod canopy_layers;
pub(crate) mod combustion_physics;
pub(crate) mod crown_fire;
pub(crate) mod element_heat_transfer;
pub(crate) mod fuel_moisture;
pub(crate) mod rothermel;
pub(crate) mod smoldering;
pub(crate) mod suppression_physics;
pub(crate) mod terrain_physics;

// Re-export internal functions and types for crate-internal use only
pub(crate) use albini_spotting::{calculate_lofting_height, calculate_maximum_spotting_distance};
pub(crate) use canopy_layers::{
    calculate_layer_transition_probability, CanopyLayer, CanopyStructure,
};
pub(crate) use crown_fire::{calculate_crown_fire_behavior, CrownFireBehavior, CrownFireType};
pub(crate) use fuel_moisture::{
    calculate_equilibrium_moisture, update_moisture_timelag, FuelMoistureState,
};
pub(crate) use rothermel::{calculate_spread_rate_with_environment, rothermel_spread_rate};
pub(crate) use smoldering::{update_smoldering_state, CombustionPhase, SmolderingState};
pub(crate) use suppression_physics::{apply_suppression_direct, SuppressionAgent};
pub(crate) use terrain_physics::{
    aspect_wind_multiplier, slope_spread_multiplier_terrain, terrain_spread_multiplier,
};
