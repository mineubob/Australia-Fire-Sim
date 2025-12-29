//! CPU-based field solver implementation
//!
//! This module provides a CPU implementation of the `FieldSolver` trait using
//! `Vec<f32>` arrays and Rayon for parallelism. This backend is always available
//! and serves as a fallback when GPU acceleration is not available.

use super::combustion::{step_combustion_cpu, CombustionParams};
use super::fields::FieldData;
use super::fuel_variation::HeterogeneityConfig;
use super::heat_transfer::{step_heat_transfer_cpu, HeatTransferParams};
use super::level_set::{
    compute_spread_rate_cpu, step_ignition_sync_cpu, step_level_set_cpu, LevelSetParams,
};
use super::quality::QualityPreset;
use super::terrain_slope::{calculate_effective_slope, calculate_slope_factor, TerrainFields};
use super::FieldSolver;
use crate::TerrainData;
use std::borrow::Cow;

// Helper to convert usize to f32, centralizing the intentional precision loss
#[inline]
#[expect(clippy::cast_precision_loss)]
fn usize_to_f32(v: usize) -> f32 {
    v as f32
}

/// CPU-based field solver using Rayon for parallelism
///
/// This solver stores all field data as `Vec<f32>` arrays and uses Rayon's
/// parallel iterators for multi-threaded computation. It implements the same
/// physics as the GPU solver but runs on the CPU.
pub struct CpuFieldSolver {
    // Ping-pong buffers for each field (read from one, write to other, then swap)
    temperature: FieldData,
    temperature_back: FieldData,

    // Additional fields (used in Phase 2-3)
    fuel_load: FieldData,
    moisture: FieldData,
    level_set: FieldData,
    level_set_back: FieldData,
    oxygen: FieldData,

    // Spread rate field (computed from temperature gradient)
    spread_rate: FieldData,

    // Static fields (don't change during simulation)
    #[expect(dead_code)]
    fuel_type: Vec<u8>,
    #[expect(dead_code)]
    terrain_height: Vec<f32>,

    // Phase 0: Terrain slope and aspect for fire spread modulation
    terrain_fields: TerrainFields,

    // Phase 2: Fuel heterogeneity configuration (stored for potential runtime queries)
    #[expect(dead_code)]
    heterogeneity_config: HeterogeneityConfig,

    // Grid dimensions
    width: usize,
    height: usize,
    cell_size: f32,
}

impl CpuFieldSolver {
    /// Create a new CPU field solver
    ///
    /// Initializes all fields based on terrain data and quality preset.
    /// Applies Phase 0 terrain slope calculation and Phase 2 fuel heterogeneity.
    ///
    /// # Arguments
    ///
    /// * `terrain` - Terrain data for initialization
    /// * `quality` - Quality preset determining grid resolution
    ///
    /// # Returns
    ///
    /// New CPU field solver instance
    #[must_use]
    pub fn new(terrain: &TerrainData, quality: QualityPreset) -> Self {
        let (width_u32, height_u32, cell_size) = quality.grid_dimensions(terrain);
        let width = width_u32 as usize;
        let height = height_u32 as usize;
        let num_cells = width * height;

        // Initialize fields
        let temperature = FieldData::with_value(width, height, 293.15); // ~20°C ambient
        let temperature_back = FieldData::new(width, height);
        let mut fuel_load = FieldData::with_value(width, height, 1.0); // 1 kg/m² default
        let mut moisture = FieldData::with_value(width, height, 0.1); // 10% moisture default
        let level_set = FieldData::with_value(width, height, f32::MAX); // All unburned initially
        let level_set_back = FieldData::new(width, height);
        let oxygen = FieldData::with_value(width, height, 0.21); // Atmospheric O₂ fraction
        let spread_rate = FieldData::new(width, height); // Computed from temperature

        // Initialize static fields
        let fuel_type = vec![0_u8; num_cells]; // Default fuel type

        // Phase 0: Initialize terrain slope/aspect from elevation data
        let terrain_fields = TerrainFields::from_terrain_data(terrain, width, height, cell_size);

        // Copy terrain height at grid resolution
        let mut terrain_height = vec![0.0_f32; num_cells];
        for y in 0..height {
            for x in 0..width {
                let wx = usize_to_f32(x) * cell_size;
                let wy = usize_to_f32(y) * cell_size;
                terrain_height[y * width + x] = *terrain.elevation_at(wx, wy);
            }
        }

        // Phase 2: Apply fuel heterogeneity for realistic spatial variation
        let heterogeneity_config = HeterogeneityConfig::default();
        let seed = 42_u64; // Deterministic seed for reproducibility
        let noise = super::noise::NoiseGenerator::new(seed);

        // Apply heterogeneity to fuel load and moisture fields
        let fuel_slice = fuel_load.as_mut_slice();
        let moisture_slice = moisture.as_mut_slice();
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let wx = usize_to_f32(x) * cell_size;
                let wy = usize_to_f32(y) * cell_size;

                // Get aspect for this cell (from terrain fields)
                let aspect = terrain_fields.aspect.get(x, y);

                // Apply heterogeneity to both fuel and moisture at once
                let (new_fuel, new_moisture) = super::fuel_variation::apply_heterogeneity_single(
                    fuel_slice[idx],
                    moisture_slice[idx],
                    aspect,
                    &noise,
                    &heterogeneity_config,
                    wx,
                    wy,
                );

                fuel_slice[idx] = new_fuel;
                moisture_slice[idx] = new_moisture;
            }
        }

        Self {
            temperature,
            temperature_back,
            fuel_load,
            moisture,
            level_set,
            level_set_back,
            oxygen,
            spread_rate,
            fuel_type,
            terrain_height,
            terrain_fields,
            heterogeneity_config,
            width,
            height,
            cell_size,
        }
    }
}

impl FieldSolver for CpuFieldSolver {
    fn step_heat_transfer(&mut self, dt: f32, wind_x: f32, wind_y: f32, ambient_temp: f32) {
        // Use Phase 2 heat transfer physics
        let params = HeatTransferParams {
            dt,
            wind_x,
            wind_y,
            ambient_temp,
            cell_size: self.cell_size,
        };

        step_heat_transfer_cpu(
            self.temperature.as_slice(),
            self.temperature_back.as_mut_slice(),
            self.level_set.as_slice(),
            self.fuel_load.as_slice(),
            self.width,
            self.height,
            params,
        );

        // Swap buffers
        std::mem::swap(&mut self.temperature, &mut self.temperature_back);
    }

    fn step_combustion(&mut self, dt: f32) {
        // Use Phase 2 combustion physics
        let params = CombustionParams {
            dt,
            cell_size: self.cell_size,
        };

        let heat_release = step_combustion_cpu(
            self.temperature.as_slice(),
            self.fuel_load.as_mut_slice(),
            self.moisture.as_mut_slice(),
            self.oxygen.as_mut_slice(),
            self.level_set.as_slice(),
            self.width,
            self.height,
            params,
        );

        // Add heat release to temperature field
        // Heat is converted to temperature rise via: ΔT = Q / (m × c)
        // Using simplified thermal mass: mass = fuel_load × cell_area, c = 1.0 kJ/(kg·K)
        let cell_area = self.cell_size * self.cell_size;
        let fuel_slice = self.fuel_load.as_slice();
        let temp_mut_slice = self.temperature.as_mut_slice();
        for (idx, &heat) in heat_release
            .iter()
            .enumerate()
            .take(self.width * self.height)
        {
            if heat > 0.0 {
                let fuel_mass = fuel_slice[idx] * cell_area;
                let thermal_mass = fuel_mass.max(0.1); // Minimum thermal mass to prevent inf
                let specific_heat = 1.5; // kJ/(kg·K) for wood
                let delta_t = heat / (thermal_mass * specific_heat * 1000.0);
                temp_mut_slice[idx] += delta_t;
            }
        }
    }

    fn step_moisture(&mut self, dt: f32, humidity: f32) {
        // Moisture equilibrium model (simplified Nelson 2000)
        // Moisture content tends toward equilibrium moisture content (EMC)
        // based on relative humidity over time

        // Calculate EMC from humidity (simplified)
        // EMC ≈ 0.85 × humidity for fine fuels
        let emc = 0.85 * humidity;

        // Time constant for moisture response (hours converted to seconds)
        // Fine fuels: ~1 hour, medium: ~10 hours
        let time_constant = 3600.0; // 1 hour in seconds

        // Exponential approach to EMC: dM/dt = (EMC - M) / τ
        let moisture_slice = self.moisture.as_mut_slice();
        let temp_slice = self.temperature.as_slice();
        let level_set_slice = self.level_set.as_slice();

        for idx in 0..(self.width * self.height) {
            let current_moisture = moisture_slice[idx];
            let temp = temp_slice[idx];
            let is_burning = level_set_slice[idx] < 0.0;

            // Burning cells: moisture continues to be driven off by combustion
            // (already handled in combustion step)
            if is_burning {
                continue;
            }

            // Hot cells dry out faster (temperature-dependent drying)
            // Drying rate increases exponentially with temperature above 100°C
            let drying_rate = if temp > 373.15 {
                // Above boiling: rapid drying
                let excess_temp = temp - 373.15;
                (1.0 + excess_temp / 100.0).min(10.0)
            } else {
                1.0
            };

            // Approach to EMC
            let rate = (emc - current_moisture) / time_constant * dt * drying_rate;
            moisture_slice[idx] = (current_moisture + rate).clamp(0.0, 1.0);
        }
    }

    fn step_level_set(&mut self, dt: f32) {
        // Phase 3: Level set evolution with curvature-dependent spread

        // First, compute spread rate from temperature gradient
        compute_spread_rate_cpu(
            self.temperature.as_slice(),
            self.fuel_load.as_slice(),
            self.moisture.as_slice(),
            self.spread_rate.as_mut_slice(),
            self.width,
            self.height,
            self.cell_size,
        );

        // Phase 0: Apply terrain slope factor to spread rate
        // Fire spreads faster uphill (McArthur 1967) and slower downhill
        let spread_slice = self.spread_rate.as_mut_slice();
        let level_set_slice = self.level_set.as_slice();
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                if spread_slice[idx] > 0.0 {
                    // Calculate fire spread direction from level set gradient
                    let spread_direction_degrees =
                        if x > 0 && x < self.width - 1 && y > 0 && y < self.height - 1 {
                            let phi_left = level_set_slice[idx - 1];
                            let phi_right = level_set_slice[idx + 1];
                            let phi_up = level_set_slice[idx - self.width];
                            let phi_down = level_set_slice[idx + self.width];

                            // Gradient points from burned to unburned (fire spreads in -∇φ direction)
                            let grad_x = (phi_right - phi_left) / (2.0 * self.cell_size);
                            let grad_y = (phi_down - phi_up) / (2.0 * self.cell_size);
                            let mag = (grad_x * grad_x + grad_y * grad_y).sqrt();
                            if mag > 1e-6 {
                                // Convert vector to degrees (0=North, 90=East)
                                // atan2(x, y) gives angle from North
                                let spread_x = -grad_x / mag;
                                let spread_y = -grad_y / mag;
                                let angle_rad = spread_x.atan2(spread_y);
                                let angle_deg = angle_rad.to_degrees();
                                if angle_deg < 0.0 {
                                    angle_deg + 360.0
                                } else {
                                    angle_deg
                                }
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        };

                    // Get slope and aspect at this cell using .get(x, y)
                    let slope = self.terrain_fields.slope.get(x, y);
                    let aspect = self.terrain_fields.aspect.get(x, y);

                    // Calculate effective slope projected onto spread direction
                    let effective_slope =
                        calculate_effective_slope(slope, aspect, spread_direction_degrees);

                    // Apply slope factor (McArthur 1967)
                    let slope_factor = calculate_slope_factor(effective_slope);
                    spread_slice[idx] *= slope_factor;
                }
            }
        }

        // Then evolve level set using spread rate
        let params = LevelSetParams {
            dt,
            cell_size: self.cell_size,
            curvature_coeff: 0.25, // Margerit 2002
            noise_amplitude: 0.05, // 5% stochastic variation
            time: 0.0,             // TODO: Track simulation time
        };

        step_level_set_cpu(
            self.level_set.as_slice(),
            self.level_set_back.as_mut_slice(),
            self.spread_rate.as_slice(),
            self.width,
            self.height,
            params,
        );

        // Swap buffers
        std::mem::swap(&mut self.level_set, &mut self.level_set_back);
    }

    fn step_ignition_sync(&mut self) {
        // Phase 3: Synchronize level set with temperature field
        let ignition_temp = 573.15; // ~300°C in Kelvin
        let moisture_extinction = 0.3; // 30% moisture prevents burning

        step_ignition_sync_cpu(
            self.level_set.as_mut_slice(),
            self.temperature.as_slice(),
            self.moisture.as_slice(),
            self.width,
            self.height,
            self.cell_size,
            ignition_temp,
            moisture_extinction,
        );
    }

    fn read_temperature(&self) -> Cow<'_, [f32]> {
        Cow::Borrowed(self.temperature.as_slice())
    }

    fn read_level_set(&self) -> Cow<'_, [f32]> {
        Cow::Borrowed(self.level_set.as_slice())
    }

    fn ignite_at(&mut self, x: f32, y: f32, radius: f32) {
        // Convert world coordinates to grid coordinates
        let grid_x = (x / self.cell_size) as i32;
        let grid_y = (y / self.cell_size) as i32;
        let grid_radius = (radius / self.cell_size) as i32;

        // Set φ < 0 in circular region and raise temperature
        for dy in -grid_radius..=grid_radius {
            for dx in -grid_radius..=grid_radius {
                let dist_sq = dx * dx + dy * dy;
                let radius_sq = grid_radius * grid_radius;

                if dist_sq <= radius_sq {
                    let gx = grid_x + dx;
                    let gy = grid_y + dy;

                    // Check bounds
                    if gx >= 0 && gx < self.width as i32 && gy >= 0 && gy < self.height as i32 {
                        let idx = (gy as usize) * self.width + (gx as usize);

                        // Set level set to negative (burning)
                        self.level_set.as_mut_slice()[idx] = -1.0;

                        // Set high temperature to initiate combustion
                        self.temperature.as_mut_slice()[idx] = 600.0; // ~327°C (ignition temp)
                    }
                }
            }
        }
    }

    fn dimensions(&self) -> (u32, u32, f32) {
        (self.width as u32, self.height as u32, self.cell_size)
    }

    fn is_gpu_accelerated(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_solver_creation() {
        let terrain = TerrainData::flat(1000.0, 1000.0, 10.0, 0.0);
        let solver = CpuFieldSolver::new(&terrain, QualityPreset::Medium);

        let (width, height, cell_size) = solver.dimensions();
        assert_eq!(width, 100);
        assert_eq!(height, 100);
        assert_eq!(cell_size, 10.0);
        assert!(!solver.is_gpu_accelerated());
    }

    #[test]
    fn test_cpu_solver_read_temperature() {
        let terrain = TerrainData::flat(100.0, 100.0, 10.0, 0.0);
        let solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        let temp = solver.read_temperature();
        assert!(!temp.is_empty());
        // Should be initialized to ambient temperature (~293.15 K)
        assert!(temp.iter().all(|&t| (t - 293.15).abs() < 0.1));
    }

    #[test]
    fn test_cpu_solver_read_level_set() {
        let terrain = TerrainData::flat(100.0, 100.0, 10.0, 0.0);
        let solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        let level_set = solver.read_level_set();
        assert!(!level_set.is_empty());
        // Should be initialized to MAX (all unburned)
        assert!(level_set.iter().all(|&phi| phi == f32::MAX));
    }

    #[test]
    fn test_cpu_solver_ignite_at() {
        let terrain = TerrainData::flat(1000.0, 1000.0, 10.0, 0.0);
        let mut solver = CpuFieldSolver::new(&terrain, QualityPreset::Medium);

        // Ignite at center with 50m radius
        solver.ignite_at(500.0, 500.0, 50.0);

        let level_set = solver.read_level_set();
        let temperature = solver.read_temperature();

        // Check that some cells are now burning (φ < 0)
        let burning_cells = level_set.iter().filter(|&&phi| phi < 0.0).count();
        assert!(burning_cells > 0, "No cells were ignited");

        // Check that some cells have elevated temperature
        let hot_cells = temperature.iter().filter(|&&t| t > 400.0).count();
        assert!(hot_cells > 0, "No cells were heated");
    }

    #[test]
    fn test_cpu_solver_heat_transfer() {
        let terrain = TerrainData::flat(100.0, 100.0, 10.0, 0.0);
        let mut solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        // Ignite a spot to create temperature gradient
        solver.ignite_at(50.0, 50.0, 10.0);

        let temp_before = solver.read_temperature().to_vec();

        // Run heat transfer step
        solver.step_heat_transfer(1.0, 0.0, 0.0, 293.15);

        let temp_after = solver.read_temperature();

        // Temperature field should have changed
        let changed = temp_before
            .iter()
            .zip(temp_after.iter())
            .any(|(&before, &after)| (before - after).abs() > 0.01);
        assert!(changed, "Temperature field did not change");
    }
}
