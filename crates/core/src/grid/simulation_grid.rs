//! 3D simulation grid with atmospheric properties and adaptive refinement
//!
//! Implements a hybrid grid system where atmospheric properties (temperature, wind,
//! humidity, oxygen, combustion products) are tracked per cell, while discrete fuel
//! elements interact with cells for extreme realism.

use crate::core_types::element::Vec3;
use crate::grid::TerrainData;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Atmospheric properties tracked per grid cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCell {
    /// Air temperature (°C)
    pub(crate) temperature: f32,
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
    pub fn new(elevation: f32) -> Self {
        GridCell {
            temperature: 20.0, // Ambient 20°C
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
    /// ρ = P / (R_specific × T_kelvin)
    pub fn air_density(&self) -> f32 {
        const R_SPECIFIC_AIR: f32 = 287.05; // J/(kg·K)
        let temp_k = self.temperature + 273.15;
        self.pressure / (R_SPECIFIC_AIR * temp_k)
    }

    /// Calculate buoyancy force per unit volume (N/m³)
    /// Based on temperature difference from ambient
    pub fn buoyancy_force(&self, _ambient_temp: f32) -> f32 {
        let ambient_density = 1.2; // kg/m³ at 20°C
        let current_density = self.air_density();
        (ambient_density - current_density) * 9.81 // g = 9.81 m/s²
    }

    /// Check if oxygen is sufficient for combustion
    /// Requires at least 15% O2 concentration
    pub fn can_support_combustion(&self) -> bool {
        self.oxygen > 0.195 // 15% of normal concentration
    }

    /// Calculate effective thermal conductivity (W/(m·K))
    /// Accounts for smoke and water vapor
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
    pub fn temperature(&self) -> f32 {
        self.temperature
    }
    
    /// Get wind velocity vector (m/s) - returns reference for zero-copy access
    pub fn wind(&self) -> &Vec3 {
        &self.wind
    }
    
    /// Get relative humidity (0-1)
    pub fn humidity(&self) -> f32 {
        self.humidity
    }
    
    /// Get oxygen concentration (kg/m³)
    pub fn oxygen(&self) -> f32 {
        self.oxygen
    }
    
    /// Get CO concentration (kg/m³)
    pub fn carbon_monoxide(&self) -> f32 {
        self.carbon_monoxide
    }
    
    /// Get CO2 concentration (kg/m³)
    pub fn carbon_dioxide(&self) -> f32 {
        self.carbon_dioxide
    }
    
    /// Get smoke/particulate concentration (kg/m³)
    pub fn smoke_particles(&self) -> f32 {
        self.smoke_particles
    }
    
    /// Get water vapor concentration (kg/m³)
    pub fn water_vapor(&self) -> f32 {
        self.water_vapor
    }
    
    /// Get suppression agent concentration (kg/m³)
    pub fn suppression_agent(&self) -> f32 {
        self.suppression_agent
    }
    
    /// Get cell elevation (m)
    pub fn elevation(&self) -> f32 {
        self.elevation
    }
    
    /// Check if cell is active
    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

/// 3D simulation grid with adaptive refinement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationGrid {
    /// Grid dimensions in world space (m)
    pub width: f32,
    pub height: f32,
    pub depth: f32,

    /// Grid resolution (cells)
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,

    /// Cell size (m)
    pub cell_size: f32,

    /// Grid cells in row-major order: [z * (ny * nx) + y * nx + x]
    pub cells: Vec<GridCell>,

    /// Terrain data for elevation
    pub terrain: TerrainData,

    /// Ambient conditions
    pub ambient_temperature: f32,
    pub ambient_wind: Vec3,
    pub ambient_humidity: f32,
}

impl SimulationGrid {
    /// Create a new simulation grid
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
                    let x = ix as f32 * cell_size + cell_size / 2.0;
                    let y = iy as f32 * cell_size + cell_size / 2.0;
                    let elevation = terrain.elevation_at(x, y);

                    cells.push(GridCell::new(elevation));
                }
            }
        }

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
            ambient_temperature: 20.0,
            ambient_wind: Vec3::zeros(),
            ambient_humidity: 0.4,
        }
    }

    /// Get cell index from (x, y, z) indices
    #[inline]
    pub fn cell_index(&self, ix: usize, iy: usize, iz: usize) -> usize {
        iz * (self.ny * self.nx) + iy * self.nx + ix
    }

    /// Get cell at grid indices (bounds-checked)
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

        let fx = gx - ix0 as f32;
        let fy = gy - iy0 as f32;
        let fz = gz - iz0 as f32;

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
        let lerp_vec = |a: Vec3, b: Vec3, t: f32| a * (1.0 - t) + b * t;

        // Interpolate along x
        let c00 = GridCell {
            temperature: lerp(c000.temperature, c100.temperature, fx),
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
            temperature: lerp(c010.temperature, c110.temperature, fx),
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
            temperature: lerp(c001.temperature, c101.temperature, fx),
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
            temperature: lerp(c011.temperature, c111.temperature, fx),
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
            temperature: lerp(c00.temperature, c10.temperature, fy),
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
            temperature: lerp(c01.temperature, c11.temperature, fy),
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
            temperature: lerp(c0.temperature, c1.temperature, fz),
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

    /// Update atmospheric diffusion (parallel)
    pub fn update_diffusion(&mut self, dt: f32) {
        let diffusion_coefficient = 0.00002; // m²/s for heat in air
        let dx2 = self.cell_size * self.cell_size;
        let diffusion_factor = diffusion_coefficient * dt / dx2;

        // Parallel diffusion calculation
        let temp_updates: Vec<(usize, f32)> = self
            .cells
            .par_iter()
            .enumerate()
            .flat_map(|(idx, cell)| {
                let iz = idx / (self.ny * self.nx);
                let iy = (idx % (self.ny * self.nx)) / self.nx;
                let ix = idx % self.nx;

                let mut updates = Vec::new();

                // 6-neighbor stencil for diffusion
                let mut laplacian = 0.0;
                let mut neighbor_count = 0;

                for (di, dj, dk) in [
                    (-1i32, 0i32, 0i32),
                    (1, 0, 0),
                    (0, -1, 0),
                    (0, 1, 0),
                    (0, 0, -1),
                    (0, 0, 1),
                ] {
                    let ni = ix as i32 + di;
                    let nj = iy as i32 + dj;
                    let nk = iz as i32 + dk;

                    if ni >= 0
                        && ni < self.nx as i32
                        && nj >= 0
                        && nj < self.ny as i32
                        && nk >= 0
                        && nk < self.nz as i32
                    {
                        let n_idx = self.cell_index(ni as usize, nj as usize, nk as usize);
                        let neighbor_temp = self.cells[n_idx].temperature;
                        laplacian += neighbor_temp - cell.temperature;
                        neighbor_count += 1;
                    }
                }

                if neighbor_count > 0 {
                    let temp_change = diffusion_factor * laplacian;
                    let mut new_temp = cell.temperature + temp_change;

                    // Add modest cooling for hot cells to prevent unrealistic accumulation
                    if new_temp > 100.0 {
                        // Natural cooling increases with temperature
                        let cooling_factor = 0.005; // 0.5% per second above ambient
                        let cooling = (new_temp - self.ambient_temperature) * cooling_factor * dt;
                        new_temp -= cooling;
                        new_temp = new_temp.max(self.ambient_temperature);
                    }

                    // Cap at realistic maximum
                    new_temp = new_temp.min(800.0);

                    updates.push((idx, new_temp));
                }

                updates
            })
            .collect();

        // Apply updates
        for (idx, new_temp) in temp_updates {
            self.cells[idx].temperature = new_temp;
        }
    }

    /// Update buoyancy-driven convection (hot air rises)
    pub fn update_buoyancy(&mut self, dt: f32) {
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
                        if temp_diff > 0.0 {
                            let heat_transfer = temp_diff * transfer_fraction;
                            self.cells[idx_current].temperature += heat_transfer;
                            // Cap at realistic maximum for wildfire air temperatures
                            self.cells[idx_current].temperature =
                                self.cells[idx_current].temperature.min(800.0);
                            self.cells[idx_below].temperature -= heat_transfer;
                        }
                    }
                }
            }
        }
    }

    /// Mark cells near burning elements as active
    pub fn mark_active_cells(&mut self, active_positions: &[Vec3], activation_radius: f32) {
        // Reset all cells to inactive
        for cell in &mut self.cells {
            cell.is_active = false;
        }

        // Mark cells within radius of active positions
        let cells_radius = (activation_radius / self.cell_size).ceil() as i32;

        for pos in active_positions {
            let cx = (pos.x / self.cell_size) as i32;
            let cy = (pos.y / self.cell_size) as i32;
            let cz = (pos.z / self.cell_size) as i32;

            for dz in -cells_radius..=cells_radius {
                for dy in -cells_radius..=cells_radius {
                    for dx in -cells_radius..=cells_radius {
                        let ix = cx + dx;
                        let iy = cy + dy;
                        let iz = cz + dz;

                        if ix >= 0
                            && ix < self.nx as i32
                            && iy >= 0
                            && iy < self.ny as i32
                            && iz >= 0
                            && iz < self.nz as i32
                        {
                            let idx = self.cell_index(ix as usize, iy as usize, iz as usize);
                            self.cells[idx].is_active = true;
                        }
                    }
                }
            }
        }
    }

    /// Get number of active cells
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
            cell.temperature = 100.0;
        }

        // Verify change
        assert_eq!(grid.cell_at(5, 5, 2).unwrap().temperature, 100.0);
    }

    #[test]
    fn test_position_query() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let mut grid = SimulationGrid::new(100.0, 100.0, 50.0, 10.0, terrain);

        let pos = Vec3::new(55.0, 55.0, 25.0);
        if let Some(cell) = grid.cell_at_position_mut(pos) {
            cell.temperature = 200.0;
        }

        assert_eq!(grid.cell_at_position(pos).unwrap().temperature, 200.0);
    }

    #[test]
    fn test_air_density() {
        let cell_cold = GridCell::new(0.0);
        let mut cell_hot = GridCell::new(0.0);
        cell_hot.temperature = 500.0;

        // Hot air is less dense
        assert!(cell_hot.air_density() < cell_cold.air_density());
    }

    #[test]
    fn test_buoyancy() {
        let mut cell_hot = GridCell::new(0.0);
        cell_hot.temperature = 300.0;

        let buoyancy = cell_hot.buoyancy_force(20.0);

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
