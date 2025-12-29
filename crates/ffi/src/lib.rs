/// FFI (Foreign Function Interface) bindings for the bushfire simulation engine.
/// Provides a C-compatible API for integration with game engines (Godot, Unreal)
/// and other C/C++ applications.
///
/// # Module Organization
/// - [`error`] - Error codes and error handling
/// - [`terrain`] - Terrain configuration types
/// - [`field_simulation`] - Field-based simulation (GPU/CPU system)
/// - [`field_queries`] - Field simulation queries (fire front, grids, stats)
/// - [`helpers`] - Internal helper functions (not exposed in C API)
mod error;
mod field_queries;
mod field_simulation;
mod helpers;
mod terrain;

// Re-export public API types and functions

// Error handling
pub use error::{fire_sim_get_last_error, fire_sim_get_last_error_code, FireSimErrorCode};

// Field-based API (GPU/CPU system)
pub use field_queries::{
    fire_sim_field_get_fire_front, fire_sim_field_get_stats, fire_sim_field_get_temperature_grid,
    fire_sim_field_read_level_set, fire_sim_free_fire_front, fire_sim_free_grid,
    fire_sim_free_stats, FieldSimStats, FireFrontData, FireFrontVertex,
};
pub use field_simulation::{
    fire_sim_field_destroy, fire_sim_field_ignite_at, fire_sim_field_is_gpu_accelerated,
    fire_sim_field_new, fire_sim_field_update, FireSimFieldInstance,
};

// Terrain
pub use terrain::Terrain;
