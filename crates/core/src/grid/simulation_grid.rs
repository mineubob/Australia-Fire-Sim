//! 3D simulation grid with atmospheric properties and adaptive refinement
//!
//! Implements a hybrid grid system where atmospheric properties (temperature, wind,
//! humidity, oxygen, combustion products) are tracked per cell, while discrete fuel
//! elements interact with cells for extreme realism.

use crate::core_types::element::Vec3;
use crate::core_types::units::{Celsius, Fraction, Meters};
use crate::grid::{TerrainCache, TerrainData};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

// Local helper to centralize deliberate integer -> f32 conversions.
// These conversions are deliberate for index→world-coordinate math (small ranges)
// and are annotated so linting tools see clear intent in one place.
#[inline]
#[expect(clippy::cast_precision_loss)]
fn usize_to_f32(v: usize) -> f32 {
    v as f32
}

// Pre-computed neighbor offsets for mark_active_cells (compile-time computation)
// cells_radius=2 (30m / 15m cell_size) = 5×5×5 = 125 offsets
const MARK_ACTIVE_OFFSETS: [(i32, i32, i32); 125] = {
    let mut offsets = [(0, 0, 0); 125];
    let mut idx = 0;
    let mut dz = -2;
    while dz <= 2 {
        let mut dy = -2;
        while dy <= 2 {
            let mut dx = -2;
            while dx <= 2 {
                offsets[idx] = (dx, dy, dz);
                idx += 1;
                dx += 1;
            }
            dy += 1;
        }
        dz += 1;
    }
    offsets
};

/// Atmospheric properties tracked per grid cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCell {
    /// Air temperature (°C)
    pub(crate) temperature: Celsius,
    /// Wind velocity vector (m/s)
    pub(crate) wind: Vec3,
    /// Relative humidity (0-1)
    pub(crate) humidity: f32,
    /// Oxygen concentration (kg/m³)
    pub(crate) oxygen: f32,
    /// CO concentration (kg/m³)
    pub(crate) carbon_monoxide: f32,
    /// CO2 concentration (kg/m³)
    pub(crate) carbon_dioxide: f32,
    /// Smoke/particulate concentration (kg/m³)
    pub(crate) smoke_particles: f32,
    /// Water vapor from combustion (kg/m³)
    pub(crate) water_vapor: f32,
    /// Incident radiation flux (W/m²)
    pub(crate) radiation_flux: f32,
    /// Terrain elevation at cell center (m)
    pub(crate) elevation: f32,
    /// Cell refinement level (0 = base, higher = finer)
    pub(crate) refinement_level: u8,
    /// Is cell active (near fire or important)
    pub(crate) is_active: bool,
    /// Pressure (Pa) - for buoyancy calculations
    pub(crate) pressure: f32,
    /// Suppression agent concentration (kg/m³) - water/retardant/foam
    pub(crate) suppression_agent: f32,
}

impl GridCell {
    /// Create a new cell with atmospheric defaults
    #[must_use]
    pub fn new(elevation: f32) -> Self {
        GridCell {
            temperature: Celsius::new(20.0), // Ambient 20°C
            wind: Vec3::zeros(),
            humidity: 0.4, // 40% RH
            oxygen: 0.273, // Normal air: 21% O2 by volume ≈ 0.273 kg/m³
            carbon_monoxide: 0.0,
            carbon_dioxide: 0.0007, // Ambient CO2: ~400 ppm
            smoke_particles: 0.0,
            water_vapor: 0.008, // ~10 g/m³ at 40% RH
            radiation_flux: 0.0,
            elevation,
            refinement_level: 0,
            is_active: false,
            pressure: 101325.0, // Sea level pressure (Pa)
            suppression_agent: 0.0,
        }
    }

    /// Calculate air density (kg/m³) using ideal gas law
    /// ρ = P / (`R_specific` × `T_kelvin`)
    #[must_use]
    pub fn air_density(&self) -> f32 {
        const R_SPECIFIC_AIR: f32 = 287.05; // J/(kg·K)
        let temp_k = *self.temperature as f32 + 273.15;
        self.pressure / (R_SPECIFIC_AIR * temp_k)
    }

    /// Calculate buoyancy force per unit volume (N/m³)
    /// Based on temperature difference from ambient
    #[must_use]
    pub fn buoyancy_force(&self, ambient_temp: Celsius) -> f32 {
        // Calculate ambient air density from temperature using ideal gas law
        const R_SPECIFIC_AIR: f32 = 287.05; // J/(kg·K)
        let ambient_temp_k = ambient_temp.to_kelvin();
        let ambient_density = self.pressure / (R_SPECIFIC_AIR * (*ambient_temp_k as f32));

        let current_density = self.air_density();
        (ambient_density - current_density) * 9.81 // g = 9.81 m/s²
    }

    /// Check if oxygen is sufficient for combustion
    /// Requires at least 15% O2 concentration
    #[must_use]
    pub fn can_support_combustion(&self) -> bool {
        self.oxygen > 0.195 // 15% of normal concentration
    }

    /// Calculate effective thermal conductivity (W/(m·K))
    /// Accounts for smoke and water vapor
    #[must_use]
    pub fn thermal_conductivity(&self) -> f32 {
        let base_conductivity = 0.026; // Air at 20°C

        // Smoke increases conductivity slightly
        let smoke_factor = 1.0 + self.smoke_particles * 0.1;

        // Water vapor increases conductivity
        let vapor_factor = 1.0 + self.water_vapor * 0.02;

        base_conductivity * smoke_factor * vapor_factor
    }

    /// Reset cell to ambient conditions (for initialization/cleanup)
    pub fn reset_to_ambient(&mut self, elevation: f32) {
        *self = GridCell::new(elevation);
    }

    // Public accessor methods for FFI and external use

    /// Get air temperature (°C)
    #[must_use]
    pub fn temperature(&self) -> Celsius {
        self.temperature
    }

    /// Get wind velocity vector (m/s) - returns reference for zero-copy access
    #[must_use]
    pub fn wind(&self) -> &Vec3 {
        &self.wind
    }

    /// Get relative humidity (0-1)
    #[must_use]
    pub fn humidity(&self) -> Fraction {
        Fraction::new(self.humidity)
    }

    /// Get oxygen concentration (kg/m³)
    #[must_use]
    pub fn oxygen(&self) -> f32 {
        self.oxygen
    }

    /// Get CO concentration (kg/m³)
    #[must_use]
    pub fn carbon_monoxide(&self) -> f32 {
        self.carbon_monoxide
    }

    /// Get CO2 concentration (kg/m³)
    #[must_use]
    pub fn carbon_dioxide(&self) -> f32 {
        self.carbon_dioxide
    }

    /// Get smoke/particulate concentration (kg/m³)
    #[must_use]
    pub fn smoke_particles(&self) -> f32 {
        self.smoke_particles
    }

    /// Get water vapor concentration (kg/m³)
    #[must_use]
    pub fn water_vapor(&self) -> f32 {
        self.water_vapor
    }

    /// Get suppression agent concentration (kg/m³)
    #[must_use]
    pub fn suppression_agent(&self) -> f32 {
        self.suppression_agent
    }

    /// Get cell elevation (m)
    #[must_use]
    pub fn elevation(&self) -> Meters {
        Meters::new(self.elevation)
    }

    /// Check if cell is active
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

/// 3D simulation grid with adaptive refinement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationGrid {
    /// Grid dimensions in world space (m)
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) depth: f32,

    /// Grid resolution (cells)
    pub(crate) nx: usize,
    pub(crate) ny: usize,
    pub(crate) nz: usize,

    /// Cell size (m)
    pub(crate) cell_size: f32,

    /// Grid cells in row-major order: [z * (ny * nx) + y * nx + x]
    pub(crate) cells: Vec<GridCell>,

    /// Terrain data for elevation
    pub(crate) terrain: TerrainData,

    /// Precomputed terrain properties cache (slope/aspect)
    pub(crate) terrain_cache: TerrainCache,

    /// Last base wind used for updates (to skip redundant updates)
    pub(crate) last_base_wind: Vec3,

    /// Track indices of currently active cells for efficient reset
    active_cell_indices: Vec<usize>,

    /// Reusable buffer for marking cells (avoids allocation per frame)
    #[serde(skip)]
    cell_marked_buffer: Vec<bool>,

    /// Ambient conditions
    pub(crate) ambient_temperature: Celsius,
    pub(crate) ambient_wind: Vec3,
    pub(crate) ambient_humidity: f32,
}

impl SimulationGrid {
    /// Create a new simulation grid
    #[must_use]
    pub fn new(width: f32, height: f32, depth: f32, cell_size: f32, terrain: TerrainData) -> Self {
        let nx = (width / cell_size).ceil() as usize;
        let ny = (height / cell_size).ceil() as usize;
        let nz = (depth / cell_size).ceil() as usize;

        let total_cells = nx * ny * nz;
        let mut cells = Vec::with_capacity(total_cells);

        // Initialize cells with terrain elevation
        for _iz in 0..nz {
            for iy in 0..ny {
                for ix in 0..nx {
                    let x = usize_to_f32(ix) * cell_size + cell_size / 2.0;
                    let y = usize_to_f32(iy) * cell_size + cell_size / 2.0;
                    let elevation = terrain.elevation_at(x, y);

                    cells.push(GridCell::new(*elevation));
                }
            }
        }

        // Build terrain cache for fast slope/aspect lookups
        let terrain_cache = terrain.build_cache(nx, ny, cell_size);

        SimulationGrid {
            width,
            height,
            depth,
            nx,
            ny,
            nz,
            cell_size,
            cells,
            terrain,
            terrain_cache,
            last_base_wind: Vec3::zeros(),
            active_cell_indices: Vec::new(),
            cell_marked_buffer: Vec::new(),
            ambient_temperature: Celsius::new(20.0),
            ambient_wind: Vec3::zeros(),
            ambient_humidity: 0.4,
        }
    }

    /// Get cell index from (x, y, z) indices
    #[inline]
    #[must_use]
    pub fn cell_index(&self, ix: usize, iy: usize, iz: usize) -> usize {
        iz * (self.ny * self.nx) + iy * self.nx + ix
    }

    /// Get cell at grid indices (bounds-checked)
    #[must_use]
    pub fn cell_at(&self, ix: usize, iy: usize, iz: usize) -> Option<&GridCell> {
        if ix < self.nx && iy < self.ny && iz < self.nz {
            Some(&self.cells[self.cell_index(ix, iy, iz)])
        } else {
            None
        }
    }

    /// Get mutable cell at grid indices (bounds-checked)
    pub fn cell_at_mut(&mut self, ix: usize, iy: usize, iz: usize) -> Option<&mut GridCell> {
        if ix < self.nx && iy < self.ny && iz < self.nz {
            let idx = self.cell_index(ix, iy, iz);
            Some(&mut self.cells[idx])
        } else {
            None
        }
    }

    /// Get cell at world position using nearest neighbor
    #[must_use]
    pub fn cell_at_position(&self, pos: Vec3) -> Option<&GridCell> {
        let ix = (pos.x / self.cell_size).floor() as isize;
        let iy = (pos.y / self.cell_size).floor() as isize;
        let iz = (pos.z / self.cell_size).floor() as isize;

        if ix >= 0 && iy >= 0 && iz >= 0 {
            self.cell_at(ix as usize, iy as usize, iz as usize)
        } else {
            None
        }
    }

    /// Get mutable cell at world position
    pub fn cell_at_position_mut(&mut self, pos: Vec3) -> Option<&mut GridCell> {
        let ix = (pos.x / self.cell_size).floor() as isize;
        let iy = (pos.y / self.cell_size).floor() as isize;
        let iz = (pos.z / self.cell_size).floor() as isize;

        if ix >= 0 && iy >= 0 && iz >= 0 {
            self.cell_at_mut(ix as usize, iy as usize, iz as usize)
        } else {
            None
        }
    }

    /// Interpolate cell properties at world position (trilinear)
    #[must_use]
    pub fn interpolate_at_position(&self, pos: Vec3) -> GridCell {
        let gx = pos.x / self.cell_size;
        let gy = pos.y / self.cell_size;
        let gz = pos.z / self.cell_size;

        let ix0 = (gx.floor() as usize).min(self.nx.saturating_sub(2));
        let iy0 = (gy.floor() as usize).min(self.ny.saturating_sub(2));
        let iz0 = (gz.floor() as usize).min(self.nz.saturating_sub(2));

        let ix1 = ix0 + 1;
        let iy1 = iy0 + 1;
        let iz1 = iz0 + 1;

        let fx = gx - usize_to_f32(ix0);
        let fy = gy - usize_to_f32(iy0);
        let fz = gz - usize_to_f32(iz0);

        // Get 8 corner cells
        let c000 = &self.cells[self.cell_index(ix0, iy0, iz0)];
        let c100 = &self.cells[self.cell_index(ix1, iy0, iz0)];
        let c010 = &self.cells[self.cell_index(ix0, iy1, iz0)];
        let c110 = &self.cells[self.cell_index(ix1, iy1, iz0)];
        let c001 = &self.cells[self.cell_index(ix0, iy0, iz1)];
        let c101 = &self.cells[self.cell_index(ix1, iy0, iz1)];
        let c011 = &self.cells[self.cell_index(ix0, iy1, iz1)];
        let c111 = &self.cells[self.cell_index(ix1, iy1, iz1)];

        // Trilinear interpolation helper
        let lerp = |a: f32, b: f32, t: f32| a * (1.0 - t) + b * t;
        let lerp_celsius = |a: Celsius, b: Celsius, t: f32| {
            let result = *a * (1.0 - f64::from(t)) + *b * f64::from(t);
            Celsius::new(result.max(-273.15))
        };
        let lerp_vec = |a: Vec3, b: Vec3, t: f32| a * (1.0 - t) + b * t;

        // Interpolate along x
        let c00 = GridCell {
            temperature: lerp_celsius(c000.temperature, c100.temperature, fx),
            wind: lerp_vec(c000.wind, c100.wind, fx),
            humidity: lerp(c000.humidity, c100.humidity, fx),
            oxygen: lerp(c000.oxygen, c100.oxygen, fx),
            carbon_monoxide: lerp(c000.carbon_monoxide, c100.carbon_monoxide, fx),
            carbon_dioxide: lerp(c000.carbon_dioxide, c100.carbon_dioxide, fx),
            smoke_particles: lerp(c000.smoke_particles, c100.smoke_particles, fx),
            water_vapor: lerp(c000.water_vapor, c100.water_vapor, fx),
            radiation_flux: lerp(c000.radiation_flux, c100.radiation_flux, fx),
            elevation: lerp(c000.elevation, c100.elevation, fx),
            refinement_level: c000.refinement_level,
            is_active: c000.is_active || c100.is_active,
            pressure: lerp(c000.pressure, c100.pressure, fx),
            suppression_agent: lerp(c000.suppression_agent, c100.suppression_agent, fx),
        };
        let c10 = GridCell {
            temperature: lerp_celsius(c010.temperature, c110.temperature, fx),
            wind: lerp_vec(c010.wind, c110.wind, fx),
            humidity: lerp(c010.humidity, c110.humidity, fx),
            oxygen: lerp(c010.oxygen, c110.oxygen, fx),
            carbon_monoxide: lerp(c010.carbon_monoxide, c110.carbon_monoxide, fx),
            carbon_dioxide: lerp(c010.carbon_dioxide, c110.carbon_dioxide, fx),
            smoke_particles: lerp(c010.smoke_particles, c110.smoke_particles, fx),
            water_vapor: lerp(c010.water_vapor, c110.water_vapor, fx),
            radiation_flux: lerp(c010.radiation_flux, c110.radiation_flux, fx),
            elevation: lerp(c010.elevation, c110.elevation, fx),
            refinement_level: c010.refinement_level,
            is_active: c010.is_active || c110.is_active,
            pressure: lerp(c010.pressure, c110.pressure, fx),
            suppression_agent: lerp(c010.suppression_agent, c110.suppression_agent, fx),
        };
        let c01 = GridCell {
            temperature: lerp_celsius(c001.temperature, c101.temperature, fx),
            wind: lerp_vec(c001.wind, c101.wind, fx),
            humidity: lerp(c001.humidity, c101.humidity, fx),
            oxygen: lerp(c001.oxygen, c101.oxygen, fx),
            carbon_monoxide: lerp(c001.carbon_monoxide, c101.carbon_monoxide, fx),
            carbon_dioxide: lerp(c001.carbon_dioxide, c101.carbon_dioxide, fx),
            smoke_particles: lerp(c001.smoke_particles, c101.smoke_particles, fx),
            water_vapor: lerp(c001.water_vapor, c101.water_vapor, fx),
            radiation_flux: lerp(c001.radiation_flux, c101.radiation_flux, fx),
            elevation: lerp(c001.elevation, c101.elevation, fx),
            refinement_level: c001.refinement_level,
            is_active: c001.is_active || c101.is_active,
            pressure: lerp(c001.pressure, c101.pressure, fx),
            suppression_agent: lerp(c001.suppression_agent, c101.suppression_agent, fx),
        };
        let c11 = GridCell {
            temperature: lerp_celsius(c011.temperature, c111.temperature, fx),
            wind: lerp_vec(c011.wind, c111.wind, fx),
            humidity: lerp(c011.humidity, c111.humidity, fx),
            oxygen: lerp(c011.oxygen, c111.oxygen, fx),
            carbon_monoxide: lerp(c011.carbon_monoxide, c111.carbon_monoxide, fx),
            carbon_dioxide: lerp(c011.carbon_dioxide, c111.carbon_dioxide, fx),
            smoke_particles: lerp(c011.smoke_particles, c111.smoke_particles, fx),
            water_vapor: lerp(c011.water_vapor, c111.water_vapor, fx),
            radiation_flux: lerp(c011.radiation_flux, c111.radiation_flux, fx),
            elevation: lerp(c011.elevation, c111.elevation, fx),
            refinement_level: c011.refinement_level,
            is_active: c011.is_active || c111.is_active,
            pressure: lerp(c011.pressure, c111.pressure, fx),
            suppression_agent: lerp(c011.suppression_agent, c111.suppression_agent, fx),
        };

        // Interpolate along y
        let c0 = GridCell {
            temperature: lerp_celsius(c00.temperature, c10.temperature, fy),
            wind: lerp_vec(c00.wind, c10.wind, fy),
            humidity: lerp(c00.humidity, c10.humidity, fy),
            oxygen: lerp(c00.oxygen, c10.oxygen, fy),
            carbon_monoxide: lerp(c00.carbon_monoxide, c10.carbon_monoxide, fy),
            carbon_dioxide: lerp(c00.carbon_dioxide, c10.carbon_dioxide, fy),
            smoke_particles: lerp(c00.smoke_particles, c10.smoke_particles, fy),
            water_vapor: lerp(c00.water_vapor, c10.water_vapor, fy),
            radiation_flux: lerp(c00.radiation_flux, c10.radiation_flux, fy),
            elevation: lerp(c00.elevation, c10.elevation, fy),
            refinement_level: c00.refinement_level,
            is_active: c00.is_active || c10.is_active,
            pressure: lerp(c00.pressure, c10.pressure, fy),
            suppression_agent: lerp(c00.suppression_agent, c10.suppression_agent, fy),
        };
        let c1 = GridCell {
            temperature: lerp_celsius(c01.temperature, c11.temperature, fy),
            wind: lerp_vec(c01.wind, c11.wind, fy),
            humidity: lerp(c01.humidity, c11.humidity, fy),
            oxygen: lerp(c01.oxygen, c11.oxygen, fy),
            carbon_monoxide: lerp(c01.carbon_monoxide, c11.carbon_monoxide, fy),
            carbon_dioxide: lerp(c01.carbon_dioxide, c11.carbon_dioxide, fy),
            smoke_particles: lerp(c01.smoke_particles, c11.smoke_particles, fy),
            water_vapor: lerp(c01.water_vapor, c11.water_vapor, fy),
            radiation_flux: lerp(c01.radiation_flux, c11.radiation_flux, fy),
            elevation: lerp(c01.elevation, c11.elevation, fy),
            refinement_level: c01.refinement_level,
            is_active: c01.is_active || c11.is_active,
            pressure: lerp(c01.pressure, c11.pressure, fy),
            suppression_agent: lerp(c01.suppression_agent, c11.suppression_agent, fy),
        };

        // Final interpolation along z
        GridCell {
            temperature: lerp_celsius(c0.temperature, c1.temperature, fz),
            wind: lerp_vec(c0.wind, c1.wind, fz),
            humidity: lerp(c0.humidity, c1.humidity, fz),
            oxygen: lerp(c0.oxygen, c1.oxygen, fz),
            carbon_monoxide: lerp(c0.carbon_monoxide, c1.carbon_monoxide, fz),
            carbon_dioxide: lerp(c0.carbon_dioxide, c1.carbon_dioxide, fz),
            smoke_particles: lerp(c0.smoke_particles, c1.smoke_particles, fz),
            water_vapor: lerp(c0.water_vapor, c1.water_vapor, fz),
            radiation_flux: lerp(c0.radiation_flux, c1.radiation_flux, fz),
            elevation: lerp(c0.elevation, c1.elevation, fz),
            refinement_level: c0.refinement_level,
            is_active: c0.is_active || c1.is_active,
            pressure: lerp(c0.pressure, c1.pressure, fz),
            suppression_agent: lerp(c0.suppression_agent, c1.suppression_agent, fz),
        }
    }

    /// Update atmospheric diffusion (parallel, active cells only)
    pub(crate) fn update_diffusion(&mut self, dt: f32) {
        // Early exit if no active cells (performance optimization)
        if self.active_cell_indices.is_empty() {
            return;
        }

        // Enhanced diffusion coefficient to account for fire-driven convection
        // Still air: 0.00002 m²/s, but fires create strong convective mixing
        // Effective diffusivity can be 100-1000x higher near fires
        let diffusion_coefficient = 0.002; // m²/s - 100x higher for fire convection
        let dx2 = self.cell_size * self.cell_size;
        let diffusion_factor = diffusion_coefficient * dt / dx2;

        // Pre-cache grid dimensions to avoid repeated field access
        let nx = self.nx;
        let ny = self.ny;
        let nz = self.nz;
        let nx_i32 = nx as i32;
        let ny_i32 = ny as i32;
        let nz_i32 = nz as i32;
        let ny_nx = ny * nx;

        // Collect cells that need diffusion processing:
        // 1. Active cells themselves (hot from fire)
        // 2. Cells within 1 cell of active cells (reduced from 2 for performance)
        // PERFORMANCE: Reduced from 5x5x5 (125) to 3x3x3 (27) stencil for 4.6x speedup
        // This still allows heat spreading but reduces overhead at high element counts

        // Pre-compute neighbor offsets for 3x3x3 stencil (optimized for performance)
        static NEIGHBOR_OFFSETS: [(i32, i32, i32); 27] = {
            let mut offsets = [(0, 0, 0); 27];
            let mut i = 0;
            let mut dz = -1;
            while dz <= 1 {
                let mut dy = -1;
                while dy <= 1 {
                    let mut dx = -1;
                    while dx <= 1 {
                        offsets[i] = (dx, dy, dz);
                        i += 1;
                        dx += 1;
                    }
                    dy += 1;
                }
                dz += 1;
            }
            offsets
        };

        // OPTIMIZATION: Reuse existing cell_marked_buffer to avoid allocation each frame
        // This buffer is already sized correctly from mark_active_cells
        let total_cells = self.cells.len();
        if self.cell_marked_buffer.len() != total_cells {
            self.cell_marked_buffer.resize(total_cells, false);
        }
        // Note: cell_marked_buffer is already all false from mark_active_cells cleanup
        let mut cells_to_process = Vec::with_capacity(self.active_cell_indices.len() * 27);

        // OPTIMIZATION: Use direct slice access to avoid iterator overhead
        let active_indices = &self.active_cell_indices[..];
        let active_len = active_indices.len();

        for i in 0..active_len {
            // SAFETY: i is bounded by active_len, which equals active_indices.len(),
            // so i is always a valid index into active_indices
            let idx = unsafe { *active_indices.get_unchecked(i) };
            let iz = idx / ny_nx;
            let iy = (idx % ny_nx) / nx;
            let ix = idx % nx;

            // Use pre-computed offsets to avoid nested loop overhead
            for offset_idx in 0..27 {
                // SAFETY: offset_idx is bounded by 0..27, and NEIGHBOR_OFFSETS has exactly 27 elements
                // (const array of size 27), so offset_idx is always a valid index
                let (dx, dy, dz) = unsafe { *NEIGHBOR_OFFSETS.get_unchecked(offset_idx) };
                let ni = ix as i32 + dx;
                let nj = iy as i32 + dy;
                let nk = iz as i32 + dz;

                // Combined bounds check (compiler optimizes better)
                if ni >= 0 && ni < nx_i32 && nj >= 0 && nj < ny_i32 && nk >= 0 && nk < nz_i32 {
                    // Inline cell_index calculation for performance
                    let n_idx = (nk as usize) * ny_nx + (nj as usize) * nx + (ni as usize);

                    // Only add if not already marked (avoids duplicates)
                    if !self.cell_marked_buffer[n_idx] {
                        self.cell_marked_buffer[n_idx] = true;
                        cells_to_process.push(n_idx);
                    }
                }
            }
        }

        let cells_vec = cells_to_process;

        // Cache ambient temperature
        let ambient_temp = self.ambient_temperature;
        let grid_dims = (nx, ny, nz, ny_nx);
        let params = (ambient_temp, diffusion_factor, dt);

        // OPTIMIZATION: Adjust parallel threshold based on profiling
        // For <16000 cells, sequential is faster (eliminates 12.4% Rayon overhead)
        const PARALLEL_THRESHOLD: usize = 16000;

        let temp_updates: Vec<(usize, Celsius)> = if cells_vec.len() < PARALLEL_THRESHOLD {
            // Sequential processing for small/medium workloads
            cells_vec
                .iter()
                .filter_map(|&idx| self.process_diffusion_cell(idx, grid_dims, params))
                .collect()
        } else {
            // Parallel processing for large workloads
            // Use larger chunk size to reduce Rayon overhead
            const CHUNK_SIZE: usize = 512;
            cells_vec
                .par_chunks(CHUNK_SIZE)
                .flat_map(|chunk| {
                    let mut local_updates = Vec::with_capacity(chunk.len());
                    for &idx in chunk {
                        if let Some(update) = self.process_diffusion_cell(idx, grid_dims, params) {
                            local_updates.push(update);
                        }
                    }
                    local_updates
                })
                .collect()
        };

        // Apply updates (batched for cache locality)
        for (idx, new_temp) in temp_updates {
            self.cells[idx].temperature = new_temp;
        }

        // OPTIMIZATION: Clear cell_marked_buffer for next frame (cleanup)
        for &idx in &cells_vec {
            self.cell_marked_buffer[idx] = false;
        }
    }

    /// Process diffusion for a single cell (extracted for chunked parallelism)
    #[inline(always)]
    fn process_diffusion_cell(
        &self,
        idx: usize,
        grid_dims: (usize, usize, usize, usize), // (nx, ny, nz, ny_nx)
        params: (Celsius, f32, f32),             // (ambient_temp, diffusion_factor, dt)
    ) -> Option<(usize, Celsius)> {
        let (nx, ny, nz, ny_nx) = grid_dims;
        let (ambient_temp, diffusion_factor, dt) = params;

        // SAFETY: idx comes from cells_to_process which contains only valid cell indices
        // that were marked in cell_marked_buffer. All marked indices are guaranteed to be
        // within bounds (0..self.cells.len()) by the marking logic in update_diffusion
        let cell = unsafe { self.cells.get_unchecked(idx) };
        let cell_temp = cell.temperature;

        // Early termination if temperature near ambient
        let temp_diff = cell_temp - ambient_temp;
        if temp_diff.abs() < 1.0 {
            return None;
        }

        let iz = idx / ny_nx;
        let iy = (idx % ny_nx) / nx;
        let ix = idx % nx;

        // OPTIMIZATION: Compute all neighbors with fewer branches
        // Use conditional addition instead of if statements for better branch prediction
        let mut laplacian = Celsius::new(0.0);

        // X neighbors (most likely to exist - interior cells)
        let has_x_minus = ix > 0;
        let has_x_plus = ix < nx - 1;
        if has_x_minus {
            // SAFETY: has_x_minus guarantees ix > 0, so idx - 1 is a valid cell index
            // within the grid bounds (idx is already validated)
            laplacian =
                laplacian + unsafe { self.cells.get_unchecked(idx - 1).temperature } - cell_temp;
        }
        if has_x_plus {
            // SAFETY: has_x_plus guarantees ix < nx - 1, so idx + 1 is a valid cell index
            // within the grid bounds (idx is already validated)
            laplacian =
                laplacian + unsafe { self.cells.get_unchecked(idx + 1).temperature } - cell_temp;
        }

        // Y neighbors
        let has_y_minus = iy > 0;
        let has_y_plus = iy < ny - 1;
        if has_y_minus {
            // SAFETY: has_y_minus guarantees iy > 0, so idx - nx is a valid cell index
            // within the grid bounds (idx is already validated)
            laplacian =
                laplacian + unsafe { self.cells.get_unchecked(idx - nx).temperature } - cell_temp;
        }
        if has_y_plus {
            // SAFETY: has_y_plus guarantees iy < ny - 1, so idx + nx is a valid cell index
            // within the grid bounds (idx is already validated)
            laplacian =
                laplacian + unsafe { self.cells.get_unchecked(idx + nx).temperature } - cell_temp;
        }

        // Z neighbors
        let has_z_minus = iz > 0;
        let has_z_plus = iz < nz - 1;
        if has_z_minus {
            // SAFETY: has_z_minus guarantees iz > 0, so idx - ny_nx is a valid cell index
            // within the grid bounds (idx is already validated)
            laplacian = laplacian + unsafe { self.cells.get_unchecked(idx - ny_nx).temperature }
                - cell_temp;
        }
        if has_z_plus {
            // SAFETY: has_z_plus guarantees iz < nz - 1, so idx + ny_nx is a valid cell index
            // within the grid bounds (idx is already validated)
            laplacian = laplacian + unsafe { self.cells.get_unchecked(idx + ny_nx).temperature }
                - cell_temp;
        }

        // Most cells have all 6 neighbors (interior cells), early exit rare
        let neighbor_count = u32::from(has_x_minus)
            + u32::from(has_x_plus)
            + u32::from(has_y_minus)
            + u32::from(has_y_plus)
            + u32::from(has_z_minus)
            + u32::from(has_z_plus);

        if neighbor_count == 0 {
            return None;
        }

        let temp_change = laplacian * f64::from(diffusion_factor);
        let mut new_temp = cell_temp + temp_change;

        // OPTIMIZATION: Combine cooling and clamping into single branch
        if *new_temp > 100.0 {
            // Natural cooling increases with temperature (0.5% per second above ambient)
            let cooling = (new_temp - ambient_temp) * f64::from(0.005 * dt);
            new_temp = (new_temp - cooling)
                .max(ambient_temp)
                .min(Celsius::new(800.0));
        } else {
            new_temp = new_temp.min(Celsius::new(800.0));
        }

        Some((idx, new_temp))
    }

    /// Update buoyancy-driven convection (hot air rises)
    pub(crate) fn update_buoyancy(&mut self, dt: f32) {
        // Vertical advection of heat due to buoyancy
        for iz in 1..self.nz {
            for iy in 0..self.ny {
                for ix in 0..self.nx {
                    let idx_below = self.cell_index(ix, iy, iz - 1);
                    let idx_current = self.cell_index(ix, iy, iz);

                    let cell_below = &self.cells[idx_below];
                    let buoyancy = cell_below.buoyancy_force(self.ambient_temperature);

                    if buoyancy > 0.0 {
                        // Hot air rises - transfer heat upward
                        let vertical_velocity = (buoyancy * dt).sqrt(); // Simplified
                        let transfer_fraction = (vertical_velocity * dt / self.cell_size).min(0.3);

                        let temp_diff =
                            cell_below.temperature - self.cells[idx_current].temperature;
                        if *temp_diff > 0.0 {
                            let heat_transfer = temp_diff * f64::from(transfer_fraction);
                            self.cells[idx_current].temperature =
                                (self.cells[idx_current].temperature + heat_transfer)
                                    .min(Celsius::new(800.0));
                            let new_below_temp = (*self.cells[idx_below].temperature
                                - *heat_transfer)
                                .max(*self.ambient_temperature);
                            self.cells[idx_below].temperature = Celsius::new(new_below_temp);
                        }
                    }
                }
            }
        }
    }

    /// Mark cells near burning elements as active
    pub(crate) fn mark_active_cells(&mut self, active_positions: &[Vec3], activation_radius: f32) {
        // Early exit if no positions
        if active_positions.is_empty() {
            // Deactivate all previously active cells
            for &idx in &self.active_cell_indices {
                self.cells[idx].is_active = false;
            }
            self.active_cell_indices.clear();
            return;
        }

        // Ensure buffer is sized correctly
        let total_cells = self.cells.len();
        if self.cell_marked_buffer.len() != total_cells {
            self.cell_marked_buffer.resize(total_cells, false);
        }

        // OPTIMIZATION: Batch processing - compute constants once
        let cells_radius = (activation_radius / self.cell_size).ceil() as i32;
        let nx_i32 = self.nx as i32;
        let ny_i32 = self.ny as i32;
        let nz_i32 = self.nz as i32;
        let ny_nx = self.ny * self.nx;

        // OPTIMIZATION: Spatial bucketing to eliminate redundant work
        // Group burning elements by grid cell to avoid marking same cells multiple times
        // This is critical when many burning elements are clustered together
        let mut cell_buckets: rustc_hash::FxHashMap<(i32, i32, i32), u32> =
            rustc_hash::FxHashMap::with_capacity_and_hasher(
                active_positions.len() / 4,
                rustc_hash::FxBuildHasher,
            );

        for pos in active_positions {
            let cx = (pos.x / self.cell_size) as i32;
            let cy = (pos.y / self.cell_size) as i32;
            let cz = (pos.z / self.cell_size) as i32;
            *cell_buckets.entry((cx, cy, cz)).or_insert(0) += 1;
        }

        // Pre-allocate marked_this_frame based on unique buckets (much smaller than positions)
        let mut marked_this_frame = Vec::with_capacity(cell_buckets.len() * 125);

        // OPTIMIZATION: Use pre-computed offsets, only process unique bucket centers
        // This eliminates redundant work when multiple elements are in same cell
        for &(cx, cy, cz) in cell_buckets.keys() {
            if cells_radius == 2 {
                // FAST PATH: Use pre-computed offsets (99.9% of cases)
                for offset_idx in 0..125 {
                    // SAFETY: offset_idx is bounded by 0..125, and MARK_ACTIVE_OFFSETS has exactly 125 elements
                    // (const array of size 125), so offset_idx is always a valid index
                    let (dx, dy, dz) = unsafe { *MARK_ACTIVE_OFFSETS.get_unchecked(offset_idx) };
                    let nx = cx + dx;
                    let ny = cy + dy;
                    let nz = cz + dz;

                    // Bounds check (branchless multiplication for better performance)
                    let in_bounds = (nx >= 0 && nx < nx_i32)
                        & (ny >= 0 && ny < ny_i32)
                        & (nz >= 0 && nz < nz_i32);

                    if in_bounds {
                        let idx = (nz as usize) * ny_nx + (ny as usize) * self.nx + (nx as usize);
                        if !self.cell_marked_buffer[idx] {
                            self.cell_marked_buffer[idx] = true;
                            marked_this_frame.push(idx);
                        }
                    }
                }
            } else {
                // SLOW PATH: Dynamic radius (rare, only if activation_radius changes)
                let dz_min = (-cells_radius).max(-cz);
                let dz_max = cells_radius.min(nz_i32 - 1 - cz);
                let dy_min = (-cells_radius).max(-cy);
                let dy_max = cells_radius.min(ny_i32 - 1 - cy);
                let dx_min = (-cells_radius).max(-cx);
                let dx_max = cells_radius.min(nx_i32 - 1 - cx);

                for dz in dz_min..=dz_max {
                    let iz_offset = ((cz + dz) as usize) * ny_nx;
                    for dy in dy_min..=dy_max {
                        let iy_offset = ((cy + dy) as usize) * self.nx;
                        for dx in dx_min..=dx_max {
                            let idx = iz_offset + iy_offset + (cx + dx) as usize;
                            if !self.cell_marked_buffer[idx] {
                                self.cell_marked_buffer[idx] = true;
                                marked_this_frame.push(idx);
                            }
                        }
                    }
                }
            }
        }

        // OPTIMIZATION: Only iterate cells that matter
        // Single pass: update active_cell_indices to point to marked_this_frame
        // Deactivate old cells that are no longer marked
        for &idx in &self.active_cell_indices {
            if !self.cell_marked_buffer[idx] {
                self.cells[idx].is_active = false;
            }
        }

        // Activate newly marked cells and use marked_this_frame as new index
        for &idx in &marked_this_frame {
            if !self.cells[idx].is_active {
                self.cells[idx].is_active = true;
            }
            self.cell_marked_buffer[idx] = false; // Reset for next frame
        }

        // Reuse marked_this_frame allocation (it already has the right indices)
        self.active_cell_indices = marked_this_frame;
    }

    /// Get number of active cells
    #[must_use]
    pub fn active_cell_count(&self) -> usize {
        self.cells.iter().filter(|c| c.is_active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::TerrainData;

    #[test]
    fn test_grid_creation() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let grid = SimulationGrid::new(100.0, 100.0, 50.0, 5.0, terrain);

        assert_eq!(grid.nx, 20);
        assert_eq!(grid.ny, 20);
        assert_eq!(grid.nz, 10);
        assert_eq!(grid.cells.len(), 20 * 20 * 10);
    }

    #[test]
    fn test_cell_access() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let mut grid = SimulationGrid::new(100.0, 100.0, 50.0, 10.0, terrain);

        // Grid is 10x10x5 cells (100/10, 100/10, 50/10)
        // Get cell and modify
        if let Some(cell) = grid.cell_at_mut(5, 5, 2) {
            cell.temperature = Celsius::new(100.0);
        }

        // Verify change
        assert_eq!(*grid.cell_at(5, 5, 2).unwrap().temperature, 100.0);
    }

    #[test]
    fn test_position_query() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let mut grid = SimulationGrid::new(100.0, 100.0, 50.0, 10.0, terrain);

        let pos = Vec3::new(55.0, 55.0, 25.0);
        if let Some(cell) = grid.cell_at_position_mut(pos) {
            cell.temperature = Celsius::new(200.0);
        }

        assert_eq!(*grid.cell_at_position(pos).unwrap().temperature, 200.0);
    }

    #[test]
    fn test_air_density() {
        let cell_cold = GridCell::new(0.0);
        let mut cell_hot = GridCell::new(0.0);
        cell_hot.temperature = Celsius::new(500.0);

        // Hot air is less dense
        assert!(cell_hot.air_density() < cell_cold.air_density());
    }

    #[test]
    fn test_buoyancy() {
        use crate::core_types::units::Celsius;

        let mut cell_hot = GridCell::new(0.0);
        cell_hot.temperature = Celsius::new(300.0);

        let buoyancy = cell_hot.buoyancy_force(Celsius::new(20.0));

        // Hot air creates upward force
        assert!(buoyancy > 0.0);
    }

    #[test]
    fn test_oxygen_combustion() {
        let cell_normal = GridCell::new(0.0);
        assert!(cell_normal.can_support_combustion());

        let mut cell_depleted = GridCell::new(0.0);
        cell_depleted.oxygen = 0.1; // Low oxygen
        assert!(!cell_depleted.can_support_combustion());
    }

    #[test]
    fn test_active_cells() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let mut grid = SimulationGrid::new(100.0, 100.0, 50.0, 5.0, terrain);

        let active_pos = vec![Vec3::new(50.0, 50.0, 10.0)];
        grid.mark_active_cells(&active_pos, 15.0);

        let active_count = grid.active_cell_count();
        assert!(active_count > 0);
        assert!(active_count < grid.cells.len());
    }
}
