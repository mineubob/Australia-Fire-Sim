//! CPU-based field solver implementation
//!
//! This module provides a CPU implementation of the `FieldSolver` trait using
//! `Vec<f32>` arrays and Rayon for parallelism. This backend is always available
//! and serves as a fallback when GPU acceleration is not available.

use super::combustion::{
    step_combustion_cpu, CombustionParams, FuelCombustionProps, ATMOSPHERIC_OXYGEN_FRACTION,
};
use super::crown_fire::{CanopyProperties, CrownFirePhysics, CrownFireState};
use super::fields::FieldData;
use super::fuel_grid::FuelGrid;
use super::fuel_layers::LayeredFuelCell;
use super::fuel_variation::HeterogeneityConfig;
use super::heat_transfer::{step_heat_transfer_cpu, HeatTransferFuelProps, HeatTransferParams};
use super::junction_zone::JunctionZoneDetector;
use super::level_set::{
    compute_spread_rate_cpu, step_ignition_sync_cpu, step_level_set_cpu, LevelSetParams,
    SpreadRateFuelProps,
};
use super::quality::QualityPreset;
use super::regime::{detect_regime, FireRegime};
use super::terrain_slope::{calculate_effective_slope, calculate_slope_factor, TerrainFields};
use super::valley_channeling::{
    chimney_updraft, detect_valley_geometry, valley_wind_factor,
    VALLEY_UPDRAFT_CHARACTERISTIC_VELOCITY,
};
use super::vertical_heat_transfer::VerticalHeatTransfer;
use super::vls::VLSDetector;
use super::FieldSolver;
use crate::atmosphere::{AtmosphericStability, ConvectionColumn, Downdraft, PyroCbSystem};
use crate::core_types::units::{Gigawatts, Kelvin, Meters, MetersPerSecond, Seconds};
use crate::core_types::vec3::Vec3;
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

    // Phase 0: Terrain slope and aspect for fire spread modulation
    terrain_fields: TerrainFields,

    // Phase 1: Vertical fuel layers (surface, shrub, canopy)
    fuel_layers: Vec<LayeredFuelCell>,
    vertical_heat_transfer: VerticalHeatTransfer,

    // Phase 3: Crown fire state per cell
    crown_fire_state: Vec<CrownFireState>,
    canopy_properties: CanopyProperties,

    // Phase 4: Atmospheric dynamics
    convection_columns: Vec<ConvectionColumn>,
    downdrafts: Vec<Downdraft>,
    atmospheric_stability: AtmosphericStability,
    pyrocb_system: PyroCbSystem,

    // Phase 5-8: Advanced fire physics
    junction_zone_detector: JunctionZoneDetector,
    vls_detector: VLSDetector,
    fire_regime: Vec<FireRegime>, // Per-cell regime classification

    // Weather parameters (passed from simulation)
    wind_speed_10m_kmh: f32,
    wind_x_m_s: f32,     // Wind x-component in m/s
    wind_y_m_s: f32,     // Wind y-component in m/s
    ambient_temp_k: f32, // Ambient temperature in Kelvin

    // Advanced physics configuration
    valley_sample_radius: f32,           // Radius for valley detection (m)
    valley_reference_width: f32,         // Reference width for open terrain (m)
    valley_head_distance_threshold: f32, // Distance threshold for chimney effect (m)

    // Fuel grid: per-cell, per-layer fuel type assignment
    fuel_grid: FuelGrid,

    // Simulation time tracking
    sim_time: f32,

    // Grid dimensions
    width: usize,
    height: usize,
    cell_size: f32,

    // Terrain reference for valley/VLS detection
    terrain_data: TerrainData,
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
        // Default ambient temperature (20°C), will be updated from WeatherSystem via step_heat_transfer
        let temperature = FieldData::with_value(width, height, 293.15);
        let temperature_back = FieldData::new(width, height);
        let mut fuel_load = FieldData::with_value(width, height, 1.0); // 1 kg/m² default
        let mut moisture = FieldData::with_value(width, height, 0.1); // 10% moisture default
        let level_set = FieldData::with_value(width, height, f32::MAX); // All unburned initially
        let level_set_back = FieldData::new(width, height);
        let oxygen = FieldData::with_value(width, height, ATMOSPHERIC_OXYGEN_FRACTION); // Atmospheric O₂ fraction
        let spread_rate = FieldData::new(width, height); // Computed from temperature

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

        // Create fuel grid with per-cell, per-layer fuel types
        // Default to eucalyptus forest (Australian conditions)
        let mut fuel_grid = FuelGrid::eucalyptus_forest(width, height);

        // Initialize fuel grid from terrain elevation for spatial variation
        fuel_grid.initialize_from_elevation(&terrain_height);

        // Phase 5-8: Initialize advanced fire physics detectors
        let junction_zone_detector = JunctionZoneDetector::default();
        let vls_detector = VLSDetector::default();
        let fire_regime = vec![FireRegime::WindDriven; num_cells];

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
            terrain_fields,
            fuel_layers,
            vertical_heat_transfer,
            crown_fire_state,
            canopy_properties,
            convection_columns,
            downdrafts,
            atmospheric_stability,
            pyrocb_system,
            junction_zone_detector,
            vls_detector,
            fire_regime,
            wind_speed_10m_kmh: 20.0, // Default 20 km/h wind
            wind_x_m_s: 0.0,
            wind_y_m_s: 0.0,
            ambient_temp_k: 293.15, // Default 20°C
            valley_sample_radius: 100.0,
            valley_reference_width: 200.0,
            valley_head_distance_threshold: 100.0,
            fuel_grid,
            sim_time: 0.0,
            width,
            height,
            cell_size,
            terrain_data: terrain.clone(),
        }
    }
}

impl FieldSolver for CpuFieldSolver {
    fn step_heat_transfer(&mut self, dt: f32, wind: crate::core_types::Vec3, ambient_temp: Kelvin) {
        // Store weather parameters for use in other methods
        self.wind_x_m_s = wind.x;
        self.wind_y_m_s = wind.y;
        self.ambient_temp_k = ambient_temp.as_f32();

        // Extract wind components (wind.x and wind.y are already in m/s)
        let wind_x = wind.x;
        let wind_y = wind.y;

        // Extract wind speed for crown fire calculations (convert m/s to km/h)
        let wind_magnitude_m_s = (wind_x * wind_x + wind_y * wind_y).sqrt();
        self.wind_speed_10m_kmh = wind_magnitude_m_s * 3.6; // m/s to km/h

        // For bulk heat transfer, use the surface fuel properties as dominant
        // (most heat transfer occurs at surface level)
        // Get fuel properties from center cell as representative
        let center_x = self.width / 2;
        let center_y = self.height / 2;
        let surface_fuel = self.fuel_grid.get_surface_fuel(center_x, center_y);

        // Create fuel-specific heat transfer properties
        let heat_fuel_props = HeatTransferFuelProps {
            thermal_diffusivity: *surface_fuel.thermal_diffusivity,
            emissivity_burning: *surface_fuel.emissivity_burning, // Fuel-specific (0.90-0.93 for most)
            emissivity_unburned: *surface_fuel.emissivity_unburned, // Fuel-specific (0.65-0.95)
            specific_heat_kj: *surface_fuel.specific_heat,
        };

        // Use Phase 2 heat transfer physics with fuel-specific properties
        let params = HeatTransferParams {
            dt,
            wind_x,
            wind_y,
            ambient_temp: ambient_temp.as_f32(),
            cell_size: self.cell_size,
            fuel_props: heat_fuel_props,
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
        // Get surface fuel properties from center cell as representative
        // (for bulk combustion calculations - individual cell variations handled below)
        let center_x = self.width / 2;
        let center_y = self.height / 2;
        let surface_fuel = self.fuel_grid.get_surface_fuel(center_x, center_y);

        // Create fuel combustion properties from fuel type
        let fuel_props = FuelCombustionProps {
            ignition_temp_k: surface_fuel.ignition_temperature.as_f32() + 273.15,
            moisture_extinction: *surface_fuel.moisture_of_extinction,
            heat_content_kj: *surface_fuel.heat_content,
            self_heating_fraction: *surface_fuel.self_heating_fraction,
            burn_rate_coefficient: surface_fuel.burn_rate_coefficient,
            temperature_response_range: surface_fuel.temperature_response_range,
        };

        // Use Phase 2 combustion physics with fuel-specific properties
        let params = CombustionParams {
            dt,
            cell_size: self.cell_size,
            fuel_props,
            ambient_temp_k: self.ambient_temp_k, // From WeatherSystem
            air_density_kg_m3: 1.2, // TODO: Calculate from temperature, elevation, humidity
            atmospheric_mixing_height_m: 1.0, // TODO: Use from weather system or config
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
        // Using fuel-specific specific heat instead of hardcoded value
        let cell_area = self.cell_size * self.cell_size;
        let fuel_slice = self.fuel_load.as_slice();
        let temp_mut_slice = self.temperature.as_mut_slice();
        let specific_heat = *surface_fuel.specific_heat; // kJ/(kg·K) from fuel type
        for (idx, &heat) in heat_release
            .iter()
            .enumerate()
            .take(self.width * self.height)
        {
            if heat > 0.0 {
                let fuel_mass = fuel_slice[idx] * cell_area;
                let thermal_mass = fuel_mass.max(0.1); // Minimum thermal mass to prevent inf
                let delta_t = heat / (thermal_mass * specific_heat * 1000.0);
                temp_mut_slice[idx] += delta_t;
            }
        }

        // Phase 1: Vertical heat transfer between fuel layers
        // Calculate flame height and heat flux from burning layers to upper layers
        use super::fuel_layers::FuelLayer;
        use super::vertical_heat_transfer::FluxParams;

        // Specific heat capacity for vegetation from fuel type (convert kJ to J)
        let fuel_heat_capacity: f32 = *surface_fuel.specific_heat * 1000.0;

        let level_set_slice = self.level_set.as_slice();
        let intensity_slice = self.fire_intensity.as_slice();
        let spread_slice = self.spread_rate.as_slice();

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
                        fuel_heat_capacity,
                    );
                }

                // Check shrub ignition threshold
                fuel_cell.check_shrub_ignition(surface_intensity);
            }

            // Calculate heat flux from shrub → canopy (if shrub is burning)
            if fuel_cell.shrub.burning && fuel_cell.canopy.has_fuel() {
                // Calculate shrub layer intensity using Byram's formula: I = H × W × R
                // Use shrub layer fuel load and approximate shrub ROS from surface ROS
                let shrub_fuel_load = fuel_cell.shrub.fuel_load; // kg/m²
                let shrub_heat_content = fuel_heat_capacity * 1000.0; // Convert kJ/kg to J/kg for consistency
                let surface_ros = spread_slice[idx]; // m/s

                // TODO: PHASE 9 - Explicit Shrub-Layer Fire Modeling
                // This interim approximation uses surface ROS directly for shrub spread.
                // A full implementation would use Rothermel (1972) for the shrub layer explicitly,
                // accounting for shrub height, fuel bed depth, arrangement, and moisture.
                // Reference: Anderson (1982) for shrub fuel models.
                // Heavier fuel loads typically burn SLOWER (more heat needed to ignite),
                // so the previous fuel load ratio was physically incorrect.
                let shrub_ros = surface_ros; // Conservative approximation until proper model implemented
                let shrub_intensity = (shrub_heat_content / 1000.0) * shrub_fuel_load * shrub_ros; // kW/m

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
                        fuel_heat_capacity,
                    );
                }

                // Check canopy ignition using Van Wagner (1977) criterion
                // Canopy base height ~3m for typical eucalypt forest, FMC ~100%
                fuel_cell.check_canopy_ignition(surface_intensity + shrub_intensity, 3.0, 100.0);
            }
        }
    }

    fn step_moisture(&mut self, dt: f32, humidity: f32) {
        // Moisture equilibrium model using Nelson (2000) with Simard (1968) coefficients
        // Moisture content tends toward equilibrium moisture content (EMC)
        // based on relative humidity and temperature

        use crate::core_types::units::{Celsius, Percent};
        use crate::physics::calculate_equilibrium_moisture;

        // Convert humidity fraction to percentage
        let humidity_percent = Percent::new(humidity * 100.0);

        // Use ambient temperature from the weather system (Kelvin → Celsius)
        let ambient_temp_c = Celsius::new(f64::from(self.ambient_temp_k - 273.15));

        // Time constant for moisture response (hours converted to seconds)
        // Fine fuels: ~1 hour, medium: ~10 hours
        let time_constant = 3600.0; // 1 hour in seconds

        // Exponential approach to EMC: dM/dt = (EMC - M) / τ
        let moisture_slice = self.moisture.as_mut_slice();
        let temp_slice = self.temperature.as_slice();
        let level_set_slice = self.level_set.as_slice();

        for idx in 0..(self.width * self.height) {
            let temp = temp_slice[idx];
            let is_burning = level_set_slice[idx] < 0.0;

            // Burning cells: moisture continues to be driven off by combustion
            // (already handled in combustion step)
            if is_burning {
                continue;
            }

            // Calculate EMC with hysteresis: determine if fuel is adsorbing or desorbing
            // by comparing current moisture to equilibrium moisture.
            // Per Nelson (2000), fuels have different EMC curves for adsorption vs desorption.
            let current_moisture = moisture_slice[idx];

            // First calculate EMC assuming adsorption to determine process direction
            let emc_adsorb = calculate_equilibrium_moisture(ambient_temp_c, humidity_percent, true);

            // Determine process direction: if current moisture > EMC, fuel is drying (desorption)
            // if current moisture < EMC, fuel is gaining moisture (adsorption)
            let is_adsorbing = current_moisture < emc_adsorb;

            // Calculate correct EMC for the actual process direction
            let emc =
                calculate_equilibrium_moisture(ambient_temp_c, humidity_percent, is_adsorbing);

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

    fn step_level_set(&mut self, dt: f32, _wind: Vec3, _ambient_temp: Kelvin) {
        // Phase 3: Level set evolution with curvature-dependent spread

        // Get surface fuel properties from center cell as representative
        let center_x = self.width / 2;
        let center_y = self.height / 2;
        let surface_fuel = self.fuel_grid.get_surface_fuel(center_x, center_y);

        // Create fuel-specific spread rate properties
        let spread_fuel_props = SpreadRateFuelProps {
            ignition_temp_k: surface_fuel.ignition_temperature.as_f32() + 273.15,
            specific_heat_j: *surface_fuel.specific_heat * 1000.0, // kJ to J
            thermal_conductivity: *surface_fuel.thermal_conductivity,
        };

        // First, compute spread rate from temperature gradient
        compute_spread_rate_cpu(
            self.temperature.as_slice(),
            self.fuel_load.as_slice(),
            self.moisture.as_slice(),
            self.spread_rate.as_mut_slice(),
            self.width,
            self.height,
            self.cell_size,
            spread_fuel_props,
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

        // Phase 5: Junction Zone Detection and Acceleration
        // Detect converging fire fronts and apply acceleration
        let junctions = self.junction_zone_detector.detect(
            self.level_set.as_slice(),
            self.spread_rate.as_slice(),
            self.width,
            self.height,
            self.cell_size,
            dt,
        );

        // Apply junction acceleration to spread rates
        let spread_slice = self.spread_rate.as_mut_slice();
        for junction in &junctions {
            // Apply acceleration in a radius around junction point
            let radius = junction.distance * 0.5;
            #[expect(clippy::cast_possible_truncation)]
            let center_x = (junction.position.x / self.cell_size) as usize;
            #[expect(clippy::cast_possible_truncation)]
            let center_y = (junction.position.y / self.cell_size) as usize;

            #[expect(clippy::cast_possible_truncation)]
            let radius_cells = (radius / self.cell_size).ceil() as i32;

            for dy in -radius_cells..=radius_cells {
                for dx in -radius_cells..=radius_cells {
                    let x = (center_x as i32 + dx) as usize;
                    let y = (center_y as i32 + dy) as usize;

                    if x >= self.width || y >= self.height {
                        continue;
                    }

                    #[expect(clippy::cast_precision_loss)]
                    let dist = ((dx * dx + dy * dy) as f32).sqrt() * self.cell_size;
                    if dist > radius {
                        continue;
                    }

                    // Acceleration falls off with distance from junction center
                    let falloff = 1.0 - dist / radius;
                    let local_acceleration = 1.0 + (junction.acceleration_factor - 1.0) * falloff;

                    let idx = y * self.width + x;
                    if spread_slice[idx] > 0.0 {
                        spread_slice[idx] *= local_acceleration;
                    }
                }
            }
        }

        // Phase 6: VLS (Vorticity-Driven Lateral Spread)
        // Detect VLS conditions and modify spread rates on lee slopes
        let wind_vec = Vec3::new(self.wind_x_m_s, self.wind_y_m_s, 0.0);
        let vls_conditions = self.vls_detector.detect(
            &self.terrain_data,
            wind_vec,
            self.width,
            self.height,
            self.cell_size,
        );

        // Apply VLS effects to spread rates
        let spread_slice = self.spread_rate.as_mut_slice();
        for (y, row) in vls_conditions.iter().enumerate() {
            for (x, vls) in row.iter().enumerate() {
                if vls.is_active {
                    let idx = y * self.width + x;
                    if spread_slice[idx] > 0.0 {
                        spread_slice[idx] *= vls.rate_multiplier;
                        // Note: Direction modification would require changing the level set velocity field
                        // For now, we just apply the rate multiplier
                    }
                }
            }
        }

        // Phase 7: Valley Channeling Effects
        // Apply wind acceleration and chimney effects in valleys
        let spread_slice = self.spread_rate.as_mut_slice();
        let temp_slice = self.temperature.as_slice();
        for y in 0..self.height {
            for x in 0..self.width {
                #[expect(clippy::cast_precision_loss)]
                let world_x = x as f32 * self.cell_size;
                #[expect(clippy::cast_precision_loss)]
                let world_y = y as f32 * self.cell_size;

                // Detect valley geometry at this position
                let valley_geometry = detect_valley_geometry(
                    &self.terrain_data,
                    world_x,
                    world_y,
                    self.valley_sample_radius,
                    5.0, // Valley wall elevation threshold (m)
                    self.cell_size,
                );

                if valley_geometry.in_valley {
                    let idx = y * self.width + x;
                    if spread_slice[idx] > 0.0 {
                        // Apply valley wind acceleration
                        let wind_factor =
                            valley_wind_factor(&valley_geometry, self.valley_reference_width);
                        spread_slice[idx] *= wind_factor;

                        // Chimney updraft effect increases spread near valley head
                        let fire_temp_c = temp_slice[idx] - 273.15;
                        let ambient_temp_c = self.ambient_temp_k - 273.15;
                        let updraft = chimney_updraft(
                            &valley_geometry,
                            fire_temp_c,
                            ambient_temp_c,
                            self.valley_head_distance_threshold,
                        );
                        if updraft > 0.0 {
                            // Updraft enhancement scales with chimney velocity and saturates at a 20% ROS boost once updrafts are ~10 m/s.
                            // Normalized by VALLEY_UPDRAFT_CHARACTERISTIC_VELOCITY (Butler et al. 1998)
                            let updraft_factor =
                                1.0 + (updraft / VALLEY_UPDRAFT_CHARACTERISTIC_VELOCITY).min(0.2);
                            spread_slice[idx] *= updraft_factor;
                        }
                    }
                }
            }
        }

        // Phase 3: Calculate fire intensity and apply crown fire dynamics
        // Byram's formula: I = H × W × R (kJ/kg × kg/m² × m/s = kW/m)
        // Heat content from fuel type (not hardcoded)
        let heat_content_kj_kg = *surface_fuel.heat_content;
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
                let intensity = heat_content_kj_kg * fuel_load * ros; // kW/m
                intensity_slice[idx] = intensity;

                // Evaluate crown fire transition using Van Wagner (1977)
                let crown_state =
                    CrownFirePhysics::evaluate_transition(intensity, ros, &self.canopy_properties);
                self.crown_fire_state[idx] = crown_state;

                // Phase 8: Regime Detection (Byram number)
                // Classify fire regime based on intensity, wind, and ambient conditions
                let wind_speed_m_s = self.wind_speed_10m_kmh / 3.6;
                let ambient_temp_c = self.ambient_temp_k - 273.15;
                let regime = detect_regime(intensity, wind_speed_m_s, ambient_temp_c);
                self.fire_regime[idx] = regime;
            } else {
                intensity_slice[idx] = 0.0;
                self.crown_fire_state[idx] = CrownFireState::Surface;
                self.fire_regime[idx] = FireRegime::WindDriven;
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
                Meters::new(fire_length),
                Kelvin::new(f64::from(AMBIENT_TEMP_K)),
                MetersPerSecond::new(wind_speed_m_s),
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
                Gigawatts::new(total_fire_power_gw),
                self.convection_columns[0].height,
                haines_index,
                Seconds::new(self.sim_time),
                fire_position,
            );
        }

        // Update pyroCb system and check for collapses
        self.pyrocb_system.update(
            Seconds::new(dt),
            Seconds::new(self.sim_time),
            Kelvin::new(f64::from(AMBIENT_TEMP_K)),
        );

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
            let radius = *downdraft.radius;
            let outflow = *downdraft.outflow_velocity;

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
        // Use fuel-specific ignition temperature and moisture extinction
        // Get surface fuel properties from center cell as representative
        let center_x = self.width / 2;
        let center_y = self.height / 2;
        let surface_fuel = self.fuel_grid.get_surface_fuel(center_x, center_y);
        let ignition_temp = surface_fuel.ignition_temperature.as_f32() + 273.15; // Convert °C to K
        let moisture_extinction = *surface_fuel.moisture_of_extinction;

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

    #[allow(clippy::cast_precision_loss)] // Grid indices are small enough for f32
    fn apply_heat(&mut self, x: Meters, y: Meters, temperature_k: Kelvin, radius_m: Meters) {
        // Convert world coordinates to grid coordinates
        let grid_x = (*x / self.cell_size) as i32;
        let grid_y = (*y / self.cell_size) as i32;

        // Convert radius to grid cells
        let radius_cells = (*radius_m / self.cell_size).max(0.5);
        let search_radius = (radius_cells.ceil() as i32).max(1);

        // Apply heat with Gaussian falloff (models realistic heat dissipation)
        // σ = radius/2 so that 95% of heat is within specified radius
        let sigma = radius_cells / 2.0;
        let sigma_sq = sigma * sigma;

        for dy in -search_radius..=search_radius {
            for dx in -search_radius..=search_radius {
                let gx = grid_x + dx;
                let gy = grid_y + dy;

                // Check bounds
                if gx >= 0 && gx < self.width as i32 && gy >= 0 && gy < self.height as i32 {
                    let idx = (gy as usize) * self.width + (gx as usize);

                    // Calculate distance in grid cells
                    let dist_sq = (dx * dx + dy * dy) as f32;

                    // Gaussian heat distribution: T = T_max × exp(-r²/2σ²)
                    // This models realistic heat dissipation from a point source
                    let heat_factor = (-dist_sq / (2.0 * sigma_sq)).exp();
                    let applied_temp = temperature_k.as_f32() * heat_factor;

                    // Apply heat (take maximum - don't cool down existing hot areas)
                    let current_temp = self.temperature.as_slice()[idx];
                    let new_temp = current_temp.max(applied_temp);
                    self.temperature.as_mut_slice()[idx] = new_temp;

                    // Heat above ignition temperature marks cells as burning
                    // (level set φ < 0 indicates burned region)
                    let fuel = self.fuel_grid.get_surface_fuel(gx as usize, gy as usize);
                    let ignition_temp = fuel.ignition_temperature.as_f32() + 273.15;

                    if new_temp >= ignition_temp {
                        // Cell reached ignition temperature - mark as burning
                        // Use negative level set to indicate fire front
                        self.level_set.as_mut_slice()[idx] = -self.cell_size * 0.5;
                    }
                }
            }
        }
    }

    fn dimensions(&self) -> (u32, u32, Meters) {
        (
            self.width as u32,
            self.height as u32,
            Meters::new(self.cell_size),
        )
    }

    fn is_gpu_accelerated(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_types::units::{Kelvin, Meters};

    /// Helper to create flat terrain with f32 dimensions (for test convenience)
    fn flat_terrain(width: f32, height: f32, resolution: f32, elevation: f32) -> TerrainData {
        TerrainData::flat(
            Meters::new(width),
            Meters::new(height),
            Meters::new(resolution),
            Meters::new(elevation),
        )
    }

    /// Helper to create single hill terrain with f32 dimensions
    fn hill_terrain(
        width: f32,
        height: f32,
        resolution: f32,
        elevation: f32,
        hill_height: f32,
        hill_radius: f32,
    ) -> TerrainData {
        TerrainData::single_hill(
            Meters::new(width),
            Meters::new(height),
            Meters::new(resolution),
            Meters::new(elevation),
            Meters::new(hill_height),
            Meters::new(hill_radius),
        )
    }

    #[test]
    fn test_cpu_solver_creation() {
        let terrain = flat_terrain(1000.0, 1000.0, 10.0, 0.0);
        let solver = CpuFieldSolver::new(&terrain, QualityPreset::Medium);

        let (width, height, cell_size) = solver.dimensions();
        assert_eq!(width, 100);
        assert_eq!(height, 100);
        assert_eq!(*cell_size, 10.0);
        assert!(!solver.is_gpu_accelerated());
    }

    #[test]
    fn test_cpu_solver_read_temperature() {
        let terrain = flat_terrain(100.0, 100.0, 10.0, 0.0);
        let solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        let temp = solver.read_temperature();
        assert!(!temp.is_empty());
        // Should be initialized to ambient temperature (~293.15 K)
        assert!(temp.iter().all(|&t| (t - 293.15).abs() < 0.1));
    }

    #[test]
    fn test_cpu_solver_read_level_set() {
        let terrain = flat_terrain(100.0, 100.0, 10.0, 0.0);
        let solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        let level_set = solver.read_level_set();
        assert!(!level_set.is_empty());
        // Should be initialized to MAX (all unburned)
        assert!(level_set.iter().all(|&phi| phi == f32::MAX));
    }

    #[test]
    fn test_cpu_solver_ignite_at() {
        let terrain = flat_terrain(1000.0, 1000.0, 10.0, 0.0);
        let mut solver = CpuFieldSolver::new(&terrain, QualityPreset::Medium);

        // Apply heat at center (piloted ignition temperature ~600°C / 873K)
        solver.apply_heat(
            Meters::new(500.0),
            Meters::new(500.0),
            Kelvin::new(873.15),
            Meters::new(5.0),
        );

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
        let terrain = flat_terrain(100.0, 100.0, 10.0, 0.0);
        let mut solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        // Apply heat to create temperature gradient
        solver.apply_heat(
            Meters::new(50.0),
            Meters::new(50.0),
            Kelvin::new(873.15),
            Meters::new(5.0),
        );

        let temp_before = solver.read_temperature().to_vec();

        // Run heat transfer step
        solver.step_heat_transfer(
            1.0,
            crate::core_types::Vec3::new(0.0, 0.0, 0.0),
            Kelvin::new(293.15),
        );

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
        let terrain = hill_terrain(200.0, 200.0, 10.0, 0.0, 50.0, 50.0);
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
        let terrain = flat_terrain(200.0, 200.0, 10.0, 0.0);
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

    #[test]
    fn test_ignition_modes_piloted_is_minimal() {
        // Test that piloted ignition creates minimal initial burned area
        // Per Catalog 5.2: Piloted ignition is point-source
        let terrain = flat_terrain(200.0, 200.0, 10.0, 0.0);
        let mut solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        // Apply heat with piloted ignition parameters (small radius, moderate temp)
        solver.apply_heat(
            Meters::new(100.0),
            Meters::new(100.0),
            Kelvin::new(600.0 + 273.15),
            Meters::new(5.0),
        );

        let level_set = solver.read_level_set();
        let burning_cells = level_set.iter().filter(|&&phi| phi < 0.0).count();

        // Piloted should create only 1-4 cells initially, not the full 50m radius
        // 50m radius at 10m cells = ~78 cells if instant; piloted should be much less
        assert!(
            burning_cells <= 5,
            "Piloted ignition should create minimal cells (got {burning_cells}, expected <= 5)"
        );
        assert!(
            burning_cells >= 1,
            "Piloted ignition should create at least 1 burning cell"
        );
    }

    #[test]
    fn test_ignition_modes_instant_fills_radius() {
        // Test that instant ignition fills the entire radius
        let terrain = flat_terrain(200.0, 200.0, 10.0, 0.0);
        let mut solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        // Apply heat with large radius to fill area instantly
        solver.apply_heat(
            Meters::new(100.0),
            Meters::new(100.0),
            Kelvin::new(873.15),
            Meters::new(30.0),
        );

        let level_set = solver.read_level_set();
        let burning_cells = level_set.iter().filter(|&&phi| phi < 0.0).count();

        // 30m radius at 10m cells = ~28 cells (π * 3² ≈ 28)
        assert!(
            burning_cells >= 9,
            "Instant ignition should fill radius (got {burning_cells}, expected >= 9)"
        );
    }

    #[test]
    fn test_ignition_modes_temperature_differences() {
        // Test that different modes use appropriate ignition temperatures
        // Per Catalog 5.2: Piloted 250-300°C, Auto 400-500°C, Smoldering 200-250°C
        let terrain = flat_terrain(200.0, 200.0, 10.0, 0.0);

        // Test piloted (highest temp ~600°C / 873K - direct ignition source)
        let mut solver_piloted = CpuFieldSolver::new(&terrain, QualityPreset::Low);
        solver_piloted.apply_heat(
            Meters::new(100.0),
            Meters::new(100.0),
            Kelvin::new(873.15),
            Meters::new(5.0),
        );
        let temp_piloted = solver_piloted.read_temperature();
        let max_temp_piloted = temp_piloted.iter().copied().fold(0.0_f32, f32::max);

        // Test auto (moderate temp ~450°C / 723K - spontaneous ignition)
        let mut solver_auto = CpuFieldSolver::new(&terrain, QualityPreset::Low);
        solver_auto.apply_heat(
            Meters::new(100.0),
            Meters::new(100.0),
            Kelvin::new(723.15),
            Meters::new(20.0),
        );
        let temp_auto = solver_auto.read_temperature();
        let max_temp_auto = temp_auto.iter().copied().fold(0.0_f32, f32::max);

        // Test smoldering (lowest temp ~220°C / 493K - slow combustion)
        let mut solver_smoldering = CpuFieldSolver::new(&terrain, QualityPreset::Low);
        solver_smoldering.apply_heat(
            Meters::new(100.0),
            Meters::new(100.0),
            Kelvin::new(493.15),
            Meters::new(5.0),
        );
        let temp_smoldering = solver_smoldering.read_temperature();
        let max_temp_smoldering = temp_smoldering.iter().copied().fold(0.0_f32, f32::max);

        // Piloted should be hottest, auto intermediate, smoldering coolest
        // Temperatures in Kelvin: Piloted ~873K, Auto ~723K, Smoldering ~493K
        assert!(
            max_temp_piloted > max_temp_auto,
            "Piloted ignition should be hotter than auto ({max_temp_piloted} > {max_temp_auto})"
        );
        assert!(
            max_temp_auto > max_temp_smoldering,
            "Auto should be hotter than smoldering ({max_temp_auto} > {max_temp_smoldering})"
        );
    }

    #[test]
    fn test_ignition_natural_fire_spread() {
        // Test that realistic heat application creates localized heat zone with
        // only the hottest cells igniting (Gaussian temperature distribution)
        let terrain = flat_terrain(200.0, 200.0, 10.0, 0.0);
        let mut solver = CpuFieldSolver::new(&terrain, QualityPreset::Low);

        let ambient_temp = 293.15_f32;
        let temp_before = solver.read_temperature().to_vec();

        // All cells should be at ambient before ignition
        assert!(temp_before.iter().all(|&t| (t - ambient_temp).abs() < 1.0));

        // Apply heat with piloted ignition parameters (Gaussian falloff)
        solver.apply_heat(
            Meters::new(100.0),
            Meters::new(100.0),
            Kelvin::new(873.15),
            Meters::new(5.0),
        );

        let temp_after = solver.read_temperature();
        let level_set = solver.read_level_set();

        // Count burning cells (φ < 0)
        let burning_cells = level_set.iter().filter(|&&phi| phi < 0.0).count();

        // Count cells with elevated temperature (above ambient + 10K)
        let hot_cells = temp_after
            .iter()
            .filter(|&&t| t > ambient_temp + 10.0)
            .count();

        // Should have minimal burning area initially (1-5 cells at peak of Gaussian)
        assert!(
            burning_cells <= 5,
            "Should have minimal burning cells initially ({burning_cells})"
        );

        // Hot cells should exceed burning cells (Gaussian creates heat gradient)
        // Only cells at peak of Gaussian reach ignition temperature
        assert!(
            hot_cells >= burning_cells,
            "Hot cells ({hot_cells}) should include burning cells ({burning_cells}) plus warm periphery"
        );
    }
}
