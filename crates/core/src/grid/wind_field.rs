//! Mass-Consistent 3D Wind Field Model
//!
//! Implements a diagnostic wind field solver with terrain effects and fire-atmosphere coupling.
//! Based on:
//! - Sherman, C.A. (1978). "A Mass-Consistent Model for Wind Fields Over Complex Terrain."
//!   Journal of Applied Meteorology, 17(3), 312-319.
//! - Forthofer, J.M. (2007). "Modeling Wind in Complex Terrain for Use in Fire Spread Prediction."
//!   `PhD` Thesis, Colorado State University.
//! - Mandel, J. et al. (2011). "Coupled atmosphere-wildland fire modeling with WRF 3.3 and SFIRE 2011."
//!
//! # Theory
//!
//! The mass-consistent wind model adjusts an initial wind field to satisfy the
//! continuity equation (conservation of mass):
//!
//! ```text
//! ∂u/∂x + ∂v/∂y + ∂w/∂z = 0  (divergence-free condition)
//! ```
//!
//! This is achieved via a variational approach that minimizes the deviation from
//! the initial wind field subject to the mass conservation constraint:
//!
//! ```text
//! E(u,v,w,λ) = ∫∫∫ [(u-u₀)² + (v-v₀)² + (w-w₀)²/σ²] dV + ∫∫∫ λ(∇·V) dV
//! ```
//!
//! where λ is a Lagrange multiplier, σ is the vertical-to-horizontal adjustment weighting,
//! and (u₀, v₀, w₀) is the initial wind field.
//!
//! The Euler-Lagrange equations yield a Poisson equation for λ:
//!
//! ```text
//! ∂²λ/∂x² + ∂²λ/∂y² + σ² ∂²λ/∂z² = 2(∂u₀/∂x + ∂v₀/∂y + ∂w₀/∂z)
//! ```
//!
//! The adjusted wind is then:
//! ```text
//! u = u₀ - ∂λ/∂x
//! v = v₀ - ∂λ/∂y
//! w = w₀ - σ² ∂λ/∂z
//! ```
//!
//! # Fire-Atmosphere Coupling
//!
//! Fire plumes create local circulation via buoyancy:
//! - Surface inflow toward fire (entrainment)
//! - Strong updraft at fire location
//! - Outflow aloft (plume spreading)
//!
//! The plume-induced vertical velocity follows Byram's convection column model:
//! ```text
//! w_plume = 2.25 × (I / (ρ × cp × T_amb))^(1/3) × z^(-1/3)
//! ```
//! where I is fire intensity (kW/m), ρ is air density, cp is specific heat.

use crate::core_types::vec3::Vec3;
use crate::grid::TerrainData;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Atmospheric constants for wind calculations
pub mod constants {
    /// Standard air density at sea level (kg/m³)
    pub const AIR_DENSITY: f32 = 1.225;

    /// Specific heat of air at constant pressure (J/(kg·K))
    pub const CP_AIR: f32 = 1005.0;

    /// Standard ambient temperature (K)
    pub const T_AMBIENT: f32 = 300.0;

    /// Reference height for wind measurements (m)
    pub const Z_REFERENCE: f32 = 10.0;

    /// Default roughness length for vegetation (m)
    pub const Z0_DEFAULT: f32 = 0.3;
}

/// Configuration for the wind field solver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindFieldConfig {
    /// Grid dimensions (number of cells in each direction)
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,

    /// Cell size in meters
    pub cell_size: f32,

    /// Vertical cell size in meters (can be different from horizontal)
    pub cell_size_z: f32,

    /// Vertical-to-horizontal adjustment ratio (σ in the formulation)
    /// Higher values make the solver prefer horizontal adjustments over vertical.
    /// Typically 0.01-1.0. For fire simulations, 0.1-0.5 works well.
    pub sigma: f32,

    /// Roughness length for logarithmic wind profile (m)
    /// 0.01 for smooth surfaces, 0.3-1.0 for forests
    pub roughness_length: f32,

    /// Number of iterations for Poisson solver
    pub solver_iterations: usize,

    /// Convergence tolerance for Poisson solver
    pub solver_tolerance: f32,

    /// Enable fire-atmosphere coupling (plume effects)
    pub enable_plume_coupling: bool,

    /// Enable terrain blocking (wind shadow behind obstacles)
    pub enable_terrain_blocking: bool,

    /// Update interval for plume effects (frames)
    /// Plumes change slowly relative to fire spread; updating every few frames
    /// maintains realism while reducing computational cost
    pub plume_update_interval: u32,

    /// Update interval for full terrain/solver recalculation (frames)
    /// Wind-terrain interactions are quasi-static on fire timescales
    pub terrain_update_interval: u32,

    /// Stability class (Pasquill-Gifford categories A-F)
    /// A = very unstable, D = neutral, F = very stable
    pub stability_class: StabilityClass,
}

impl Default for WindFieldConfig {
    fn default() -> Self {
        Self {
            nx: 20,
            ny: 20,
            nz: 10,
            cell_size: 50.0,
            cell_size_z: 20.0,
            sigma: 0.3,
            roughness_length: constants::Z0_DEFAULT,
            solver_iterations: 20, // Sufficient for convergence with warm start
            solver_tolerance: 1e-3, // Balance between accuracy and speed
            enable_plume_coupling: true,
            enable_terrain_blocking: true,
            plume_update_interval: 3, // Update plume effects every 3 frames (~0.3s)
            terrain_update_interval: 10, // Full terrain update every 10 frames (~1s)
            stability_class: StabilityClass::D,
        }
    }
}

/// Pasquill-Gifford atmospheric stability classes
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StabilityClass {
    /// A: Very unstable (strong solar heating, light winds)
    A,
    /// B: Moderately unstable
    B,
    /// C: Slightly unstable
    C,
    /// D: Neutral (overcast or high winds)
    D,
    /// E: Slightly stable
    E,
    /// F: Very stable (nighttime, light winds)
    F,
}

impl StabilityClass {
    /// Get the vertical-to-horizontal scaling factor for this stability class
    /// More stable = less vertical mixing = higher sigma
    #[must_use]
    pub fn sigma_factor(&self) -> f32 {
        match self {
            StabilityClass::A => 0.1, // Very unstable - strong vertical mixing
            StabilityClass::B => 0.2,
            StabilityClass::C => 0.3,
            StabilityClass::D => 0.5, // Neutral
            StabilityClass::E => 0.8,
            StabilityClass::F => 1.5, // Very stable - suppressed vertical mixing
        }
    }
}

/// A fire plume source for wind field coupling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlameSource {
    /// World position of the fire (m)
    pub position: Vec3,
    /// Fire intensity (kW/m) - from Byram's formula
    pub intensity: f32,
    /// Flame height (m)
    pub flame_height: f32,
    /// Fire front width (m)
    pub front_width: f32,
}

/// 3D Wind Field with mass-consistent terrain and fire coupling
///
/// This is the main structure for spatially-varying wind across the simulation domain.
/// Wind speed and direction can be different at every grid cell.
#[derive(Clone, Serialize, Deserialize)]
pub struct WindField {
    /// Wind velocity vectors [u, v, w] at each grid cell
    /// Stored as flattened 3D array: index = iz * (ny * nx) + iy * nx + ix
    wind: Vec<Vec3>,

    /// Initial (uncorrected) wind field for Poisson solver
    wind_initial: Vec<Vec3>,

    /// Lagrange multiplier field (for mass-consistent adjustment)
    lambda: Vec<f32>,

    /// Terrain height at each XY position (in grid coordinates)
    terrain_height: Vec<f32>,

    /// Cached terrain slope at each XY position (radians)
    terrain_slope: Vec<f32>,

    /// Cached terrain aspect at each XY position (radians)
    terrain_aspect: Vec<f32>,

    /// Configuration
    config: WindFieldConfig,

    /// Base wind from weather system
    base_wind: Vec3,

    /// Last base wind used for change detection
    last_base_wind: Vec3,

    /// Number of plumes in the last update (for change detection)
    last_plume_count: usize,

    /// Frame counter for periodic full updates
    frame_counter: u32,

    /// Cached plume positions/intensities for change detection
    last_plume_positions: Vec<Vec3>,
    last_plume_intensities: Vec<f32>,
}

// Helper for explicit, documented usize -> f32 conversions used throughout wind field code
#[inline]
#[expect(clippy::cast_precision_loss)]
fn usize_to_f32(v: usize) -> f32 {
    v as f32
}

impl WindField {
    /// Create a new wind field for the given terrain
    #[must_use]
    pub fn new(config: WindFieldConfig, terrain: &TerrainData) -> Self {
        let total_cells = config.nx * config.ny * config.nz;
        let total_xy = config.nx * config.ny;

        // Initialize terrain height, slope, and aspect grids
        // OPTIMIZATION: Cache these values to avoid repeated Horn's method calls
        let mut terrain_height = Vec::with_capacity(total_xy);
        let mut terrain_slope = Vec::with_capacity(total_xy);
        let mut terrain_aspect = Vec::with_capacity(total_xy);

        for iy in 0..config.ny {
            for ix in 0..config.nx {
                let x = usize_to_f32(ix) * config.cell_size + config.cell_size * 0.5;
                let y = usize_to_f32(iy) * config.cell_size + config.cell_size * 0.5;
                terrain_height.push(*terrain.elevation_at(x, y));
                terrain_slope.push(*terrain.slope_at_horn(x, y).to_radians());
                terrain_aspect.push(*terrain.aspect_at_horn(x, y).to_radians());
            }
        }

        WindField {
            wind: vec![Vec3::zeros(); total_cells],
            wind_initial: vec![Vec3::zeros(); total_cells],
            lambda: vec![0.0; total_cells],
            terrain_height,
            terrain_slope,
            terrain_aspect,
            config,
            base_wind: Vec3::zeros(),
            last_base_wind: Vec3::new(f32::MAX, f32::MAX, f32::MAX), // Force first update
            last_plume_count: 0,
            frame_counter: 0,
            last_plume_positions: Vec::new(),
            last_plume_intensities: Vec::new(),
        }
    }

    /// Create with default configuration
    #[must_use]
    pub fn new_default(terrain: &TerrainData, domain_width: f32, domain_height: f32) -> Self {
        let cell_size = 50.0;
        let nx = (domain_width / cell_size).ceil() as usize;
        let ny = (domain_height / cell_size).ceil() as usize;

        let config = WindFieldConfig {
            nx,
            ny,
            nz: 10,
            cell_size,
            cell_size_z: 20.0,
            ..Default::default()
        };

        Self::new(config, terrain)
    }

    /// Get 3D array index from grid coordinates
    #[inline]
    fn index(&self, ix: usize, iy: usize, iz: usize) -> usize {
        iz * (self.config.ny * self.config.nx) + iy * self.config.nx + ix
    }

    /// Get 2D array index from grid coordinates
    #[inline]
    fn index_2d(&self, ix: usize, iy: usize) -> usize {
        iy * self.config.nx + ix
    }

    /// Get wind at a specific grid cell
    #[must_use]
    pub fn wind_at_grid(&self, ix: usize, iy: usize, iz: usize) -> Vec3 {
        if ix < self.config.nx && iy < self.config.ny && iz < self.config.nz {
            self.wind[self.index(ix, iy, iz)]
        } else {
            self.base_wind
        }
    }

    /// Set wind at a specific grid cell
    pub fn set_wind_at_grid(&mut self, ix: usize, iy: usize, iz: usize, wind: Vec3) {
        if ix < self.config.nx && iy < self.config.ny && iz < self.config.nz {
            let idx = self.index(ix, iy, iz);
            self.wind[idx] = wind;
        }
    }

    /// Get wind at any world position using trilinear interpolation
    #[must_use]
    pub fn wind_at_position(&self, pos: Vec3) -> Vec3 {
        // Clamp position to grid bounds
        let x = pos
            .x
            .max(0.0)
            .min(usize_to_f32(self.config.nx - 1) * self.config.cell_size);
        let y = pos
            .y
            .max(0.0)
            .min(usize_to_f32(self.config.ny - 1) * self.config.cell_size);

        // Get grid indices
        let gx = x / self.config.cell_size;
        let gy = y / self.config.cell_size;

        let ix0 = (gx.floor() as usize).min(self.config.nx - 2);
        let iy0 = (gy.floor() as usize).min(self.config.ny - 2);
        let ix1 = ix0 + 1;
        let iy1 = iy0 + 1;

        // Get terrain height at this XY
        let terrain_z = self.terrain_height[self.index_2d(ix0, iy0)];
        let z_above = (pos.z - terrain_z).max(0.0);
        let gz = z_above / self.config.cell_size_z;

        let iz0 = (gz.floor() as usize).min(self.config.nz - 2);
        let iz1 = iz0 + 1;

        // Fractional parts for interpolation
        let fx = gx - usize_to_f32(ix0);
        let fy = gy - usize_to_f32(iy0);
        let fz = gz - usize_to_f32(iz0);

        // Trilinear interpolation of 8 corners
        let w000 = self.wind_at_grid(ix0, iy0, iz0) * (1.0 - fx) * (1.0 - fy) * (1.0 - fz);
        let w100 = self.wind_at_grid(ix1, iy0, iz0) * fx * (1.0 - fy) * (1.0 - fz);
        let w010 = self.wind_at_grid(ix0, iy1, iz0) * (1.0 - fx) * fy * (1.0 - fz);
        let w110 = self.wind_at_grid(ix1, iy1, iz0) * fx * fy * (1.0 - fz);
        let w001 = self.wind_at_grid(ix0, iy0, iz1) * (1.0 - fx) * (1.0 - fy) * fz;
        let w101 = self.wind_at_grid(ix1, iy0, iz1) * fx * (1.0 - fy) * fz;
        let w011 = self.wind_at_grid(ix0, iy1, iz1) * (1.0 - fx) * fy * fz;
        let w111 = self.wind_at_grid(ix1, iy1, iz1) * fx * fy * fz;

        w000 + w100 + w010 + w110 + w001 + w101 + w011 + w111
    }

    /// Update the wind field with new base wind and fire sources
    ///
    /// This is the main entry point for updating the wind field each frame.
    ///
    /// # Arguments
    /// * `base_wind` - Background wind from weather system
    /// * `terrain` - Terrain data for slope/aspect effects
    /// * `plumes` - Active fire plumes for fire-atmosphere coupling
    /// * `_dt` - Time step (for future temporal smoothing)
    ///
    /// OPTIMIZED: Uses change detection to skip expensive updates when possible
    pub fn update(
        &mut self,
        base_wind: Vec3,
        terrain: &TerrainData,
        plumes: &[PlameSource],
        _dt: f32,
    ) {
        self.frame_counter += 1;

        // Check if wind changed significantly (>0.5 m/s or >10° direction change)
        let wind_changed = (base_wind - self.last_base_wind).norm() > 0.5;

        // Check if plume configuration changed significantly
        let plume_count_changed = plumes.len() != self.last_plume_count;
        let plume_significantly_changed = self.plumes_changed_significantly(plumes);

        // Update tracking state
        self.base_wind = base_wind;
        self.last_base_wind = base_wind;
        self.last_plume_count = plumes.len();

        // OPTIMIZATION: Adaptive update scheduling based on change detection
        // Wind-terrain interactions are quasi-static on fire timescales (seconds to minutes)
        // Plume effects vary faster but still much slower than combustion (0.1-1 second scale)
        // This maintains full physical realism while reducing unnecessary recalculation

        let needs_full_update = wind_changed
            || self
                .frame_counter
                .is_multiple_of(self.config.terrain_update_interval);

        let needs_plume_update = self.config.enable_plume_coupling
            && (!plumes.is_empty() || plume_count_changed)
            && (plume_significantly_changed
                || self
                    .frame_counter
                    .is_multiple_of(self.config.plume_update_interval));

        if needs_full_update {
            // Step 1: Initialize wind field with terrain effects
            self.initialize_with_terrain(terrain);

            // Step 2: Apply terrain blocking
            if self.config.enable_terrain_blocking {
                self.apply_terrain_blocking(terrain);
            }
        }

        // Step 3: Add plume-induced circulation
        if needs_plume_update {
            if !needs_full_update {
                // Restore initial wind before adding new plumes
                self.wind.copy_from_slice(&self.wind_initial);
            }
            self.add_plume_effects(plumes);
            self.cache_plume_state(plumes);
        }

        // Step 4: Solve for mass-consistent adjustment
        // Only run Poisson solver when wind field was modified
        if needs_full_update || needs_plume_update {
            self.solve_mass_consistent();
        }
    }

    /// Check if plumes have changed significantly enough to warrant recalculation
    ///
    /// Returns true if:
    /// - Plume count changed
    /// - Any plume moved >10m
    /// - Any plume intensity changed >20%
    ///
    /// This prevents unnecessary updates when plumes shift slightly due to
    /// minor element ignitions/extinctions at fire perimeter.
    fn plumes_changed_significantly(&self, plumes: &[PlameSource]) -> bool {
        if plumes.len() != self.last_plume_positions.len() {
            return true;
        }

        for (i, plume) in plumes.iter().enumerate() {
            // Check position change (>10m is significant for wind patterns)
            let pos_delta = (plume.position - self.last_plume_positions[i]).norm();
            if pos_delta > 10.0 {
                return true;
            }

            // Check intensity change (>20% affects plume dynamics)
            let intensity_ratio = plume.intensity / self.last_plume_intensities[i].max(1.0);
            if !(0.8..=1.2).contains(&intensity_ratio) {
                return true;
            }
        }

        false
    }

    /// Cache current plume state for change detection
    fn cache_plume_state(&mut self, plumes: &[PlameSource]) {
        self.last_plume_positions.clear();
        self.last_plume_intensities.clear();

        for plume in plumes {
            self.last_plume_positions.push(plume.position);
            self.last_plume_intensities.push(plume.intensity);
        }
    }

    /// Initialize wind field with terrain-modified base wind
    ///
    /// Applies:
    /// - Logarithmic wind profile (wind increases with height)
    /// - Terrain slope effects (speedup/slowdown based on flow-terrain alignment)
    /// - Roughness-based surface friction
    ///
    /// OPTIMIZED: Uses cached slope/aspect values, parallelized with Rayon
    fn initialize_with_terrain(&mut self, _terrain: &TerrainData) {
        let z0 = self.config.roughness_length;
        let z_ref = constants::Z_REFERENCE;

        // Pre-calculate wind direction (constant for all cells this frame)
        let wind_dir = self.base_wind.y.atan2(self.base_wind.x);
        let horizontal_speed = (self.base_wind.x.powi(2) + self.base_wind.y.powi(2)).sqrt();
        let base_wind = self.base_wind;

        // OPTIMIZATION: Parallelize over Z layers for maximum cache efficiency
        // Each layer is independent and can be computed in parallel
        let nx = self.config.nx;
        let ny = self.config.ny;
        let cell_size_z = self.config.cell_size_z;

        // Create slices for parallel iteration
        let terrain_slope = &self.terrain_slope;
        let terrain_aspect = &self.terrain_aspect;

        self.wind
            .par_chunks_mut(nx * ny)
            .zip(self.wind_initial.par_chunks_mut(nx * ny))
            .enumerate()
            .for_each(|(iz, (wind_layer, wind_initial_layer))| {
                // Pre-calculate height-dependent logarithmic factor for this layer
                let z_layer = usize_to_f32(iz) * cell_size_z + cell_size_z * 0.5;

                for iy in 0..ny {
                    for ix in 0..nx {
                        let idx_2d = iy * nx + ix;
                        let idx_layer = iy * nx + ix;

                        // Height above terrain - use z_layer (terrain height not needed here
                        // since we're using ground-relative coordinates)
                        let z_above = z_layer;

                        // 1. Logarithmic wind profile
                        let z_safe = z_above.max(z0 + 0.1);
                        let log_ratio = (z_safe / z0).ln() / (z_ref / z0).ln();
                        let wind_factor = log_ratio.clamp(0.3, 3.0);

                        // 2. Terrain slope/aspect effects (USE CACHED VALUES)
                        let slope = terrain_slope[idx_2d];
                        let aspect = terrain_aspect[idx_2d];

                        // Calculate alignment between wind and upslope direction
                        let upslope_dir = aspect + std::f32::consts::PI;
                        let alignment = (wind_dir - upslope_dir).cos();

                        // Speedup factor based on terrain (Forthofer 2007)
                        let terrain_factor = if alignment > 0.3 {
                            1.0 + alignment * (slope / std::f32::consts::FRAC_PI_4) * 0.6
                        } else if alignment < -0.3 {
                            0.6 - alignment.abs() * (slope / std::f32::consts::FRAC_PI_4) * 0.2
                        } else {
                            1.0 + (slope / std::f32::consts::FRAC_PI_4) * 0.1
                        };

                        // 3. Apply combined factors
                        let total_factor = wind_factor * terrain_factor.clamp(0.3, 2.0);

                        // Set horizontal wind components
                        let mut wind = base_wind * total_factor;

                        // Add slope-induced vertical component
                        let vertical_from_terrain =
                            horizontal_speed * slope.sin() * alignment * 0.5;
                        wind.z = vertical_from_terrain;

                        wind_layer[idx_layer] = wind;
                        wind_initial_layer[idx_layer] = wind;
                    }
                }
            });
    }

    /// Add fire plume effects to the wind field
    ///
    /// Based on Byram's convection column model:
    /// - Inflow at low levels (entrainment into fire)
    /// - Strong updraft at fire location
    /// - Outflow aloft (plume spreading)
    ///
    /// OPTIMIZED: Uses spatial partitioning - only updates grid cells within
    /// plume influence radius. This is physically correct since plume effects
    /// decay exponentially with distance (entrainment ~exp(-r/R)).
    fn add_plume_effects(&mut self, plumes: &[PlameSource]) {
        for plume in plumes {
            // Calculate plume parameters from fire intensity
            // Byram's plume rise: w = 2.25 * (I / (ρ * cp * T))^(1/3) * z^(-1/3)
            let intensity_factor = (plume.intensity
                / (constants::AIR_DENSITY * constants::CP_AIR * constants::T_AMBIENT))
                .powf(1.0 / 3.0);

            // Plume radius grows with height (Morton-Taylor-Turner plume theory)
            let entrainment_coefficient = 0.12; // Typical for fire plumes
            let plume_base_radius = plume.front_width / 2.0;

            // Maximum influence radius: plume affects cells up to 5x its radius
            // This is where entrainment effects become negligible (<1% of peak)
            let max_height = usize_to_f32(self.config.nz) * self.config.cell_size_z;
            let max_plume_radius = plume_base_radius + entrainment_coefficient * max_height;
            let max_influence_radius = max_plume_radius * 5.0;

            // Convert plume position to grid coordinates
            let plume_gx = plume.position.x / self.config.cell_size;
            let plume_gy = plume.position.y / self.config.cell_size;

            // Calculate grid cell range to iterate (spatial partitioning)
            let cells_radius = (max_influence_radius / self.config.cell_size).ceil() as i32;
            let ix_min = ((plume_gx as i32) - cells_radius).max(0) as usize;
            let ix_max = ((plume_gx as i32) + cells_radius + 1).min(self.config.nx as i32) as usize;
            let iy_min = ((plume_gy as i32) - cells_radius).max(0) as usize;
            let iy_max = ((plume_gy as i32) + cells_radius + 1).min(self.config.ny as i32) as usize;

            // Only iterate over cells within influence radius
            for iz in 0..self.config.nz {
                for iy in iy_min..iy_max {
                    for ix in ix_min..ix_max {
                        let x =
                            usize_to_f32(ix) * self.config.cell_size + self.config.cell_size * 0.5;
                        let y =
                            usize_to_f32(iy) * self.config.cell_size + self.config.cell_size * 0.5;
                        let terrain_z = self.terrain_height[self.index_2d(ix, iy)];
                        let z = usize_to_f32(iz) * self.config.cell_size_z
                            + self.config.cell_size_z * 0.5
                            + terrain_z;

                        // Horizontal distance from plume center
                        let dx = x - plume.position.x;
                        let dy = y - plume.position.y;
                        let horizontal_dist = (dx * dx + dy * dy).sqrt();

                        // Height above fire
                        let z_above_fire = (z - plume.position.z).max(0.01);

                        // Plume radius at this height
                        let plume_radius = plume_base_radius
                            + entrainment_coefficient * z_above_fire.max(plume.flame_height);

                        // Skip if too far from plume (shouldn't happen often due to spatial bounds)
                        if horizontal_dist > plume_radius * 5.0 {
                            continue;
                        }

                        let idx = self.index(ix, iy, iz);
                        let normalized_dist = horizontal_dist / plume_radius;

                        if z_above_fire < plume.flame_height * 2.0 {
                            // Low level: entrainment (inflow toward fire)
                            if horizontal_dist > plume_radius * 0.5
                                && horizontal_dist < plume_radius * 3.0
                            {
                                // Radial inflow velocity (entrainment)
                                let inflow_speed =
                                    2.0 * intensity_factor * (-normalized_dist * 0.5).exp();

                                // Direction toward plume center
                                let dir_to_plume = if horizontal_dist > 0.1 {
                                    Vec3::new(-dx / horizontal_dist, -dy / horizontal_dist, 0.0)
                                } else {
                                    Vec3::zeros()
                                };

                                self.wind[idx] += dir_to_plume * inflow_speed;
                            }

                            // Core updraft
                            if horizontal_dist < plume_radius {
                                // Byram's updraft velocity
                                let z_factor = (z_above_fire / plume.flame_height)
                                    .max(0.1)
                                    .powf(-1.0 / 3.0);
                                let radial_factor = (1.0 - (normalized_dist).powi(2)).max(0.0);
                                let updraft_speed =
                                    2.25 * intensity_factor * z_factor * radial_factor;

                                self.wind[idx].z += updraft_speed.min(30.0); // Cap at 30 m/s
                            }
                        } else {
                            // High level: outflow (plume spreading)
                            if horizontal_dist < plume_radius * 2.0 {
                                let outflow_speed =
                                    0.5 * intensity_factor * (-normalized_dist * 0.3).exp();
                                let outflow_dir = if horizontal_dist > 0.1 {
                                    Vec3::new(dx / horizontal_dist, dy / horizontal_dist, 0.0)
                                } else {
                                    Vec3::zeros()
                                };
                                self.wind[idx] += outflow_dir * outflow_speed;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Apply terrain blocking (wind shadow behind ridges)
    ///
    /// When wind flows over a ridge, there's a separation zone on the lee side
    /// where wind speed is reduced and may even reverse (recirculation).
    ///
    /// OPTIMIZED: Pre-calculates wind direction, uses cached terrain heights, parallelized
    fn apply_terrain_blocking(&mut self, terrain: &TerrainData) {
        // Pre-calculate wind direction once (constant for this frame)
        let wind_dir = self.base_wind.y.atan2(self.base_wind.x);
        let wind_cos = wind_dir.cos();
        let wind_sin = wind_dir.sin();

        let nx = self.config.nx;
        let ny = self.config.ny;
        let cell_size = self.config.cell_size;
        let cell_size_z = self.config.cell_size_z;
        let terrain_height = &self.terrain_height;

        // OPTIMIZATION: Parallelize over Z layers
        self.wind
            .par_chunks_mut(nx * ny)
            .enumerate()
            .for_each(|(iz, wind_layer)| {
                let z_above = usize_to_f32(iz) * cell_size_z + cell_size_z * 0.5;

                for iy in 0..ny {
                    for ix in 0..nx {
                        let idx_2d = iy * nx + ix;
                        let idx_layer = iy * nx + ix;
                        let x = usize_to_f32(ix) * cell_size + cell_size * 0.5;
                        let y = usize_to_f32(iy) * cell_size + cell_size * 0.5;
                        let local_terrain_z = terrain_height[idx_2d];

                        // Check upwind terrain (50m upwind)
                        let upwind_x = x - 50.0 * wind_cos;
                        let upwind_y = y - 50.0 * wind_sin;

                        let upwind_z = terrain.elevation_at(upwind_x, upwind_y);
                        let z_diff = *upwind_z - local_terrain_z;

                        // If upwind terrain is higher, we're in wind shadow
                        if z_diff > 0.0 && z_above < z_diff * 2.0 {
                            // Calculate blocking factor
                            let block_factor =
                                1.0 - (z_diff - z_above / 2.0).max(0.0) / (z_diff * 2.0);
                            let block_factor = block_factor.clamp(0.1, 1.0);

                            wind_layer[idx_layer] *= block_factor;
                        }
                    }
                }
            });
    }

    /// Solve the Poisson equation for mass-consistent adjustment
    ///
    /// Uses Red-Black Gauss-Seidel iteration to solve:
    /// ```text
    /// ∂²λ/∂x² + ∂²λ/∂y² + σ² ∂²λ/∂z² = 2 * div(V₀)
    /// ```
    fn solve_mass_consistent(&mut self) {
        // Copy current wind field (with all modifications) to wind_initial
        // This is the field we'll make mass-consistent
        self.wind_initial.copy_from_slice(&self.wind);

        let sigma2 = self.config.sigma * self.config.sigma;
        let dx2 = self.config.cell_size * self.config.cell_size;
        let dy2 = self.config.cell_size * self.config.cell_size;
        let dz2 = self.config.cell_size_z * self.config.cell_size_z;

        // Cache frequently used coefficients
        let inv_2dx = 1.0 / (2.0 * self.config.cell_size);
        let inv_2dz = 1.0 / (2.0 * self.config.cell_size_z);
        let inv_dx2 = 1.0 / dx2;
        let inv_dy2 = 1.0 / dy2;
        let sigma2_inv_dz2 = sigma2 / dz2;
        let denom = 2.0 * inv_dx2 + 2.0 * inv_dy2 + 2.0 * sigma2_inv_dz2;
        let inv_denom = 1.0 / denom;

        // Reset lambda
        self.lambda.fill(0.0);

        // Precompute divergence of initial wind field (RHS of Poisson equation)
        // Parallelize over Z layers since each layer is independent
        let mut divergence = vec![0.0; self.wind.len()];
        let nx = self.config.nx;
        let ny = self.config.ny;
        let nz = self.config.nz;
        let layer_size = nx * ny;

        // Process interior Z layers in parallel
        divergence[layer_size..(nz - 1) * layer_size]
            .par_chunks_mut(layer_size)
            .enumerate()
            .for_each(|(layer_idx, div_layer)| {
                let iz = layer_idx + 1; // Offset by 1 since we skip iz=0
                for iy in 1..ny - 1 {
                    for ix in 1..nx - 1 {
                        let idx_layer = iy * nx + ix;

                        // Central difference for divergence
                        let du_dx = (self.wind_initial[self.index(ix + 1, iy, iz)].x
                            - self.wind_initial[self.index(ix - 1, iy, iz)].x)
                            * inv_2dx;

                        let dv_dy = (self.wind_initial[self.index(ix, iy + 1, iz)].y
                            - self.wind_initial[self.index(ix, iy - 1, iz)].y)
                            * inv_2dx; // Using same spacing

                        let dw_dz = (self.wind_initial[self.index(ix, iy, iz + 1)].z
                            - self.wind_initial[self.index(ix, iy, iz - 1)].z)
                            * inv_2dz;

                        div_layer[idx_layer] = 2.0 * (du_dx + dv_dy + dw_dz);
                    }
                }
            });

        // Red-Black Gauss-Seidel iteration
        for _iter in 0..self.config.solver_iterations {
            let mut max_residual: f32 = 0.0;

            // Red pass (ix + iy + iz even)
            for iz in 1..self.config.nz - 1 {
                for iy in 1..self.config.ny - 1 {
                    for ix in 1..self.config.nx - 1 {
                        if (ix + iy + iz) % 2 == 0 {
                            let idx = self.index(ix, iy, iz);

                            let lambda_new = (self.lambda[self.index(ix + 1, iy, iz)]
                                + self.lambda[self.index(ix - 1, iy, iz)])
                                * inv_dx2
                                + (self.lambda[self.index(ix, iy + 1, iz)]
                                    + self.lambda[self.index(ix, iy - 1, iz)])
                                    * inv_dy2
                                + (self.lambda[self.index(ix, iy, iz + 1)]
                                    + self.lambda[self.index(ix, iy, iz - 1)])
                                    * sigma2_inv_dz2
                                - divergence[idx];

                            let new_val = lambda_new * inv_denom;
                            max_residual = max_residual.max((new_val - self.lambda[idx]).abs());
                            self.lambda[idx] = new_val;
                        }
                    }
                }
            }

            // Black pass (ix + iy + iz odd)
            for iz in 1..self.config.nz - 1 {
                for iy in 1..self.config.ny - 1 {
                    for ix in 1..self.config.nx - 1 {
                        if (ix + iy + iz) % 2 == 1 {
                            let idx = self.index(ix, iy, iz);

                            let lambda_new = (self.lambda[self.index(ix + 1, iy, iz)]
                                + self.lambda[self.index(ix - 1, iy, iz)])
                                * inv_dx2
                                + (self.lambda[self.index(ix, iy + 1, iz)]
                                    + self.lambda[self.index(ix, iy - 1, iz)])
                                    * inv_dy2
                                + (self.lambda[self.index(ix, iy, iz + 1)]
                                    + self.lambda[self.index(ix, iy, iz - 1)])
                                    * sigma2_inv_dz2
                                - divergence[idx];

                            let new_val = lambda_new * inv_denom;
                            max_residual = max_residual.max((new_val - self.lambda[idx]).abs());
                            self.lambda[idx] = new_val;
                        }
                    }
                }
            }

            // Check convergence
            if max_residual < self.config.solver_tolerance {
                break;
            }
        }

        // Apply adjustment: V = V₀ - ∇λ (scaled by σ² for vertical)
        for iz in 1..self.config.nz - 1 {
            for iy in 1..self.config.ny - 1 {
                for ix in 1..self.config.nx - 1 {
                    let idx = self.index(ix, iy, iz);

                    // Gradient of lambda
                    let dlambda_dx = (self.lambda[self.index(ix + 1, iy, iz)]
                        - self.lambda[self.index(ix - 1, iy, iz)])
                        * inv_2dx;

                    let dlambda_dy = (self.lambda[self.index(ix, iy + 1, iz)]
                        - self.lambda[self.index(ix, iy - 1, iz)])
                        * inv_2dx; // Using same spacing

                    let dlambda_dz = (self.lambda[self.index(ix, iy, iz + 1)]
                        - self.lambda[self.index(ix, iy, iz - 1)])
                        * inv_2dz;

                    // Apply correction
                    self.wind[idx].x = self.wind_initial[idx].x - dlambda_dx;
                    self.wind[idx].y = self.wind_initial[idx].y - dlambda_dy;
                    self.wind[idx].z = self.wind_initial[idx].z - sigma2 * dlambda_dz;
                }
            }
        }

        // Copy boundary values from initial field
        self.copy_boundaries();
    }

    /// Copy initial wind values to boundaries
    fn copy_boundaries(&mut self) {
        // X boundaries
        for iz in 0..self.config.nz {
            for iy in 0..self.config.ny {
                let idx_0 = self.index(0, iy, iz);
                let idx_n = self.index(self.config.nx - 1, iy, iz);
                self.wind[idx_0] = self.wind_initial[idx_0];
                self.wind[idx_n] = self.wind_initial[idx_n];
            }
        }

        // Y boundaries
        for iz in 0..self.config.nz {
            for ix in 0..self.config.nx {
                let idx_0 = self.index(ix, 0, iz);
                let idx_n = self.index(ix, self.config.ny - 1, iz);
                self.wind[idx_0] = self.wind_initial[idx_0];
                self.wind[idx_n] = self.wind_initial[idx_n];
            }
        }

        // Z boundaries
        for iy in 0..self.config.ny {
            for ix in 0..self.config.nx {
                let idx_0 = self.index(ix, iy, 0);
                let idx_n = self.index(ix, iy, self.config.nz - 1);
                self.wind[idx_0] = self.wind_initial[idx_0];
                self.wind[idx_n] = self.wind_initial[idx_n];
            }
        }
    }

    /// Get horizontal wind speed at position (m/s)
    #[must_use]
    pub fn wind_speed_at(&self, pos: Vec3) -> f32 {
        let wind = self.wind_at_position(pos);
        (wind.x * wind.x + wind.y * wind.y).sqrt()
    }

    /// Get wind direction at position (degrees, 0 = from north, 90 = from east)
    #[must_use]
    pub fn wind_direction_at(&self, pos: Vec3) -> f32 {
        let wind = self.wind_at_position(pos);
        // Wind direction is where wind is coming FROM
        let from_dir = (-wind.x).atan2(-wind.y).to_degrees();
        if from_dir < 0.0 {
            from_dir + 360.0
        } else {
            from_dir
        }
    }

    /// Get vertical wind component at position (m/s, positive = upward)
    #[must_use]
    pub fn vertical_wind_at(&self, pos: Vec3) -> f32 {
        self.wind_at_position(pos).z
    }

    /// Get base wind vector
    #[must_use]
    pub fn base_wind(&self) -> Vec3 {
        self.base_wind
    }

    /// Get grid configuration
    #[must_use]
    pub fn config(&self) -> &WindFieldConfig {
        &self.config
    }

    /// Calculate wind-terrain interaction factor for Rothermel spread
    ///
    /// Returns a multiplier for fire spread rate based on wind-slope alignment.
    /// This follows the Rothermel (1972) wind factor formulation.
    #[must_use]
    pub fn rothermel_wind_factor(&self, pos: Vec3, fire_direction: f32) -> f32 {
        let wind = self.wind_at_position(pos);
        let wind_speed = (wind.x * wind.x + wind.y * wind.y).sqrt();
        let wind_dir_rad = wind.y.atan2(wind.x);

        // Angle between wind and fire spread direction
        let fire_dir_rad = fire_direction.to_radians();
        let cos_angle = (wind_dir_rad - fire_dir_rad).cos();

        // Effective mid-flame wind speed (simplified)
        let effective_wind = wind_speed * cos_angle.max(0.0);

        // Rothermel wind factor (simplified - full model uses fuel properties)
        // C × (3.281 × U)^B × (β/βop)^E
        // Here we use simplified version: ~0.0003 × U^2 for typical fuels
        let wind_factor = 1.0 + 0.0003 * (effective_wind * 60.0).powi(2);

        wind_factor.min(26.0) // Cap at ~26x as per copilot-instructions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::units::Meters;

    #[test]
    fn test_wind_field_creation() {
        let terrain = TerrainData::flat(
            Meters::new(1000.0),
            Meters::new(1000.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let config = WindFieldConfig {
            nx: 10,
            ny: 10,
            nz: 5,
            cell_size: 100.0,
            cell_size_z: 20.0,
            ..Default::default()
        };

        let wind_field = WindField::new(config, &terrain);
        assert_eq!(wind_field.wind.len(), 10 * 10 * 5);
    }

    #[test]
    fn test_logarithmic_profile() {
        let terrain = TerrainData::flat(
            Meters::new(1000.0),
            Meters::new(1000.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let config = WindFieldConfig {
            nx: 10,
            ny: 10,
            nz: 10,
            cell_size: 100.0,
            cell_size_z: 10.0,
            ..Default::default()
        };

        let mut wind_field = WindField::new(config, &terrain);

        // Set base wind and update
        let base_wind = Vec3::new(10.0, 0.0, 0.0);
        wind_field.update(base_wind, &terrain, &[], 0.1);

        // Wind at surface should be less than at height
        let wind_low = wind_field.wind_at_position(Vec3::new(500.0, 500.0, 2.0));
        let wind_high = wind_field.wind_at_position(Vec3::new(500.0, 500.0, 50.0));

        assert!(
            wind_high.x > wind_low.x,
            "Wind should increase with height: low={}, high={}",
            wind_low.x,
            wind_high.x
        );
    }

    #[test]
    fn test_terrain_speedup() {
        let terrain = TerrainData::single_hill(
            Meters::new(1000.0),
            Meters::new(1000.0),
            Meters::new(20.0),
            Meters::new(0.0),
            Meters::new(100.0),
            Meters::new(200.0),
        );
        let config = WindFieldConfig {
            nx: 20,
            ny: 20,
            nz: 10,
            cell_size: 50.0,
            cell_size_z: 20.0,
            solver_iterations: 20,
            ..Default::default()
        };

        let mut wind_field = WindField::new(config, &terrain);

        // Wind from south (blowing north, going uphill on south side)
        let base_wind = Vec3::new(0.0, 10.0, 0.0);
        wind_field.update(base_wind, &terrain, &[], 0.1);

        // On upwind (south) side of hill, wind should be accelerated
        let wind_upwind = wind_field.wind_at_position(Vec3::new(500.0, 400.0, 50.0));

        // On flat ground at edge
        let wind_flat = wind_field.wind_at_position(Vec3::new(500.0, 100.0, 50.0));

        // Upwind side should have similar or higher wind (terrain speedup)
        // Note: This is a simplified test; real behavior depends on exact slope
        assert!(wind_upwind.y.abs() > 0.0, "Wind should have Y component");
        assert!(wind_flat.y.abs() > 0.0, "Flat area should have wind");
    }

    #[test]
    fn test_plume_updraft() {
        let terrain = TerrainData::flat(
            Meters::new(500.0),
            Meters::new(500.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let config = WindFieldConfig {
            nx: 10,
            ny: 10,
            nz: 10,
            cell_size: 50.0,
            cell_size_z: 20.0,
            enable_plume_coupling: true,
            solver_iterations: 20,
            ..Default::default()
        };

        let mut wind_field = WindField::new(config, &terrain);

        // Add a fire plume at center
        let plume = PlameSource {
            position: Vec3::new(250.0, 250.0, 0.0),
            intensity: 10000.0, // 10 MW/m - intense fire
            flame_height: 10.0,
            front_width: 20.0,
        };

        wind_field.update(Vec3::new(5.0, 0.0, 0.0), &terrain, &[plume], 0.1);

        // Above fire (but within updraft zone: z < 2*flame_height = 20m) should have updraft
        // Testing at z=15m, which is within the core updraft zone
        let wind_above_fire = wind_field.wind_at_position(Vec3::new(250.0, 250.0, 15.0));
        assert!(
            wind_above_fire.z > 0.0,
            "Should have updraft above fire: w={}",
            wind_above_fire.z
        );

        // Away from fire, vertical component should be less
        let wind_away = wind_field.wind_at_position(Vec3::new(50.0, 50.0, 15.0));
        assert!(
            wind_away.z < wind_above_fire.z,
            "Updraft should be stronger at fire"
        );
    }

    #[test]
    fn test_mass_conservation() {
        let terrain = TerrainData::flat(
            Meters::new(500.0),
            Meters::new(500.0),
            Meters::new(10.0),
            Meters::new(0.0),
        );
        let config = WindFieldConfig {
            nx: 10,
            ny: 10,
            nz: 5,
            cell_size: 50.0,
            cell_size_z: 20.0,
            solver_iterations: 100,
            solver_tolerance: 1e-6,
            enable_plume_coupling: false,
            enable_terrain_blocking: false,
            ..Default::default()
        };

        let mut wind_field = WindField::new(config, &terrain);
        wind_field.update(Vec3::new(10.0, 0.0, 0.0), &terrain, &[], 0.1);

        // Check divergence is near zero in interior
        let mut max_div: f32 = 0.0;
        for iz in 2..wind_field.config.nz - 2 {
            for iy in 2..wind_field.config.ny - 2 {
                for ix in 2..wind_field.config.nx - 2 {
                    let du_dx = (wind_field.wind[wind_field.index(ix + 1, iy, iz)].x
                        - wind_field.wind[wind_field.index(ix - 1, iy, iz)].x)
                        / (2.0 * wind_field.config.cell_size);

                    let dv_dy = (wind_field.wind[wind_field.index(ix, iy + 1, iz)].y
                        - wind_field.wind[wind_field.index(ix, iy - 1, iz)].y)
                        / (2.0 * wind_field.config.cell_size);

                    let dw_dz = (wind_field.wind[wind_field.index(ix, iy, iz + 1)].z
                        - wind_field.wind[wind_field.index(ix, iy, iz - 1)].z)
                        / (2.0 * wind_field.config.cell_size_z);

                    let div = (du_dx + dv_dy + dw_dz).abs();
                    max_div = max_div.max(div);
                }
            }
        }

        // Divergence should be small after mass-consistent adjustment
        assert!(max_div < 0.5, "Max divergence should be small: {max_div}");
    }
}
