//! CPU-based field solver implementation
//!
//! This module provides a CPU implementation of the `FieldSolver` trait using
//! `Vec<f32>` arrays and Rayon for parallelism. This backend is always available
//! and serves as a fallback when GPU acceleration is not available.

use super::combustion::{step_combustion_cpu, CombustionParams};
use super::crown_fire::{CanopyProperties, CrownFirePhysics, CrownFireState};
use super::fields::FieldData;
use super::fuel_layers::LayeredFuelCell;
use super::fuel_variation::HeterogeneityConfig;
use super::heat_transfer::{step_heat_transfer_cpu, HeatTransferParams};
use super::level_set::{
    compute_spread_rate_cpu, step_ignition_sync_cpu, step_level_set_cpu, LevelSetParams,
};
use super::quality::QualityPreset;
use super::terrain_slope::{calculate_effective_slope, calculate_slope_factor, TerrainFields};
use super::vertical_heat_transfer::VerticalHeatTransfer;
use super::FieldSolver;
use crate::atmosphere::{AtmosphericStability, ConvectionColumn, Downdraft, PyroCbSystem};
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

    // Fire intensity field (kW/m) for crown fire evaluation
    fire_intensity: FieldData,

    // Static fields (don't change during simulation)
    #[expect(dead_code)]
    fuel_type: Vec<u8>,
    #[expect(dead_code)]
    terrain_height: Vec<f32>,

    // Phase 0: Terrain slope and aspect for fire spread modulation
    terrain_fields: TerrainFields,

    // Phase 1: Vertical fuel layers (surface, shrub, canopy)
    fuel_layers: Vec<LayeredFuelCell>,
    vertical_heat_transfer: VerticalHeatTransfer,

    // Phase 2: Fuel heterogeneity configuration (stored for potential runtime queries)
    #[expect(dead_code)]
    heterogeneity_config: HeterogeneityConfig,

    // Phase 3: Crown fire state per cell
    crown_fire_state: Vec<CrownFireState>,
    canopy_properties: CanopyProperties,

    // Phase 4: Atmospheric dynamics
    convection_columns: Vec<ConvectionColumn>,
    downdrafts: Vec<Downdraft>,
    atmospheric_stability: AtmosphericStability,
    pyrocb_system: PyroCbSystem,

    // Weather parameters (for crown fire and atmosphere calculations)
    wind_speed_10m_kmh: f32,

    // Simulation time tracking
    sim_time: f32,

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

        // Phase 1: Initialize layered fuel cells (surface, shrub, canopy)
        let fuel_layers: Vec<LayeredFuelCell> =
            (0..num_cells).map(|_| LayeredFuelCell::default()).collect();
        let vertical_heat_transfer = VerticalHeatTransfer::default();

        // Phase 3: Initialize crown fire state (all surface initially)
        let crown_fire_state = vec![CrownFireState::default(); num_cells];
        let canopy_properties = CanopyProperties::eucalyptus_forest();

        // Phase 4: Initialize atmospheric systems
        let convection_columns = Vec::new();
        let downdrafts = Vec::new();
        let atmospheric_stability = AtmosphericStability::default();
        let pyrocb_system = PyroCbSystem::new();

        // Fire intensity field (computed during simulation)
        let fire_intensity = FieldData::new(width, height);

        Self {
            temperature,
            temperature_back,
            fuel_load,
            moisture,
            level_set,
            level_set_back,
            oxygen,
            spread_rate,
            fire_intensity,
            fuel_type,
            terrain_height,
            terrain_fields,
            fuel_layers,
            vertical_heat_transfer,
            heterogeneity_config,
            crown_fire_state,
            canopy_properties,
            convection_columns,
            downdrafts,
            atmospheric_stability,
            pyrocb_system,
            wind_speed_10m_kmh: 20.0, // Default 20 km/h wind
            sim_time: 0.0,
            width,
            height,
            cell_size,
        }
    }
}

impl FieldSolver for CpuFieldSolver {
    fn step_heat_transfer(&mut self, dt: f32, wind_x: f32, wind_y: f32, ambient_temp: f32) {
        // Extract wind speed for crown fire calculations (convert m/s to km/h)
        let wind_magnitude_m_s = (wind_x * wind_x + wind_y * wind_y).sqrt();
        self.wind_speed_10m_kmh = wind_magnitude_m_s * 3.6; // m/s to km/h

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

        // Phase 1: Vertical heat transfer between fuel layers
        // Calculate flame height and heat flux from burning layers to upper layers
        use super::fuel_layers::FuelLayer;
        use super::vertical_heat_transfer::FluxParams;

        // Specific heat capacity for vegetation (J/kg·K)
        const FUEL_HEAT_CAPACITY: f32 = 1500.0;

        let level_set_slice = self.level_set.as_slice();
        let intensity_slice = self.fire_intensity.as_slice();

        for idx in 0..(self.width * self.height) {
            // Only process burning cells
            if level_set_slice[idx] >= 0.0 {
                continue;
            }

            let fuel_cell = &mut self.fuel_layers[idx];

            // Mark surface as burning if cell is on fire
            if !fuel_cell.surface.burning && fuel_cell.surface.has_fuel() {
                fuel_cell.surface.burning = true;
                // Initialize surface temperature if just ignited
                if fuel_cell.surface.temperature < 600.0 {
                    fuel_cell.surface.temperature = 800.0; // Typical flame temperature
                }
            }

            // Calculate flame height from surface fire intensity using Byram's equation
            let surface_intensity = intensity_slice[idx];
            let flame_height = VerticalHeatTransfer::flame_height_byram(surface_intensity);

            // Create flux parameters
            // Canopy cover affects radiative view factor - denser canopy = less radiation reaches upper layers
            let canopy_cover = if fuel_cell.canopy.has_fuel() {
                0.6
            } else {
                0.0
            };
            let flux_params = FluxParams::new(flame_height, canopy_cover, params.dt);

            // Calculate heat flux from surface → shrub
            if fuel_cell.surface.burning && fuel_cell.shrub.has_fuel() {
                let flux_to_shrub = self.vertical_heat_transfer.calculate_flux(
                    &fuel_cell.surface,
                    FuelLayer::Surface,
                    &fuel_cell.shrub,
                    FuelLayer::Shrub,
                    &flux_params,
                );

                // Apply heat with moisture evaporation first
                if flux_to_shrub > 0.0 {
                    fuel_cell.shrub.heat_received += flux_to_shrub;
                    let heat_to_apply = fuel_cell.shrub.heat_received;
                    VerticalHeatTransfer::apply_heat_to_layer(
                        &mut fuel_cell.shrub,
                        heat_to_apply,
                        FUEL_HEAT_CAPACITY,
                    );
                }

                // Check shrub ignition threshold
                fuel_cell.check_shrub_ignition(surface_intensity);
            }

            // Calculate heat flux from shrub → canopy (if shrub is burning)
            if fuel_cell.shrub.burning && fuel_cell.canopy.has_fuel() {
                // Shrub layer intensity (simplified - proportional to fuel load and burning state)
                let shrub_intensity = surface_intensity * 0.5; // Shrub contributes additional intensity
                let shrub_flame_height = VerticalHeatTransfer::flame_height_byram(shrub_intensity);
                let shrub_flux_params = FluxParams::new(
                    flame_height + shrub_flame_height, // Combined flame height
                    canopy_cover,
                    params.dt,
                );

                let flux_to_canopy = self.vertical_heat_transfer.calculate_flux(
                    &fuel_cell.shrub,
                    FuelLayer::Shrub,
                    &fuel_cell.canopy,
                    FuelLayer::Canopy,
                    &shrub_flux_params,
                );

                if flux_to_canopy > 0.0 {
                    fuel_cell.canopy.heat_received += flux_to_canopy;
                    let heat_to_apply = fuel_cell.canopy.heat_received;
                    VerticalHeatTransfer::apply_heat_to_layer(
                        &mut fuel_cell.canopy,
                        heat_to_apply,
                        FUEL_HEAT_CAPACITY,
                    );
                }

                // Check canopy ignition using Van Wagner (1977) criterion
                // Canopy base height ~3m for typical eucalypt forest, FMC ~100%
                fuel_cell.check_canopy_ignition(surface_intensity + shrub_intensity, 3.0, 100.0);
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

        // Phase 3: Calculate fire intensity and apply crown fire dynamics
        // Byram's formula: I = H × W × R (kJ/kg × kg/m² × m/s = kW/m)
        // Default heat content for vegetation: ~18000 kJ/kg
        const HEAT_CONTENT_KJ_KG: f32 = 18000.0;
        let spread_slice = self.spread_rate.as_slice();
        let fuel_slice = self.fuel_load.as_slice();
        let level_set_slice = self.level_set.as_slice();
        let moisture_slice = self.moisture.as_slice();
        let intensity_slice = self.fire_intensity.as_mut_slice();

        for idx in 0..(self.width * self.height) {
            // Only calculate intensity for burning cells (level_set < 0)
            if level_set_slice[idx] < 0.0 && spread_slice[idx] > 0.0 {
                let fuel_load = fuel_slice[idx]; // kg/m²
                let ros = spread_slice[idx]; // m/s
                let intensity = HEAT_CONTENT_KJ_KG * fuel_load * ros; // kW/m
                intensity_slice[idx] = intensity;

                // Evaluate crown fire transition using Van Wagner (1977)
                let crown_state =
                    CrownFirePhysics::evaluate_transition(intensity, ros, &self.canopy_properties);
                self.crown_fire_state[idx] = crown_state;
            } else {
                intensity_slice[idx] = 0.0;
                self.crown_fire_state[idx] = CrownFireState::Surface;
            }
        }

        // Apply effective ROS based on crown fire state
        // This modifies spread rate for passive (1.5x) and active (crown ROS) fires
        let spread_slice = self.spread_rate.as_mut_slice();
        for idx in 0..(self.width * self.height) {
            if spread_slice[idx] > 0.0 {
                let crown_state = self.crown_fire_state[idx];
                let surface_ros = spread_slice[idx];
                let moisture = moisture_slice[idx];

                let effective = CrownFirePhysics::effective_ros(
                    surface_ros,
                    crown_state,
                    self.wind_speed_10m_kmh,
                    moisture,
                );
                spread_slice[idx] = effective;
            }
        }

        // Phase 4: Atmospheric dynamics
        // Update simulation time
        self.sim_time += dt;

        // Calculate total fire power (sum of intensities × fire front length)
        let intensity_slice = self.fire_intensity.as_slice();
        let level_set_slice = self.level_set.as_slice();

        // Find fire front (cells where level_set < 0) and sum intensity
        let mut total_intensity_kw = 0.0;
        let mut fire_center_x = 0.0;
        let mut fire_center_y = 0.0;
        let mut fire_cell_count = 0;

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                if level_set_slice[idx] < 0.0 && intensity_slice[idx] > 0.0 {
                    total_intensity_kw += intensity_slice[idx];
                    fire_center_x += usize_to_f32(x) * self.cell_size;
                    fire_center_y += usize_to_f32(y) * self.cell_size;
                    fire_cell_count += 1;
                }
            }
        }

        // Calculate fire center position
        let fire_position = if fire_cell_count > 0 {
            let count = usize_to_f32(fire_cell_count);
            (fire_center_x / count, fire_center_y / count)
        } else {
            (0.0, 0.0)
        };

        // Fire front length approximation (perimeter of burning area)
        let fire_length = usize_to_f32(fire_cell_count).sqrt() * self.cell_size * 4.0;

        // Convert total intensity to power (kW × m front length → MW)
        let total_fire_power_mw = total_intensity_kw * fire_length / 1000.0;
        let total_fire_power_gw = total_fire_power_mw / 1000.0;

        // Update convection columns from high-intensity fires
        const AMBIENT_TEMP_K: f32 = 300.0; // ~27°C
        let wind_speed_m_s = self.wind_speed_10m_kmh / 3.6;

        // Create/update main convection column if intensity is significant
        const COLUMN_INTENSITY_THRESHOLD: f32 = 1000.0; // kW/m
        if total_intensity_kw / fire_length.max(1.0) > COLUMN_INTENSITY_THRESHOLD
            && fire_cell_count > 0
        {
            let avg_intensity = total_intensity_kw / usize_to_f32(fire_cell_count);

            // Calculate plume height using Briggs formula (via ConvectionColumn)
            let column = ConvectionColumn::new(
                avg_intensity,
                fire_length,
                AMBIENT_TEMP_K,
                wind_speed_m_s,
                fire_position,
            );

            // Update or add main convection column
            if self.convection_columns.is_empty() {
                self.convection_columns.push(column);
            } else {
                // Update existing column
                self.convection_columns[0] = column;
            }

            // Check for pyroCb formation
            // Requires: >5 GW fire power, >8000m plume, Haines >= 5
            let haines_index = self.atmospheric_stability.haines_index;
            self.pyrocb_system.check_formation(
                total_fire_power_gw,
                self.convection_columns[0].height,
                haines_index,
                self.sim_time,
                fire_position,
            );
        }

        // Update pyroCb system and check for collapses
        self.pyrocb_system.update(dt, self.sim_time, AMBIENT_TEMP_K);

        // Collect downdrafts from pyroCb events
        self.downdrafts.clear();
        for event in &self.pyrocb_system.active_events {
            self.downdrafts.extend(event.downdrafts.clone());
        }

        // Apply downdraft effects to spread rate
        // Downdrafts create erratic local wind enhancements
        let spread_slice = self.spread_rate.as_mut_slice();
        for downdraft in &self.downdrafts {
            let (dx, dy) = downdraft.position;
            let radius = downdraft.radius;
            let outflow = downdraft.outflow_velocity;

            // Enhance spread rate in downdraft outflow region
            for y in 0..self.height {
                for x in 0..self.width {
                    let px = usize_to_f32(x) * self.cell_size;
                    let py = usize_to_f32(y) * self.cell_size;
                    let dist = ((px - dx).powi(2) + (py - dy).powi(2)).sqrt();

                    if dist < radius && spread_slice[y * self.width + x] > 0.0 {
                        // Outflow velocity enhancement (strongest at radius edge)
                        let radial_factor = dist / radius;
                        let enhancement = 1.0 + (outflow / 20.0) * radial_factor; // Cap at ~2x
                        spread_slice[y * self.width + x] *= enhancement;
                    }
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

    #[test]
    fn test_terrain_slope_integration() {
        // Create terrain with a hill - slope should be non-zero
        let terrain = TerrainData::single_hill(200.0, 200.0, 10.0, 0.0, 50.0, 50.0);
        let solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        // Verify terrain fields were initialized with non-zero slopes
        let (width, height) = solver.terrain_fields.dimensions();
        assert!(
            width > 0 && height > 0,
            "Terrain fields should be initialized"
        );

        // Check that some cells have non-zero slope (hill terrain)
        let mut has_slope = false;
        for y in 0..height {
            for x in 0..width {
                if solver.terrain_fields.slope.get(x, y) > 0.1 {
                    has_slope = true;
                    break;
                }
            }
        }
        assert!(
            has_slope,
            "Hill terrain should have cells with non-zero slope"
        );
    }

    #[test]
    fn test_fuel_heterogeneity_integration() {
        let terrain = TerrainData::flat(200.0, 200.0, 10.0, 0.0);
        let solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        // With heterogeneity applied, fuel load should vary across cells
        let fuel_slice = solver.fuel_load.as_slice();

        // Count unique fuel values (with some tolerance)
        let min_fuel = fuel_slice.iter().copied().fold(f32::INFINITY, f32::min);
        let max_fuel = fuel_slice.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        // Heterogeneity should create variation
        assert!(
            max_fuel - min_fuel > 0.01,
            "Fuel load should vary across cells (min={min_fuel}, max={max_fuel})"
        );
    }
}
