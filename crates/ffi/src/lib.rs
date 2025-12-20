/// FFI (Foreign Function Interface) bindings for the bushfire simulation engine.
/// Provides a C-compatible API for integration with game engines (Godot, Unreal)
/// and other C/C++ applications.
///
/// # Module Organization
/// - [`error`] - Error codes and error handling
/// - [`terrain`] - Terrain configuration types
/// - [`instance`] - Main simulation instance and lifecycle management
/// - [`simulation`] - Simulation update functions
/// - [`queries`] - Query functions for simulation state
/// - [`helpers`] - Internal helper functions (not exposed in C API)
mod error;
mod helpers;
mod instance;
mod queries;
mod simulation;
mod terrain;

// Re-export public API types and functions
pub use error::{fire_sim_get_last_error, fire_sim_get_last_error_code, FireSimErrorCode};
pub use instance::{fire_sim_destroy, fire_sim_new, FireSimInstance};
pub use queries::{
    fire_sim_clear_snapshot, fire_sim_get_burning_elements, fire_sim_get_element_stats,
    ElementStats,
};
pub use simulation::fire_sim_update;
pub use terrain::Terrain;
