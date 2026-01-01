//! Field solver trait definition
//!
//! This module defines the `FieldSolver` trait, which provides a backend-agnostic
//! interface for field-based fire simulation. Both CPU and GPU implementations
//! implement this trait.

use crate::core_types::units::{Kelvin, Meters, Seconds};
use crate::core_types::vec3::Vec3;
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
    /// * `dt` - Timestep
    /// * `wind` - Wind velocity vector in m/s (x, y, z components)
    /// * `ambient_temp` - Ambient temperature
    fn step_heat_transfer(&mut self, dt: Seconds, wind: Vec3, ambient_temp: Kelvin);

    /// Advance combustion (fuel consumption, heat release)
    ///
    /// Computes fuel consumption rate, heat release, and oxygen depletion.
    ///
    /// # Arguments
    ///
    /// * `dt` - Timestep
    fn step_combustion(&mut self, dt: Seconds);

    /// Advance moisture (evaporation, equilibrium)
    ///
    /// Computes moisture evaporation from heat and equilibrium moisture recovery from humidity.
    ///
    /// # Arguments
    ///
    /// * `dt` - Timestep
    /// * `humidity` - Relative humidity (0.0 to 1.0)
    fn step_moisture(&mut self, dt: Seconds, humidity: f32);

    /// Advance level set (fire front propagation)
    ///
    /// Evolves the signed distance function φ that tracks the fire front boundary.
    ///
    /// # Arguments
    ///
    /// * `dt` - Timestep
    fn step_level_set(&mut self, dt: Seconds);

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

    /// Apply heat to a location, allowing natural ignition when temperature threshold is reached.
    ///
    /// This is the PRIMARY method for starting fires realistically. Heat accumulates
    /// in the fuel, and ignition occurs naturally when the fuel reaches its ignition
    /// temperature through the applied thermal energy.
    ///
    /// # Arguments
    ///
    /// * `x` - X position in meters
    /// * `y` - Y position in meters
    /// * `temperature_k` - Target temperature in Kelvin to apply
    /// * `radius_m` - Radius in meters over which to apply heat (Gaussian falloff)
    ///
    /// # Use Cases
    ///
    /// - **Drip torch / backburning**: Apply ~600-800K repeatedly along a line
    /// - **Match ignition**: Single application of ~600K at point (radius ~0.1m)
    /// - **Radiant heating**: Apply 400-500K over larger area
    /// - **Failed ignition**: Heat dissipates if fuel is too wet or wind is too strong
    ///
    /// # Scientific Basis
    ///
    /// Real ignition sources transfer thermal energy to fuel. Ignition is NOT
    /// instantaneous - it requires heat accumulation until the fuel reaches its
    /// ignition temperature while accounting for:
    /// - Moisture evaporation (2260 kJ/kg latent heat)
    /// - Heat losses to convection and radiation
    /// - Wind cooling effects
    ///
    /// This models the preheating phase (Catalog 5.1) realistically.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fire_sim_core::solver::FieldSolver;
    /// # use fire_sim_core::core_types::units::{Kelvin, Meters};
    /// # fn example(solver: &mut dyn FieldSolver) {
    /// // Drip torch operation (continuous heat application)
    /// for i in 0..10 {
    ///     let x = Meters::new(50.0 + i as f32 * 2.0);
    ///     solver.apply_heat(x, Meters::new(100.0), Kelvin::new(873.15), Meters::new(0.5)); // 600°C, 0.5m radius
    /// }
    /// # }
    /// ```
    fn apply_heat(&mut self, x: Meters, y: Meters, temperature_k: Kelvin, radius_m: Meters);

    /// Get grid dimensions
    ///
    /// # Returns
    ///
    /// Tuple of `(width, height, cell_size)` where width and height are in cells,
    /// and `cell_size` is in meters per cell
    fn dimensions(&self) -> (u32, u32, Meters);

    /// Check if this is the GPU backend
    ///
    /// # Returns
    ///
    /// `true` if GPU-accelerated, `false` if CPU-only
    fn is_gpu_accelerated(&self) -> bool;
}
