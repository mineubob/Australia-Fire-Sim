//! GPU-based field solver implementation
//!
//! This module provides a GPU implementation of the `FieldSolver` trait using
//! wgpu compute shaders and storage buffers. This backend is only available when the
//! `gpu` feature is enabled.
//!
//! # Shader Files
//!
//! GPU compute shaders are located in `shaders/`:
//! - `heat_transfer.wgsl` - Stefan-Boltzmann radiation, diffusion, wind advection
//! - `combustion.wgsl` - Fuel consumption, moisture evaporation, oxygen depletion
//! - `level_set.wgsl` - Level set evolution with curvature-dependent spread
//! - `ignition_sync.wgsl` - Temperature-based ignition synchronization
//! - `crown_fire.wgsl` - Phase 3: Crown fire transitions and spread enhancement
//! - `fuel_layers.wgsl` - Phase 1: Vertical fuel layer heat transfer
//! - `atmosphere_reduce.wgsl` - Phase 4: Parallel reduction for pyroCb metrics
//!
//! # Implementation
//!
//! This solver uses wgpu compute pipelines to dispatch physics shaders on GPU.
//! Data is stored in GPU storage buffers with ping-pong double-buffering for in-place
//! updates. Staging buffers handle CPU readback when needed for visualization.

use super::context::GpuContext;
use super::crown_fire::CanopyProperties;
use super::fuel_grid::{CellFuelTypes, FuelGrid};
use super::fuel_variation::HeterogeneityConfig;
use super::junction_zone::JunctionZoneDetector;
use super::quality::QualityPreset;
use super::regime::FireRegime;
use super::terrain_slope::TerrainFields;
use super::FieldSolver;
use crate::atmosphere::{AtmosphericStability, ConvectionColumn, Downdraft, PyroCbSystem};
use crate::core_types::units::{Gigawatts, Kelvin, Meters, MetersPerSecond, Seconds};
use crate::core_types::vec3::Vec3;
use crate::TerrainData;
use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;
use wgpu::util::DeviceExt;

/// Default ambient temperature for initialization (20°C = 293.15 K)
/// Actual ambient temperature comes from `WeatherSystem` via `step_heat_transfer`
const AMBIENT_TEMP_K: f32 = 293.15;

// Helper to convert usize to f32, centralizing the intentional precision loss
#[inline]
#[expect(clippy::cast_precision_loss)]
fn usize_to_f32(v: usize) -> f32 {
    v as f32
}

/// Heat transfer shader parameters (must match WGSL struct layout)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct HeatParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
    ambient_temp: f32,
    wind_x: f32,
    wind_y: f32,
    stefan_boltzmann: f32,
    // Fuel-specific properties
    thermal_diffusivity: f32, // m²/s (from Fuel type)
    emissivity_burning: f32,  // Flames emissivity (0.9 typical)
    emissivity_unburned: f32, // Fuel bed emissivity (0.7 typical)
    specific_heat_j: f32,     // J/(kg·K) (from Fuel type)
}

/// Combustion shader parameters (must match WGSL struct layout)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct CombustionParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
    // Fuel-specific properties
    ignition_temp_k: f32,       // Ignition temperature in Kelvin
    moisture_extinction: f32,   // Moisture of extinction (fraction)
    heat_content_kj: f32,       // Heat content in kJ/kg
    self_heating_fraction: f32, // Fraction of heat retained (0-1)
    burn_rate_coefficient: f32, // Base burn rate coefficient
    ambient_temp_k: f32,        // Ambient temperature in Kelvin (from WeatherSystem)
    _padding1: f32,
    _padding2: f32,
}

/// Level set shader parameters (must match WGSL struct layout)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct LevelSetParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
    curvature_coeff: f32,
    noise_amplitude: f32,
    time: f32,
    _padding: f32,
}

/// Ignition sync shader parameters (must match WGSL struct layout)
///
/// WGSL struct has `vec3<f32>` padding which requires 16-byte alignment.
/// Layout: 20 bytes of fields, then 12 bytes padding, then vec3 = 48 bytes total
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct IgnitionParams {
    width: u32,
    height: u32,
    cell_size: f32,
    ignition_temperature: f32,
    moisture_extinction: f32,
    _pad1: f32,         // Padding to align next vec3
    _pad2: f32,         // Padding to align next vec3
    _pad3: f32,         // Padding to align next vec3
    _padding: [f32; 4], // vec3 + struct end padding (16 bytes to match vec3 size in WGSL)
}

/// Crown fire shader parameters (must match WGSL struct layout)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct CrownFireParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
    // Canopy properties
    canopy_base_height: f32,
    canopy_bulk_density: f32,
    foliar_moisture: f32,
    canopy_cover_fraction: f32,
    canopy_fuel_load: f32,
    canopy_heat_content: f32,
    // Weather parameters
    wind_speed_10m_kmh: f32,
    surface_heat_content: f32,
    _padding: f32,
}

/// Fuel layer shader parameters (must match WGSL struct layout)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct FuelLayerParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
    emissivity: f32,
    convective_coeff_base: f32,
    flame_height: f32,
    canopy_cover_fraction: f32,
    fuel_specific_heat: f32,
    _padding1: f32,
    _padding2: f32,
    _padding3: f32,
}

/// Atmosphere reduction shader parameters (must match WGSL struct layout)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct AtmosphereParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
}

/// Fire metrics from atmosphere reduction (must match WGSL struct layout)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct FireMetrics {
    total_intensity: u32, // Fixed-point (×1000)
    weighted_x: u32,      // Fixed-point (×1000)
    weighted_y: u32,      // Fixed-point (×1000)
    cell_count: u32,
    max_intensity: u32, // Fixed-point (×1000)
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,
}

/// Advanced physics shader parameters (must match WGSL struct layout)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct AdvancedPhysicsParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
    wind_speed: f32,             // Wind speed (m/s)
    wind_direction: f32,         // Wind direction (degrees, 0=North, clockwise)
    ambient_temp: f32,           // Ambient temperature (°C)
    vls_threshold: f32,          // VLS index threshold (0.6 typical)
    min_slope_vls: f32,          // Minimum slope for VLS (20° typical)
    min_wind_vls: f32,           // Minimum wind for VLS (5 m/s typical)
    valley_sample_radius: f32,   // Radius for valley detection (100m typical)
    valley_reference_width: f32, // Open terrain reference width (200m typical)
    valley_head_distance: f32,   // Distance threshold for chimney effect (100m typical)
    _padding: f32,
}

/// GPU-based field solver using wgpu compute shaders
///
/// Uses compute pipelines to dispatch physics computations on the GPU.
/// Data is stored in GPU buffers with ping-pong double-buffering and
/// staging buffers for CPU readback.
pub struct GpuFieldSolver {
    // GPU handles
    device: wgpu::Device,
    queue: wgpu::Queue,

    // Grid dimensions
    width: u32,
    height: u32,
    cell_size: f32,

    // Storage buffers for field data (double-buffered where needed)
    temperature_a: wgpu::Buffer,
    temperature_b: wgpu::Buffer,
    fuel_load: wgpu::Buffer,
    moisture: wgpu::Buffer,
    oxygen: wgpu::Buffer,
    level_set_a: wgpu::Buffer,
    level_set_b: wgpu::Buffer,
    spread_rate_a: wgpu::Buffer,
    spread_rate_b: wgpu::Buffer,
    heat_release: wgpu::Buffer,

    // Phase 0: Terrain slope and aspect buffers for fire spread modulation
    slope: wgpu::Buffer,
    aspect: wgpu::Buffer,
    elevation: wgpu::Buffer, // Elevation for valley detection

    // Phase 1: Vertical fuel layer buffers (3 layers per cell)
    layer_fuel: wgpu::Buffer,     // kg/m² per layer
    layer_moisture: wgpu::Buffer, // fraction 0-1 per layer
    layer_temp: wgpu::Buffer,     // Kelvin per layer
    layer_burning: wgpu::Buffer,  // 0 or 1 per layer

    // Phase 3: Crown fire state buffers
    crown_state: wgpu::Buffer,    // CrownFireState enum as u32
    fire_intensity: wgpu::Buffer, // kW/m per cell

    // Phase 4: Atmosphere reduction metrics buffer
    fire_metrics: wgpu::Buffer,
    metrics_staging: wgpu::Buffer,

    // Staging buffers for CPU readback
    temperature_staging: wgpu::Buffer,
    level_set_staging: wgpu::Buffer,

    // Uniform buffers for shader parameters
    heat_params_buffer: wgpu::Buffer,
    combustion_params_buffer: wgpu::Buffer,
    level_set_params_buffer: wgpu::Buffer,
    ignition_params_buffer: wgpu::Buffer,
    crown_fire_params_buffer: wgpu::Buffer,
    fuel_layer_params_buffer: wgpu::Buffer,
    atmosphere_params_buffer: wgpu::Buffer,
    advanced_physics_params_buffer: wgpu::Buffer,

    // Compute pipelines
    heat_transfer_pipeline: wgpu::ComputePipeline,
    combustion_pipeline: wgpu::ComputePipeline,
    level_set_pipeline: wgpu::ComputePipeline,
    ignition_sync_pipeline: wgpu::ComputePipeline,
    crown_fire_pipeline: wgpu::ComputePipeline,
    fuel_layer_pipeline: wgpu::ComputePipeline,
    atmosphere_reduce_pipeline: wgpu::ComputePipeline,
    atmosphere_clear_pipeline: wgpu::ComputePipeline,
    advanced_physics_pipeline: wgpu::ComputePipeline,

    // Bind group layouts (needed for bind group creation)
    heat_bind_group_layout: wgpu::BindGroupLayout,
    combustion_bind_group_layout: wgpu::BindGroupLayout,
    level_set_bind_group_layout: wgpu::BindGroupLayout,
    ignition_bind_group_layout: wgpu::BindGroupLayout,
    crown_fire_bind_group_layout: wgpu::BindGroupLayout,
    fuel_layer_bind_group_layout: wgpu::BindGroupLayout,
    atmosphere_bind_group_layout: wgpu::BindGroupLayout,
    advanced_physics_bind_group_layout: wgpu::BindGroupLayout,

    // Ping-pong state (which buffer is current)
    temp_ping: bool,
    phi_ping: bool,
    spread_ping: bool,

    // Simulation time (for noise in level set)
    time: f32,

    // Phase 3: Crown fire canopy properties
    canopy_properties: CanopyProperties,

    // Fuel grid for per-cell, per-layer fuel type lookups
    fuel_grid: FuelGrid,

    // Phase 4: Atmospheric dynamics (CPU-side, requires global calculations)
    convection_columns: Vec<ConvectionColumn>,
    downdrafts: Vec<Downdraft>,
    atmospheric_stability: AtmosphericStability,
    pyrocb_system: PyroCbSystem,

    // Phase 5-8: Advanced fire physics (CPU-side calculations)
    junction_zone_detector: JunctionZoneDetector,
    fire_regime: Vec<FireRegime>,

    // Weather parameters for crown fire and advanced physics
    wind_speed_10m_kmh: f32,
    wind_x: f32,         // Wind x component (m/s)
    wind_y: f32,         // Wind y component (m/s)
    ambient_temp_k: f32, // Ambient temperature (K)

    // Advanced physics configuration
    valley_sample_radius: f32,           // Radius for valley detection (m)
    valley_reference_width: f32,         // Reference width for open terrain (m)
    valley_head_distance_threshold: f32, // Distance threshold for chimney effect (m)
}

impl GpuFieldSolver {
    /// Create a new GPU field solver
    ///
    /// Initializes GPU buffers, loads shaders, and creates compute pipelines.
    ///
    /// # Arguments
    ///
    /// * `context` - GPU context with device and queue
    /// * `terrain` - Terrain data for initialization
    /// * `quality` - Quality preset determining grid resolution
    ///
    /// # Returns
    ///
    /// New GPU field solver instance
    #[must_use]
    pub fn new(context: GpuContext, terrain: &TerrainData, quality: QualityPreset) -> Self {
        let (width, height, cell_size) = quality.grid_dimensions(terrain);
        let (device, queue, _adapter_info) = context.into_device_queue();

        let buffer_size = u64::from(width * height) * std::mem::size_of::<f32>() as u64;
        let num_cells = (width * height) as usize;

        // Phase 0: Initialize terrain fields from elevation data
        let terrain_fields =
            TerrainFields::from_terrain_data(terrain, width as usize, height as usize, cell_size);

        // Initialize field data
        // Default ambient temperature (20°C), will be updated from WeatherSystem via step_heat_transfer
        let ambient_temp: f32 = AMBIENT_TEMP_K;
        let initial_temp: Vec<f32> = vec![ambient_temp; num_cells];
        let mut initial_fuel: Vec<f32> = vec![2.0; num_cells]; // 2 kg/m²
        let mut initial_moisture: Vec<f32> = vec![0.15; num_cells]; // 15%
        let initial_oxygen: Vec<f32> = vec![0.21; num_cells]; // 21%
        let initial_phi: Vec<f32> = vec![1000.0; num_cells]; // Far from fire
        let initial_spread: Vec<f32> = vec![0.5; num_cells]; // 0.5 m/s base spread
        let zeros: Vec<f32> = vec![0.0; num_cells];

        // Phase 2: Apply fuel heterogeneity for realistic spatial variation
        let heterogeneity_config = HeterogeneityConfig::default();
        let seed = 42_u64; // Deterministic seed for reproducibility
        let noise = super::noise::NoiseGenerator::new(seed);

        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = y * width as usize + x;
                let wx = usize_to_f32(x) * cell_size;
                let wy = usize_to_f32(y) * cell_size;

                // Get aspect for this cell (from terrain fields)
                let aspect = terrain_fields.aspect.get(x, y);

                // Apply heterogeneity to both fuel and moisture
                let (new_fuel, new_moisture) = super::fuel_variation::apply_heterogeneity_single(
                    initial_fuel[idx],
                    initial_moisture[idx],
                    aspect,
                    &noise,
                    &heterogeneity_config,
                    wx,
                    wy,
                );

                initial_fuel[idx] = new_fuel;
                initial_moisture[idx] = new_moisture;
            }
        }

        // Create storage buffers
        let temperature_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Temperature A"),
            contents: bytemuck::cast_slice(&initial_temp),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let temperature_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Temperature B"),
            contents: bytemuck::cast_slice(&initial_temp),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let fuel_load = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fuel Load"),
            contents: bytemuck::cast_slice(&initial_fuel),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let moisture = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Moisture"),
            contents: bytemuck::cast_slice(&initial_moisture),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let oxygen = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Oxygen"),
            contents: bytemuck::cast_slice(&initial_oxygen),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let level_set_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Level Set A"),
            contents: bytemuck::cast_slice(&initial_phi),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let level_set_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Level Set B"),
            contents: bytemuck::cast_slice(&initial_phi),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let spread_rate_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spread Rate A"),
            contents: bytemuck::cast_slice(&initial_spread),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let spread_rate_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spread Rate B"),
            contents: bytemuck::cast_slice(&initial_spread),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let heat_release = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Heat Release"),
            contents: bytemuck::cast_slice(&zeros),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Phase 0: Create slope and aspect buffers from terrain fields
        let slope = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Slope"),
            contents: bytemuck::cast_slice(terrain_fields.slope.as_slice()),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let aspect = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Aspect"),
            contents: bytemuck::cast_slice(terrain_fields.aspect.as_slice()),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Copy terrain elevation at grid resolution for valley detection
        let mut terrain_elevation: Vec<f32> = vec![0.0; num_cells];
        for y in 0..height as usize {
            for x in 0..width as usize {
                let wx = usize_to_f32(x) * cell_size;
                let wy = usize_to_f32(y) * cell_size;
                terrain_elevation[y * width as usize + x] = *terrain.elevation_at(wx, wy);
            }
        }

        let elevation = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Elevation"),
            contents: bytemuck::cast_slice(&terrain_elevation),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Phase 1: Vertical fuel layer buffers (3 layers per cell: surface, shrub, canopy)
        let num_layer_values = num_cells * 3;
        let layer_buffer_size = u64::from(width * height) * 3 * std::mem::size_of::<f32>() as u64;

        // Initialize layer fuel (surface gets the heterogeneous fuel, others default)
        let mut initial_layer_fuel: Vec<f32> = vec![0.0; num_layer_values];
        let mut initial_layer_moisture: Vec<f32> = vec![0.15; num_layer_values];
        let initial_layer_temp: Vec<f32> = vec![ambient_temp; num_layer_values];
        let initial_layer_burning: Vec<u32> = vec![0; num_layer_values];

        for idx in 0..num_cells {
            // Surface layer (index 0): gets the heterogeneous fuel
            initial_layer_fuel[idx * 3] = initial_fuel[idx];
            initial_layer_moisture[idx * 3] = initial_moisture[idx];
            // Shrub layer (index 1): 0.5 kg/m² default
            initial_layer_fuel[idx * 3 + 1] = 0.5;
            initial_layer_moisture[idx * 3 + 1] = 0.12;
            // Canopy layer (index 2): 1.2 kg/m² default (eucalyptus)
            initial_layer_fuel[idx * 3 + 2] = 1.2;
            initial_layer_moisture[idx * 3 + 2] = 1.0; // 100% foliar moisture
        }

        let layer_fuel = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Layer Fuel"),
            contents: bytemuck::cast_slice(&initial_layer_fuel),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let layer_moisture = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Layer Moisture"),
            contents: bytemuck::cast_slice(&initial_layer_moisture),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let layer_temp = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Layer Temperature"),
            contents: bytemuck::cast_slice(&initial_layer_temp),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let layer_burning = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Layer Burning"),
            contents: bytemuck::cast_slice(&initial_layer_burning),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Phase 3: Crown fire state buffers
        let initial_crown_state: Vec<u32> = vec![0; num_cells]; // All surface fire initially
        let crown_state = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Crown State"),
            contents: bytemuck::cast_slice(&initial_crown_state),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let fire_intensity = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fire Intensity"),
            contents: bytemuck::cast_slice(&zeros),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        // Phase 4: Atmosphere reduction metrics buffer
        let initial_metrics = FireMetrics {
            total_intensity: 0,
            weighted_x: 0,
            weighted_y: 0,
            cell_count: 0,
            max_intensity: 0,
            _padding1: 0,
            _padding2: 0,
            _padding3: 0,
        };
        let fire_metrics = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fire Metrics"),
            contents: bytemuck::bytes_of(&initial_metrics),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let metrics_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Metrics Staging"),
            size: std::mem::size_of::<FireMetrics>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Ignore layer_buffer_size for now (used in future staging buffers if needed)
        let _ = layer_buffer_size;

        // Create staging buffers for CPU readback
        let temperature_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Temperature Staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let level_set_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Level Set Staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Default fuel type for GPU solver
        // Create a FuelGrid and initialize from terrain elevation
        let mut fuel_grid =
            FuelGrid::new(width as usize, height as usize, CellFuelTypes::default());
        fuel_grid.initialize_from_elevation(terrain.elevations.as_slice());

        // Get surface fuel from center cell for initial uniform parameters
        let center_x = (width / 2) as usize;
        let center_y = (height / 2) as usize;
        let surface_fuel = fuel_grid.get_surface_fuel(center_x, center_y);

        // Create uniform buffers
        let heat_params = HeatParams {
            width,
            height,
            cell_size,
            dt: 0.1,
            ambient_temp,
            wind_x: 0.0,
            wind_y: 0.0,
            stefan_boltzmann: 5.67e-8,
            thermal_diffusivity: *surface_fuel.thermal_diffusivity,
            emissivity_burning: 0.9,  // Flames have high emissivity
            emissivity_unburned: 0.7, // Fuel bed has lower emissivity
            specific_heat_j: *surface_fuel.specific_heat * 1000.0, // kJ to J
        };

        let heat_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Heat Params"),
            contents: bytemuck::bytes_of(&heat_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let combustion_params = CombustionParams {
            width,
            height,
            cell_size,
            dt: 0.1,
            ignition_temp_k: (*surface_fuel.ignition_temperature + 273.15) as f32,
            moisture_extinction: *surface_fuel.moisture_of_extinction,
            heat_content_kj: *surface_fuel.heat_content,
            self_heating_fraction: *surface_fuel.self_heating_fraction,
            burn_rate_coefficient: surface_fuel.burn_rate_coefficient,
            ambient_temp_k: AMBIENT_TEMP_K, // Default, updated via step_heat_transfer from WeatherSystem
            _padding1: 0.0,
            _padding2: 0.0,
        };

        let combustion_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Combustion Params"),
                contents: bytemuck::bytes_of(&combustion_params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let level_set_params = LevelSetParams {
            width,
            height,
            cell_size,
            dt: 0.1,
            curvature_coeff: 0.25,
            noise_amplitude: 0.1,
            time: 0.0,
            _padding: 0.0,
        };

        let level_set_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Level Set Params"),
                contents: bytemuck::bytes_of(&level_set_params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let ignition_params = IgnitionParams {
            width,
            height,
            cell_size,
            ignition_temperature: (*surface_fuel.ignition_temperature + 273.15) as f32,
            moisture_extinction: *surface_fuel.moisture_of_extinction,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
            _padding: [0.0; 4],
        };

        let ignition_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ignition Params"),
            contents: bytemuck::bytes_of(&ignition_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Phase 3: Crown fire parameters
        let canopy_properties = CanopyProperties::eucalyptus_forest();
        let crown_fire_params = CrownFireParams {
            width,
            height,
            cell_size,
            dt: 0.1,
            canopy_base_height: *canopy_properties.base_height,
            canopy_bulk_density: *canopy_properties.bulk_density,
            foliar_moisture: *canopy_properties.foliar_moisture,
            canopy_cover_fraction: *canopy_properties.cover_fraction,
            canopy_fuel_load: *canopy_properties.fuel_load,
            canopy_heat_content: *canopy_properties.heat_content,
            wind_speed_10m_kmh: 20.0,
            surface_heat_content: *surface_fuel.heat_content,
            _padding: 0.0,
        };

        let crown_fire_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Crown Fire Params"),
                contents: bytemuck::bytes_of(&crown_fire_params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Phase 1: Fuel layer parameters
        let fuel_layer_params = FuelLayerParams {
            width,
            height,
            cell_size,
            dt: 0.1,
            emissivity: 0.9,
            convective_coeff_base: 25.0,
            flame_height: 2.0,
            canopy_cover_fraction: *canopy_properties.cover_fraction,
            fuel_specific_heat: *surface_fuel.specific_heat * 1000.0, // kJ to J
            _padding1: 0.0,
            _padding2: 0.0,
            _padding3: 0.0,
        };

        let fuel_layer_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Fuel Layer Params"),
                contents: bytemuck::bytes_of(&fuel_layer_params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Phase 4: Atmosphere parameters
        let atmosphere_params = AtmosphereParams {
            width,
            height,
            cell_size,
            dt: 0.1,
        };

        let atmosphere_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Atmosphere Params"),
                contents: bytemuck::bytes_of(&atmosphere_params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Phase 5-8: Advanced physics parameters
        let advanced_physics_params = AdvancedPhysicsParams {
            width,
            height,
            cell_size,
            dt: 0.1,
            wind_speed: 5.56,              // 20 km/h default in m/s
            wind_direction: 0.0,           // North
            ambient_temp: 20.0,            // 20°C
            vls_threshold: 0.6,            // VLS activation threshold
            min_slope_vls: 20.0,           // 20° minimum slope for VLS
            min_wind_vls: 5.0,             // 5 m/s minimum wind for VLS
            valley_sample_radius: 100.0,   // 100m valley detection radius
            valley_reference_width: 200.0, // 200m open terrain reference width
            valley_head_distance: 100.0,   // 100m distance threshold for chimney effect
            _padding: 0.0,
        };

        let advanced_physics_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Advanced Physics Params"),
                contents: bytemuck::bytes_of(&advanced_physics_params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Load shaders using wgpu::include_wgsl! macro
        let heat_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/heat_transfer.wgsl"));

        let combustion_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/combustion.wgsl"));

        let level_set_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/level_set.wgsl"));

        let ignition_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/ignition_sync.wgsl"));

        // Phase 3: Crown fire shader
        let crown_fire_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/crown_fire.wgsl"));

        // Phase 1: Fuel layer shader
        let fuel_layer_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/fuel_layers.wgsl"));

        // Phase 4: Atmosphere reduction shader
        let atmosphere_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/atmosphere_reduce.wgsl"));

        // Phase 5-8: Advanced physics shader
        let advanced_physics_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/advanced_physics.wgsl"));

        // Create bind group layouts
        let heat_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Heat Bind Group Layout"),
                entries: &[
                    // temp_in (binding 0)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // fuel_load (binding 1)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // level_set (binding 2)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // temp_out (binding 3)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // params (binding 4)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let combustion_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Combustion Bind Group Layout"),
                entries: &[
                    // temperature (binding 0)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // fuel_load (binding 1)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // moisture (binding 2)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // oxygen (binding 3)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // level_set (binding 4)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // heat_release (binding 5)
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // params (binding 6)
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let level_set_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Level Set Bind Group Layout"),
                entries: &[
                    // params (binding 0)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // phi_in (binding 1)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // spread_rate (binding 2)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // phi_out (binding 3)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Phase 0: slope (binding 4)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Phase 0: aspect (binding 5)
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let ignition_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Ignition Bind Group Layout"),
                entries: &[
                    // params (binding 0)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // phi_in (binding 1)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // temperature (binding 2)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // moisture (binding 3)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // phi_out (binding 4)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Phase 3: Crown fire bind group layout
        let crown_fire_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Crown Fire Bind Group Layout"),
                entries: &[
                    // params (binding 0)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // level_set (binding 1)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // spread_rate_in (binding 2)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // fuel_load (binding 3)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // moisture (binding 4)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // spread_rate_out (binding 5)
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // crown_state (binding 6)
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // fire_intensity (binding 7)
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Phase 1: Fuel layer bind group layout
        let fuel_layer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Fuel Layer Bind Group Layout"),
                entries: &[
                    // params (binding 0)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // layer_fuel (binding 1)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // layer_moisture (binding 2)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // layer_temp (binding 3)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // layer_burning (binding 4)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // fire_intensity (binding 5)
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Phase 4: Atmosphere bind group layout
        let atmosphere_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Atmosphere Bind Group Layout"),
                entries: &[
                    // params (binding 0)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // level_set (binding 1)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // fire_intensity (binding 2)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // metrics (binding 3)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Phase 5-8: Advanced physics bind group layout
        let advanced_physics_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Advanced Physics Bind Group Layout"),
                entries: &[
                    // params (binding 0)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // elevation (binding 1)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // slope (binding 2)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // aspect (binding 3)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // temperature (binding 4)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // spread_rate (binding 5)
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create pipeline layouts
        let heat_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Heat Pipeline Layout"),
            bind_group_layouts: &[&heat_bind_group_layout],
            immediate_size: 0,
        });

        let combustion_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Combustion Pipeline Layout"),
                bind_group_layouts: &[&combustion_bind_group_layout],
                immediate_size: 0,
            });

        let level_set_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Level Set Pipeline Layout"),
                bind_group_layouts: &[&level_set_bind_group_layout],
                immediate_size: 0,
            });

        let ignition_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Ignition Pipeline Layout"),
                bind_group_layouts: &[&ignition_bind_group_layout],
                immediate_size: 0,
            });

        // Phase 3: Crown fire pipeline layout
        let crown_fire_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Crown Fire Pipeline Layout"),
                bind_group_layouts: &[&crown_fire_bind_group_layout],
                immediate_size: 0,
            });

        // Phase 1: Fuel layer pipeline layout
        let fuel_layer_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Fuel Layer Pipeline Layout"),
                bind_group_layouts: &[&fuel_layer_bind_group_layout],
                immediate_size: 0,
            });

        // Phase 4: Atmosphere pipeline layout
        let atmosphere_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Atmosphere Pipeline Layout"),
                bind_group_layouts: &[&atmosphere_bind_group_layout],
                immediate_size: 0,
            });

        // Phase 5-8: Advanced physics pipeline layout
        let advanced_physics_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Advanced Physics Pipeline Layout"),
                bind_group_layouts: &[&advanced_physics_bind_group_layout],
                immediate_size: 0,
            });

        // Create compute pipelines
        let heat_transfer_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Heat Transfer Pipeline"),
                layout: Some(&heat_pipeline_layout),
                module: &heat_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let combustion_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Combustion Pipeline"),
                layout: Some(&combustion_pipeline_layout),
                module: &combustion_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let level_set_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Level Set Pipeline"),
            layout: Some(&level_set_pipeline_layout),
            module: &level_set_shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let ignition_sync_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Ignition Sync Pipeline"),
                layout: Some(&ignition_pipeline_layout),
                module: &ignition_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        // Phase 3: Crown fire pipeline
        let crown_fire_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Crown Fire Pipeline"),
                layout: Some(&crown_fire_pipeline_layout),
                module: &crown_fire_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        // Phase 1: Fuel layer pipeline
        let fuel_layer_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Fuel Layer Pipeline"),
                layout: Some(&fuel_layer_pipeline_layout),
                module: &fuel_layer_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        // Phase 4: Atmosphere reduction pipeline
        let atmosphere_reduce_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Atmosphere Reduce Pipeline"),
                layout: Some(&atmosphere_pipeline_layout),
                module: &atmosphere_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        // Phase 4: Atmosphere clear pipeline (clears metrics before reduction)
        let atmosphere_clear_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Atmosphere Clear Pipeline"),
                layout: Some(&atmosphere_pipeline_layout),
                module: &atmosphere_shader,
                entry_point: Some("clear_metrics"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        // Phase 5-8: Advanced physics pipeline
        let advanced_physics_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Advanced Physics Pipeline"),
                layout: Some(&advanced_physics_pipeline_layout),
                module: &advanced_physics_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        Self {
            device,
            queue,
            width,
            height,
            cell_size,
            temperature_a,
            temperature_b,
            fuel_load,
            moisture,
            oxygen,
            level_set_a,
            level_set_b,
            spread_rate_a,
            spread_rate_b,
            heat_release,
            slope,
            aspect,
            elevation,
            layer_fuel,
            layer_moisture,
            layer_temp,
            layer_burning,
            crown_state,
            fire_intensity,
            fire_metrics,
            metrics_staging,
            temperature_staging,
            level_set_staging,
            heat_params_buffer,
            combustion_params_buffer,
            level_set_params_buffer,
            ignition_params_buffer,
            crown_fire_params_buffer,
            fuel_layer_params_buffer,
            atmosphere_params_buffer,
            advanced_physics_params_buffer,
            heat_transfer_pipeline,
            combustion_pipeline,
            level_set_pipeline,
            ignition_sync_pipeline,
            crown_fire_pipeline,
            fuel_layer_pipeline,
            atmosphere_reduce_pipeline,
            atmosphere_clear_pipeline,
            advanced_physics_pipeline,
            heat_bind_group_layout,
            combustion_bind_group_layout,
            level_set_bind_group_layout,
            ignition_bind_group_layout,
            crown_fire_bind_group_layout,
            fuel_layer_bind_group_layout,
            atmosphere_bind_group_layout,
            advanced_physics_bind_group_layout,
            temp_ping: true,
            phi_ping: true,
            spread_ping: true,
            time: 0.0,
            // Phase 3: Crown fire canopy properties
            canopy_properties,
            // Fuel grid for per-cell, per-layer fuel type lookups
            fuel_grid,
            // Phase 4: Atmospheric dynamics
            convection_columns: Vec::new(),
            downdrafts: Vec::new(),
            atmospheric_stability: AtmosphericStability::default(),
            pyrocb_system: PyroCbSystem::new(),
            // Phase 5-8: Advanced fire physics
            junction_zone_detector: JunctionZoneDetector::default(),
            fire_regime: vec![FireRegime::WindDriven; num_cells],
            wind_speed_10m_kmh: 20.0,
            wind_x: 0.0,
            wind_y: 5.56,           // 20 km/h north wind
            ambient_temp_k: 293.15, // 20°C default
            valley_sample_radius: 100.0,
            valley_reference_width: 200.0,
            valley_head_distance_threshold: 100.0,
        }
    }

    /// Calculate workgroup count for dispatch
    fn workgroup_count(&self) -> (u32, u32) {
        // Workgroup size is 16x16 (defined in shaders)
        let workgroup_size = 16u32;
        let x = self.width.div_ceil(workgroup_size);
        let y = self.height.div_ceil(workgroup_size);
        (x, y)
    }

    /// Dispatch Phase 3: Crown fire shader
    fn dispatch_crown_fire(&mut self, dt: f32) {
        // Update crown fire params
        let params = CrownFireParams {
            width: self.width,
            height: self.height,
            cell_size: self.cell_size,
            dt,
            canopy_base_height: *self.canopy_properties.base_height,
            canopy_bulk_density: *self.canopy_properties.bulk_density,
            foliar_moisture: *self.canopy_properties.foliar_moisture,
            canopy_cover_fraction: *self.canopy_properties.cover_fraction,
            canopy_fuel_load: *self.canopy_properties.fuel_load,
            canopy_heat_content: *self.canopy_properties.heat_content,
            wind_speed_10m_kmh: self.wind_speed_10m_kmh,
            // Get surface fuel from center cell for uniform params
            surface_heat_content: {
                let center_x = (self.width / 2) as usize;
                let center_y = (self.height / 2) as usize;
                *self
                    .fuel_grid
                    .get_surface_fuel(center_x, center_y)
                    .heat_content
            },
            _padding: 0.0,
        };
        self.queue.write_buffer(
            &self.crown_fire_params_buffer,
            0,
            bytemuck::bytes_of(&params),
        );

        let bind_group = self.create_crown_fire_bind_group();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Crown Fire Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Crown Fire Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.crown_fire_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let (wg_x, wg_y) = self.workgroup_count();
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Flip spread rate ping-pong
        self.spread_ping = !self.spread_ping;
    }

    /// Dispatch Phase 1: Fuel layer shader (vertical heat transfer)
    fn dispatch_fuel_layers(&mut self, dt: f32) {
        // Update fuel layer params
        let params = FuelLayerParams {
            width: self.width,
            height: self.height,
            cell_size: self.cell_size,
            dt,
            emissivity: 0.9,
            convective_coeff_base: 25.0,
            flame_height: 2.0, // Could be calculated from fire intensity
            canopy_cover_fraction: *self.canopy_properties.cover_fraction,
            // Get surface fuel from center cell for uniform params
            fuel_specific_heat: {
                let center_x = (self.width / 2) as usize;
                let center_y = (self.height / 2) as usize;
                *self
                    .fuel_grid
                    .get_surface_fuel(center_x, center_y)
                    .specific_heat
                    * 1000.0
            }, // kJ to J
            _padding1: 0.0,
            _padding2: 0.0,
            _padding3: 0.0,
        };
        self.queue.write_buffer(
            &self.fuel_layer_params_buffer,
            0,
            bytemuck::bytes_of(&params),
        );

        let bind_group = self.create_fuel_layer_bind_group();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Fuel Layer Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Fuel Layer Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.fuel_layer_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let (wg_x, wg_y) = self.workgroup_count();
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Dispatch Phase 4: Atmosphere reduction and update CPU state
    fn dispatch_atmosphere(&mut self, dt: f32) {
        // Update atmosphere params
        let params = AtmosphereParams {
            width: self.width,
            height: self.height,
            cell_size: self.cell_size,
            dt,
        };
        self.queue.write_buffer(
            &self.atmosphere_params_buffer,
            0,
            bytemuck::bytes_of(&params),
        );

        let bind_group = self.create_atmosphere_bind_group();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Atmosphere Encoder"),
            });

        // First pass: Clear metrics
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Atmosphere Clear Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.atmosphere_clear_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }

        // Second pass: Reduction
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Atmosphere Reduce Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.atmosphere_reduce_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let (wg_x, wg_y) = self.workgroup_count();
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // Copy metrics to staging
        encoder.copy_buffer_to_buffer(
            &self.fire_metrics,
            0,
            &self.metrics_staging,
            0,
            std::mem::size_of::<FireMetrics>() as u64,
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Read back metrics
        let buffer_slice = self.metrics_staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let metrics: FireMetrics = *bytemuck::from_bytes(&data);
        drop(data);
        self.metrics_staging.unmap();

        // Convert fixed-point metrics back to floats
        // These casts are intentional - metrics values are within f32 precision range
        #[expect(clippy::cast_precision_loss)]
        let total_intensity = metrics.total_intensity as f32 / 1000.0;
        let cell_count = metrics.cell_count as usize;

        if cell_count > 0 {
            #[expect(clippy::cast_precision_loss)]
            let fire_center_x = (metrics.weighted_x as f32 / 1000.0) / total_intensity;
            #[expect(clippy::cast_precision_loss)]
            let fire_center_y = (metrics.weighted_y as f32 / 1000.0) / total_intensity;
            let fire_position = (fire_center_x, fire_center_y);

            let fire_length = usize_to_f32(cell_count).sqrt() * self.cell_size * 4.0;
            let total_fire_power_gw = total_intensity * fire_length / 1_000_000.0;

            const AMBIENT_TEMP_K: f32 = 300.0;
            let wind_speed_m_s = self.wind_speed_10m_kmh / 3.6;

            let avg_intensity = total_intensity / usize_to_f32(cell_count);

            // Create/update convection column
            let column = ConvectionColumn::new(
                avg_intensity,
                Meters::new(fire_length),
                Kelvin::new(f64::from(AMBIENT_TEMP_K)),
                MetersPerSecond::new(wind_speed_m_s),
                fire_position,
            );

            if self.convection_columns.is_empty() {
                self.convection_columns.push(column);
            } else {
                self.convection_columns[0] = column;
            }

            // Check for pyroCb formation
            let haines_index = self.atmospheric_stability.haines_index;
            self.pyrocb_system.check_formation(
                Gigawatts::new(total_fire_power_gw),
                self.convection_columns[0].height,
                haines_index,
                Seconds::new(self.time),
                fire_position,
            );
        }

        // Update pyroCb system
        self.pyrocb_system.update(
            Seconds::new(dt),
            Seconds::new(self.time),
            Kelvin::new(300.0),
        );

        // Collect downdrafts from pyroCb events
        self.downdrafts.clear();
        for event in &self.pyrocb_system.active_events {
            self.downdrafts.extend(event.downdrafts.clone());
        }
    }

    /// Dispatch Phase 5-8: Advanced physics shader (VLS and valley channeling)
    fn dispatch_advanced_physics(&mut self, dt: f32, wind_x: f32, wind_y: f32) {
        // Calculate wind direction from components
        let wind_speed = (wind_x * wind_x + wind_y * wind_y).sqrt();
        let wind_direction = if wind_speed > 0.1 {
            // atan2(x, y) gives angle from North (0=North, 90=East)
            let angle_rad = wind_x.atan2(wind_y);
            let angle_deg = angle_rad.to_degrees();
            if angle_deg < 0.0 {
                angle_deg + 360.0
            } else {
                angle_deg
            }
        } else {
            0.0 // Default to North if no wind
        };

        // Update advanced physics params
        let params = AdvancedPhysicsParams {
            width: self.width,
            height: self.height,
            cell_size: self.cell_size,
            dt,
            wind_speed,
            wind_direction,
            ambient_temp: self.ambient_temp_k - 273.15, // Convert K to °C
            vls_threshold: 0.6,
            min_slope_vls: 20.0,
            min_wind_vls: 5.0,
            valley_sample_radius: self.valley_sample_radius,
            valley_reference_width: self.valley_reference_width,
            valley_head_distance: self.valley_head_distance_threshold,
            _padding: 0.0,
        };
        self.queue.write_buffer(
            &self.advanced_physics_params_buffer,
            0,
            bytemuck::bytes_of(&params),
        );

        let bind_group = self.create_advanced_physics_bind_group();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Advanced Physics Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Advanced Physics Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.advanced_physics_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let (wg_x, wg_y) = self.workgroup_count();
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Create heat transfer bind group for current ping-pong state
    fn create_heat_bind_group(&self) -> wgpu::BindGroup {
        let (temp_in, temp_out) = if self.temp_ping {
            (&self.temperature_a, &self.temperature_b)
        } else {
            (&self.temperature_b, &self.temperature_a)
        };

        let phi = if self.phi_ping {
            &self.level_set_a
        } else {
            &self.level_set_b
        };

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Heat Bind Group"),
            layout: &self.heat_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: temp_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.fuel_load.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: phi.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: temp_out.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.heat_params_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Create combustion bind group
    fn create_combustion_bind_group(&self) -> wgpu::BindGroup {
        let temp = if self.temp_ping {
            &self.temperature_a
        } else {
            &self.temperature_b
        };

        let phi = if self.phi_ping {
            &self.level_set_a
        } else {
            &self.level_set_b
        };

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Combustion Bind Group"),
            layout: &self.combustion_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: temp.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.fuel_load.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.moisture.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.oxygen.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: phi.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.heat_release.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: self.combustion_params_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Create level set bind group for current ping-pong state
    fn create_level_set_bind_group(&self) -> wgpu::BindGroup {
        let (phi_in, phi_out) = if self.phi_ping {
            (&self.level_set_a, &self.level_set_b)
        } else {
            (&self.level_set_b, &self.level_set_a)
        };

        // Use current spread rate buffer (modified by crown fire shader)
        let spread_rate = if self.spread_ping {
            &self.spread_rate_a
        } else {
            &self.spread_rate_b
        };

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Level Set Bind Group"),
            layout: &self.level_set_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.level_set_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: phi_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: spread_rate.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: phi_out.as_entire_binding(),
                },
                // Phase 0: Terrain slope buffer
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.slope.as_entire_binding(),
                },
                // Phase 0: Terrain aspect buffer
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.aspect.as_entire_binding(),
                },
            ],
        })
    }

    /// Create ignition sync bind group
    fn create_ignition_bind_group(&self) -> wgpu::BindGroup {
        let temp = if self.temp_ping {
            &self.temperature_a
        } else {
            &self.temperature_b
        };

        let (phi_in, phi_out) = if self.phi_ping {
            (&self.level_set_a, &self.level_set_b)
        } else {
            (&self.level_set_b, &self.level_set_a)
        };

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ignition Bind Group"),
            layout: &self.ignition_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.ignition_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: phi_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: temp.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.moisture.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: phi_out.as_entire_binding(),
                },
            ],
        })
    }

    /// Create crown fire bind group for Phase 3
    fn create_crown_fire_bind_group(&self) -> wgpu::BindGroup {
        let phi = if self.phi_ping {
            &self.level_set_a
        } else {
            &self.level_set_b
        };

        let (spread_in, spread_out) = if self.spread_ping {
            (&self.spread_rate_a, &self.spread_rate_b)
        } else {
            (&self.spread_rate_b, &self.spread_rate_a)
        };

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Crown Fire Bind Group"),
            layout: &self.crown_fire_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.crown_fire_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: phi.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: spread_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.fuel_load.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.moisture.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: spread_out.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: self.crown_state.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: self.fire_intensity.as_entire_binding(),
                },
            ],
        })
    }

    /// Create fuel layer bind group for Phase 1
    fn create_fuel_layer_bind_group(&self) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fuel Layer Bind Group"),
            layout: &self.fuel_layer_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.fuel_layer_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.layer_fuel.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.layer_moisture.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.layer_temp.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.layer_burning.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.fire_intensity.as_entire_binding(),
                },
            ],
        })
    }

    /// Create atmosphere bind group for Phase 4
    fn create_atmosphere_bind_group(&self) -> wgpu::BindGroup {
        let phi = if self.phi_ping {
            &self.level_set_a
        } else {
            &self.level_set_b
        };

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Atmosphere Bind Group"),
            layout: &self.atmosphere_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.atmosphere_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: phi.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.fire_intensity.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.fire_metrics.as_entire_binding(),
                },
            ],
        })
    }

    /// Create advanced physics bind group for Phase 5-8
    fn create_advanced_physics_bind_group(&self) -> wgpu::BindGroup {
        let temp = if self.temp_ping {
            &self.temperature_a
        } else {
            &self.temperature_b
        };

        let spread = if self.spread_ping {
            &self.spread_rate_a
        } else {
            &self.spread_rate_b
        };

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Advanced Physics Bind Group"),
            layout: &self.advanced_physics_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.advanced_physics_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.elevation.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.slope.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.aspect.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: temp.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: spread.as_entire_binding(),
                },
            ],
        })
    }

    /// Read spread rate buffer from GPU (for CPU-side junction zone processing)
    fn read_spread_rate(&self) -> Vec<f32> {
        let buffer_size = u64::from(self.width * self.height) * std::mem::size_of::<f32>() as u64;

        // Create temporary staging buffer
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Spread Rate Staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy current spread rate buffer to staging
        let src_buffer = if self.spread_ping {
            &self.spread_rate_a
        } else {
            &self.spread_rate_b
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Spread Rate Readback Encoder"),
            });

        encoder.copy_buffer_to_buffer(src_buffer, 0, &staging, 0, buffer_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map and read
        let buffer_slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging.unmap();

        result
    }

    /// Write spread rate buffer to GPU (after CPU-side junction zone processing)
    fn write_spread_rate(&mut self, data: &[f32]) {
        let dst_buffer = if self.spread_ping {
            &self.spread_rate_a
        } else {
            &self.spread_rate_b
        };
        self.queue
            .write_buffer(dst_buffer, 0, bytemuck::cast_slice(data));
    }

    /// Read fire intensity buffer from GPU (for CPU-side regime detection)
    fn read_fire_intensity(&self) -> Vec<f32> {
        let buffer_size = u64::from(self.width * self.height) * std::mem::size_of::<f32>() as u64;

        // Create temporary staging buffer
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fire Intensity Staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Fire Intensity Readback Encoder"),
            });

        encoder.copy_buffer_to_buffer(&self.fire_intensity, 0, &staging, 0, buffer_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map and read
        let buffer_slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging.unmap();

        result
    }
}

impl FieldSolver for GpuFieldSolver {
    fn step_heat_transfer(&mut self, dt: f32, wind: crate::core_types::Vec3, ambient_temp: Kelvin) {
        // Extract wind components (wind.x and wind.y are already in m/s)
        let wind_x = wind.x;
        let wind_y = wind.y;

        // Store weather parameters for use in other methods
        let wind_magnitude_m_s = (wind_x * wind_x + wind_y * wind_y).sqrt();
        self.wind_speed_10m_kmh = wind_magnitude_m_s * 3.6;
        self.wind_x = wind_x;
        self.wind_y = wind_y;
        self.ambient_temp_k = ambient_temp.as_f32();

        // Update uniform buffer with new parameters
        let params = HeatParams {
            width: self.width,
            height: self.height,
            cell_size: self.cell_size,
            dt,
            ambient_temp: ambient_temp.as_f32(),
            wind_x,
            wind_y,
            stefan_boltzmann: 5.67e-8,
            // Get surface fuel from center cell for uniform params
            thermal_diffusivity: {
                let center_x = (self.width / 2) as usize;
                let center_y = (self.height / 2) as usize;
                *self
                    .fuel_grid
                    .get_surface_fuel(center_x, center_y)
                    .thermal_diffusivity
            },
            emissivity_burning: 0.9,
            emissivity_unburned: 0.7,
            specific_heat_j: {
                let center_x = (self.width / 2) as usize;
                let center_y = (self.height / 2) as usize;
                *self
                    .fuel_grid
                    .get_surface_fuel(center_x, center_y)
                    .specific_heat
                    * 1000.0
            },
        };
        self.queue
            .write_buffer(&self.heat_params_buffer, 0, bytemuck::bytes_of(&params));

        // Create bind group for current ping-pong state
        let bind_group = self.create_heat_bind_group();

        // Create command encoder and dispatch
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Heat Transfer Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Heat Transfer Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.heat_transfer_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let (wg_x, wg_y) = self.workgroup_count();
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Flip ping-pong
        self.temp_ping = !self.temp_ping;
    }

    fn step_combustion(&mut self, dt: f32) {
        // Update uniform buffer
        let params = CombustionParams {
            width: self.width,
            height: self.height,
            cell_size: self.cell_size,
            dt,
            // Get surface fuel from center cell for uniform params
            ignition_temp_k: {
                let center_x = (self.width / 2) as usize;
                let center_y = (self.height / 2) as usize;
                let fuel = self.fuel_grid.get_surface_fuel(center_x, center_y);
                (*fuel.ignition_temperature + 273.15) as f32
            },
            moisture_extinction: {
                let center_x = (self.width / 2) as usize;
                let center_y = (self.height / 2) as usize;
                *self
                    .fuel_grid
                    .get_surface_fuel(center_x, center_y)
                    .moisture_of_extinction
            },
            heat_content_kj: {
                let center_x = (self.width / 2) as usize;
                let center_y = (self.height / 2) as usize;
                *self
                    .fuel_grid
                    .get_surface_fuel(center_x, center_y)
                    .heat_content
            },
            self_heating_fraction: {
                let center_x = (self.width / 2) as usize;
                let center_y = (self.height / 2) as usize;
                *self
                    .fuel_grid
                    .get_surface_fuel(center_x, center_y)
                    .self_heating_fraction
            },
            burn_rate_coefficient: {
                let center_x = (self.width / 2) as usize;
                let center_y = (self.height / 2) as usize;
                self.fuel_grid
                    .get_surface_fuel(center_x, center_y)
                    .burn_rate_coefficient
            },
            ambient_temp_k: self.ambient_temp_k, // From WeatherSystem via step_heat_transfer
            _padding1: 0.0,
            _padding2: 0.0,
        };
        self.queue.write_buffer(
            &self.combustion_params_buffer,
            0,
            bytemuck::bytes_of(&params),
        );

        let bind_group = self.create_combustion_bind_group();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Combustion Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Combustion Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.combustion_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let (wg_x, wg_y) = self.workgroup_count();
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    fn step_moisture(&mut self, _dt: f32, _humidity: f32) {
        // Moisture update is handled in combustion shader
        // This is a placeholder for more advanced moisture dynamics
    }

    fn step_level_set(&mut self, dt: f32, _wind: Vec3, _ambient_temp: Kelvin) {
        self.time += dt;

        // Update uniform buffer
        let params = LevelSetParams {
            width: self.width,
            height: self.height,
            cell_size: self.cell_size,
            dt,
            curvature_coeff: 0.25,
            noise_amplitude: 0.1,
            time: self.time,
            _padding: 0.0,
        };
        self.queue.write_buffer(
            &self.level_set_params_buffer,
            0,
            bytemuck::bytes_of(&params),
        );

        let bind_group = self.create_level_set_bind_group();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Level Set Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Level Set Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.level_set_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let (wg_x, wg_y) = self.workgroup_count();
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Flip ping-pong
        self.phi_ping = !self.phi_ping;

        // Phase 3: Dispatch crown fire shader (updates spread_rate and fire_intensity on GPU)
        self.dispatch_crown_fire(dt);

        // Phase 5-8: Dispatch advanced physics shader (VLS and valley channeling)
        self.dispatch_advanced_physics(dt, self.wind_x, self.wind_y);

        // Phase 5: CPU-side junction zone detection (requires global analysis)
        // Read back level_set and spread_rate from GPU for junction detection
        let level_set_data = self.read_level_set();
        let mut spread_rate_data = self.read_spread_rate();

        let junctions = self.junction_zone_detector.detect(
            &level_set_data,
            &spread_rate_data,
            self.width as usize,
            self.height as usize,
            self.cell_size,
            dt,
        );

        // Apply junction acceleration to spread rates
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

                    if x >= self.width as usize || y >= self.height as usize {
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

                    let idx = y * self.width as usize + x;
                    if spread_rate_data[idx] > 0.0 {
                        spread_rate_data[idx] *= local_acceleration;
                    }
                }
            }
        }

        // Write modified spread rates back to GPU
        self.write_spread_rate(&spread_rate_data);

        // Phase 8: CPU-side regime detection (uses fire intensity from GPU)
        let intensity_data = self.read_fire_intensity();
        let wind_speed_m_s = self.wind_speed_10m_kmh / 3.6;
        let ambient_temp_c = self.ambient_temp_k - 273.15; // Convert K to °C

        for (idx, &intensity) in intensity_data.iter().enumerate() {
            if intensity > 0.0 {
                use super::regime::detect_regime;
                let regime = detect_regime(intensity, wind_speed_m_s, ambient_temp_c);
                self.fire_regime[idx] = regime;
            } else {
                use super::regime::FireRegime;
                self.fire_regime[idx] = FireRegime::WindDriven;
            }
        }

        // Phase 1: Dispatch fuel layer shader (vertical heat transfer)
        self.dispatch_fuel_layers(dt);

        // Phase 4: Dispatch atmosphere reduction and update CPU-side state
        self.dispatch_atmosphere(dt);
    }

    fn step_ignition_sync(&mut self) {
        let bind_group = self.create_ignition_bind_group();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Ignition Sync Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Ignition Sync Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.ignition_sync_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let (wg_x, wg_y) = self.workgroup_count();
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Flip ping-pong (ignition updates phi)
        self.phi_ping = !self.phi_ping;
    }

    fn read_temperature(&self) -> Cow<'_, [f32]> {
        let buffer_size = u64::from(self.width * self.height) * std::mem::size_of::<f32>() as u64;

        // Copy current temperature buffer to staging
        let src_buffer = if self.temp_ping {
            &self.temperature_a
        } else {
            &self.temperature_b
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Temperature Readback Encoder"),
            });

        encoder.copy_buffer_to_buffer(src_buffer, 0, &self.temperature_staging, 0, buffer_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map and read
        let buffer_slice = self.temperature_staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        self.temperature_staging.unmap();

        Cow::Owned(result)
    }

    fn read_level_set(&self) -> Cow<'_, [f32]> {
        let buffer_size = u64::from(self.width * self.height) * std::mem::size_of::<f32>() as u64;

        // Copy current level set buffer to staging
        let src_buffer = if self.phi_ping {
            &self.level_set_a
        } else {
            &self.level_set_b
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Level Set Readback Encoder"),
            });

        encoder.copy_buffer_to_buffer(src_buffer, 0, &self.level_set_staging, 0, buffer_size);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map and read
        let buffer_slice = self.level_set_staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        self.level_set_staging.unmap();

        Cow::Owned(result)
    }

    #[allow(clippy::cast_precision_loss)] // Grid indices are small enough for f32
    fn apply_heat(&mut self, x: Meters, y: Meters, temperature_k: Kelvin, radius_m: Meters) {
        // Read current temperature and level set from GPU
        let mut temp_data = self.read_temperature().into_owned();
        let mut phi_data = self.read_level_set().into_owned();

        // Apply heat with Gaussian falloff (CPU-side for now)
        let grid_x = (*x / self.cell_size) as i32;
        let grid_y = (*y / self.cell_size) as i32;
        let radius_cells = (*radius_m / self.cell_size).max(0.5);
        let search_radius = (radius_cells.ceil() as i32).max(1);
        let sigma = radius_cells / 2.0;
        let sigma_sq = sigma * sigma;

        for dy in -search_radius..=search_radius {
            for dx in -search_radius..=search_radius {
                let gx = grid_x + dx;
                let gy = grid_y + dy;

                if gx >= 0 && gx < self.width as i32 && gy >= 0 && gy < self.height as i32 {
                    let idx = (gy as u32 * self.width + gx as u32) as usize;
                    let dist_sq = (dx * dx + dy * dy) as f32;
                    let heat_factor = (-dist_sq / (2.0 * sigma_sq)).exp();
                    let applied_temp = temperature_k.as_f32() * heat_factor;

                    // Apply heat (max with current temp)
                    let new_temp = temp_data[idx].max(applied_temp);
                    temp_data[idx] = new_temp;

                    // Mark as burning if temperature reaches ignition threshold
                    let fuel = self.fuel_grid.get_surface_fuel(gx as usize, gy as usize);
                    let ignition_temp = fuel.ignition_temperature.as_f32() + 273.15;

                    if new_temp >= ignition_temp {
                        phi_data[idx] = -self.cell_size * 0.5;
                    }
                }
            }
        }

        // Write back to GPU buffers
        self.queue
            .write_buffer(&self.temperature_a, 0, bytemuck::cast_slice(&temp_data));
        self.queue
            .write_buffer(&self.temperature_b, 0, bytemuck::cast_slice(&temp_data));
        self.queue
            .write_buffer(&self.level_set_a, 0, bytemuck::cast_slice(&phi_data));
        self.queue
            .write_buffer(&self.level_set_b, 0, bytemuck::cast_slice(&phi_data));
    }

    fn dimensions(&self) -> (u32, u32, Meters) {
        (self.width, self.height, Meters::new(self.cell_size))
    }

    fn is_gpu_accelerated(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::context::GpuInitResult;

    /// Helper to create flat terrain with f32 dimensions (for test convenience)
    fn flat_terrain(width: f32, height: f32, resolution: f32, elevation: f32) -> TerrainData {
        TerrainData::flat(
            crate::core_types::units::Meters::new(width),
            crate::core_types::units::Meters::new(height),
            crate::core_types::units::Meters::new(resolution),
            crate::core_types::units::Meters::new(elevation),
        )
    }

    #[test]
    fn test_gpu_solver_creation() {
        // Only run if GPU is available
        if let GpuInitResult::Success(context) = GpuContext::new() {
            let terrain = flat_terrain(1000.0, 1000.0, 10.0, 0.0);
            let solver = GpuFieldSolver::new(context, &terrain, QualityPreset::Medium);

            let (width, height, cell_size) = solver.dimensions();
            assert_eq!(width, 100);
            assert_eq!(height, 100);
            assert_eq!(*cell_size, 10.0);
            assert!(solver.is_gpu_accelerated());
        }
    }

    #[test]
    fn test_gpu_solver_read_temperature() {
        // Only run if GPU is available
        if let GpuInitResult::Success(context) = GpuContext::new() {
            let terrain = TerrainData::flat(
                Meters::new(100.0),
                Meters::new(100.0),
                Meters::new(10.0),
                Meters::new(0.0),
            );
            let solver = GpuFieldSolver::new(context, &terrain, QualityPreset::Low);

            let temp = solver.read_temperature();
            assert!(!temp.is_empty());
            // Should return ambient temperature (~293.15 K)
            assert!(temp.iter().all(|&t| (t - 293.15).abs() < 1.0));
        }
    }

    #[test]
    fn test_gpu_solver_dimensions() {
        // Only run if GPU is available
        if let GpuInitResult::Success(context) = GpuContext::new() {
            let terrain = TerrainData::flat(
                Meters::new(500.0),
                Meters::new(300.0),
                Meters::new(10.0),
                Meters::new(0.0),
            );
            let solver = GpuFieldSolver::new(context, &terrain, QualityPreset::High);

            let (width, height, _cell_size) = solver.dimensions();
            // 500m / 5m per cell = 100, 300m / 5m per cell = 60 → clamped to 64 (minimum)
            assert_eq!(width, 100);
            assert_eq!(height, 64); // Minimum grid size is 64
        }
    }
}
