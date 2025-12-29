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
//!
//! # Implementation
//!
//! This solver uses wgpu compute pipelines to dispatch physics shaders on GPU.
//! Data is stored in GPU storage buffers with ping-pong double-buffering for in-place
//! updates. Staging buffers handle CPU readback when needed for visualization.

use super::context::GpuContext;
use super::quality::QualityPreset;
use super::FieldSolver;
use crate::TerrainData;
use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;
use wgpu::util::DeviceExt;

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
}

/// Combustion shader parameters (must match WGSL struct layout)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct CombustionParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
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
    spread_rate: wgpu::Buffer,
    heat_release: wgpu::Buffer,

    // Staging buffers for CPU readback
    temperature_staging: wgpu::Buffer,
    level_set_staging: wgpu::Buffer,

    // Uniform buffers for shader parameters
    heat_params_buffer: wgpu::Buffer,
    combustion_params_buffer: wgpu::Buffer,
    level_set_params_buffer: wgpu::Buffer,
    ignition_params_buffer: wgpu::Buffer,

    // Compute pipelines
    heat_transfer_pipeline: wgpu::ComputePipeline,
    combustion_pipeline: wgpu::ComputePipeline,
    level_set_pipeline: wgpu::ComputePipeline,
    ignition_sync_pipeline: wgpu::ComputePipeline,

    // Bind group layouts (needed for bind group creation)
    heat_bind_group_layout: wgpu::BindGroupLayout,
    combustion_bind_group_layout: wgpu::BindGroupLayout,
    level_set_bind_group_layout: wgpu::BindGroupLayout,
    ignition_bind_group_layout: wgpu::BindGroupLayout,

    // Ping-pong state (which buffer is current)
    temp_ping: bool,
    phi_ping: bool,

    // Simulation time (for noise in level set)
    time: f32,
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

        // Initialize field data
        let ambient_temp: f32 = 293.15; // 20°C in Kelvin
        let initial_temp: Vec<f32> = vec![ambient_temp; (width * height) as usize];
        let initial_fuel: Vec<f32> = vec![2.0; (width * height) as usize]; // 2 kg/m²
        let initial_moisture: Vec<f32> = vec![0.15; (width * height) as usize]; // 15%
        let initial_oxygen: Vec<f32> = vec![0.21; (width * height) as usize]; // 21%
        let initial_phi: Vec<f32> = vec![1000.0; (width * height) as usize]; // Far from fire
        let initial_spread: Vec<f32> = vec![0.5; (width * height) as usize]; // 0.5 m/s base spread
        let zeros: Vec<f32> = vec![0.0; (width * height) as usize];

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
            usage: wgpu::BufferUsages::STORAGE,
        });

        let moisture = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Moisture"),
            contents: bytemuck::cast_slice(&initial_moisture),
            usage: wgpu::BufferUsages::STORAGE,
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

        let spread_rate = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spread Rate"),
            contents: bytemuck::cast_slice(&initial_spread),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let heat_release = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Heat Release"),
            contents: bytemuck::cast_slice(&zeros),
            usage: wgpu::BufferUsages::STORAGE,
        });

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
            ignition_temperature: 573.15,
            moisture_extinction: 0.3,
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

        // Load shaders using wgpu::include_wgsl! macro
        let heat_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/heat_transfer.wgsl"));

        let combustion_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/combustion.wgsl"));

        let level_set_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/level_set.wgsl"));

        let ignition_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/ignition_sync.wgsl"));

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
            spread_rate,
            heat_release,
            temperature_staging,
            level_set_staging,
            heat_params_buffer,
            combustion_params_buffer,
            level_set_params_buffer,
            ignition_params_buffer,
            heat_transfer_pipeline,
            combustion_pipeline,
            level_set_pipeline,
            ignition_sync_pipeline,
            heat_bind_group_layout,
            combustion_bind_group_layout,
            level_set_bind_group_layout,
            ignition_bind_group_layout,
            temp_ping: true,
            phi_ping: true,
            time: 0.0,
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
                    resource: self.spread_rate.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: phi_out.as_entire_binding(),
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
}

impl FieldSolver for GpuFieldSolver {
    fn step_heat_transfer(&mut self, dt: f32, wind_x: f32, wind_y: f32, ambient_temp: f32) {
        // Update uniform buffer with new parameters
        let params = HeatParams {
            width: self.width,
            height: self.height,
            cell_size: self.cell_size,
            dt,
            ambient_temp,
            wind_x,
            wind_y,
            stefan_boltzmann: 5.67e-8,
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

    fn step_level_set(&mut self, dt: f32) {
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

    #[allow(clippy::cast_precision_loss)] // Grid cell indices are small enough for f32
    fn ignite_at(&mut self, x: f32, y: f32, radius: f32) {
        // Calculate grid coordinates
        let cx = (x / self.cell_size) as i32;
        let cy = (y / self.cell_size) as i32;
        let r_cells = (radius / self.cell_size) as i32;

        // Read current level set
        let mut phi = self.read_level_set().into_owned();

        // Set phi to negative (burning) for cells within radius
        for dy in -r_cells..=r_cells {
            for dx in -r_cells..=r_cells {
                let gx = cx + dx;
                let gy = cy + dy;

                if gx >= 0 && gx < self.width as i32 && gy >= 0 && gy < self.height as i32 {
                    let dist = ((dx * dx + dy * dy) as f32).sqrt() * self.cell_size;
                    if dist <= radius {
                        let idx = (gy as u32 * self.width + gx as u32) as usize;
                        phi[idx] = -1.0; // Inside fire
                    }
                }
            }
        }

        // Write back to both GPU buffers
        self.queue
            .write_buffer(&self.level_set_a, 0, bytemuck::cast_slice(&phi));
        self.queue
            .write_buffer(&self.level_set_b, 0, bytemuck::cast_slice(&phi));

        // Set high temperature at ignition point
        let mut temp = self.read_temperature().into_owned();
        for dy in -r_cells..=r_cells {
            for dx in -r_cells..=r_cells {
                let gx = cx + dx;
                let gy = cy + dy;

                if gx >= 0 && gx < self.width as i32 && gy >= 0 && gy < self.height as i32 {
                    let dist = ((dx * dx + dy * dy) as f32).sqrt() * self.cell_size;
                    if dist <= radius {
                        let idx = (gy as u32 * self.width + gx as u32) as usize;
                        temp[idx] = 800.0; // High temperature to start combustion
                    }
                }
            }
        }

        self.queue
            .write_buffer(&self.temperature_a, 0, bytemuck::cast_slice(&temp));
        self.queue
            .write_buffer(&self.temperature_b, 0, bytemuck::cast_slice(&temp));
    }

    fn dimensions(&self) -> (u32, u32, f32) {
        (self.width, self.height, self.cell_size)
    }

    fn is_gpu_accelerated(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::context::GpuInitResult;

    #[test]
    fn test_gpu_solver_creation() {
        // Only run if GPU is available
        if let GpuInitResult::Success(context) = GpuContext::new() {
            let terrain = TerrainData::flat(1000.0, 1000.0, 10.0, 0.0);
            let solver = GpuFieldSolver::new(context, &terrain, QualityPreset::Medium);

            let (width, height, cell_size) = solver.dimensions();
            assert_eq!(width, 100);
            assert_eq!(height, 100);
            assert_eq!(cell_size, 10.0);
            assert!(solver.is_gpu_accelerated());
        }
    }

    #[test]
    fn test_gpu_solver_read_temperature() {
        // Only run if GPU is available
        if let GpuInitResult::Success(context) = GpuContext::new() {
            let terrain = TerrainData::flat(100.0, 100.0, 10.0, 0.0);
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
            let terrain = TerrainData::flat(500.0, 300.0, 10.0, 0.0);
            let solver = GpuFieldSolver::new(context, &terrain, QualityPreset::High);

            let (width, height, _cell_size) = solver.dimensions();
            // 500m / 5m per cell = 100, 300m / 5m per cell = 60 → clamped to 64 (minimum)
            assert_eq!(width, 100);
            assert_eq!(height, 64); // Minimum grid size is 64
        }
    }
}
