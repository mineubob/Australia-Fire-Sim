//! Field solver trait definition
//!
//! This module defines the `FieldSolver` trait, which provides a backend-agnostic
//! interface for field-based fire simulation. Both CPU and GPU implementations
//! implement this trait.

use std::borrow::Cow;

/// Backend-agnostic interface for field-based fire simulation
///
/// This trait defines the operations that both CPU and GPU field solvers must implement.
/// All field operations work on continuous 2D grids where each cell represents a small
/// area of terrain with temperature, fuel load, moisture, and other properties.
pub trait FieldSolver: Send + Sync {
    /// Advance heat transfer by dt seconds
    ///
    /// Computes Stefan-Boltzmann radiation, thermal diffusion, convection, and wind advection.
    ///
    /// # Arguments
    ///
    /// * `dt` - Timestep in seconds
    /// * `wind_x` - Wind velocity in x direction (m/s)
    /// * `wind_y` - Wind velocity in y direction (m/s)
    /// * `ambient_temp` - Ambient temperature in Kelvin
    fn step_heat_transfer(&mut self, dt: f32, wind_x: f32, wind_y: f32, ambient_temp: f32);

    /// Advance combustion (fuel consumption, heat release)
    ///
    /// Computes fuel consumption rate, heat release, and oxygen depletion.
    ///
    /// # Arguments
    ///
    /// * `dt` - Timestep in seconds
    fn step_combustion(&mut self, dt: f32);

    /// Advance moisture (evaporation, equilibrium)
    ///
    /// Computes moisture evaporation from heat and equilibrium moisture recovery from humidity.
    ///
    /// # Arguments
    ///
    /// * `dt` - Timestep in seconds
    /// * `humidity` - Relative humidity (0.0 to 1.0)
    fn step_moisture(&mut self, dt: f32, humidity: f32);

    /// Advance level set (fire front propagation)
    ///
    /// Evolves the signed distance function φ that tracks the fire front boundary.
    ///
    /// # Arguments
    ///
    /// * `dt` - Timestep in seconds
    fn step_level_set(&mut self, dt: f32);

    /// Sync ignition (T > `T_ign` → update φ)
    ///
    /// Updates the level set to include cells that have reached ignition temperature.
    fn step_ignition_sync(&mut self);

    /// Read temperature field (for visualization/queries)
    ///
    /// Returns the temperature field as a flat array in row-major order.
    /// Values are in Kelvin.
    ///
    /// # Returns
    ///
    /// Temperature field. CPU backend returns borrowed slice, GPU backend returns owned Vec.
    fn read_temperature(&self) -> Cow<'_, [f32]>;

    /// Read level set field (for fire front extraction)
    ///
    /// Returns the signed distance function φ where:
    /// - φ < 0: Inside fire (burning/burned)
    /// - φ > 0: Outside fire (unburned)
    /// - φ = 0: Fire front (perimeter)
    ///
    /// # Returns
    ///
    /// Level set field in row-major order
    fn read_level_set(&self) -> Cow<'_, [f32]>;

    /// Ignite at position with radius
    ///
    /// Sets the level set φ < 0 in a circular region and raises temperature to ignition.
    ///
    /// # Arguments
    ///
    /// * `x` - X position in meters
    /// * `y` - Y position in meters
    /// * `radius` - Ignition radius in meters
    fn ignite_at(&mut self, x: f32, y: f32, radius: f32);

    /// Get grid dimensions
    ///
    /// # Returns
    ///
    /// Tuple of `(width, height, cell_size)` where width and height are in cells,
    /// and `cell_size` is in meters per cell
    fn dimensions(&self) -> (u32, u32, f32);

    /// Check if this is the GPU backend
    ///
    /// # Returns
    ///
    /// `true` if GPU-accelerated, `false` if CPU-only
    fn is_gpu_accelerated(&self) -> bool;
}
