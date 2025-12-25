//! GPU-Accelerated Rothermel Spread Rate with Curvature and Vorticity
//!
//! Calculates composite fire spread rate field using:
//! - Rothermel (1972) base spread rate model
//! - Margerit & Séro-Guillaume (2002) curvature effects
//! - Countryman (1971) fire whirl vorticity physics
//!
//! Formula: `R(x,y,t) = R_base × wind_factor × slope_factor × (1 + 0.25×κ) × vortex_boost`

use super::GpuContext;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

/// Fuel properties for Rothermel calculation (matches shader layout)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct FuelParams {
    // Per fuel type properties (up to 8 fuel types)
    sigma: [f32; 8],         // Surface-area-to-volume ratio (1/ft)
    delta: [f32; 8],         // Fuel bed depth (ft)
    mx_dead: [f32; 8],       // Dead fuel moisture content
    mx_extinction: [f32; 8], // Moisture of extinction
    heat_content: [f32; 8],  // Heat of combustion (BTU/lb)
    fuel_load: [f32; 8],     // Fuel load (lb/ft²)
    // Global parameters
    width: u32,
    height: u32,
    dx: f32,          // Grid spacing (meters)
    fixed_scale: i32, // Fixed-point scale
}

/// GPU-accelerated Rothermel spread rate calculator
///
/// Computes fire spread rates incorporating:
/// - Rothermel surface fire spread model
/// - Fire front curvature effects (convex fronts spread faster)
/// - Vorticity-induced acceleration (fire whirls)
pub struct GpuRothermelSolver {
    context: Arc<GpuContext>,
    compute_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    width: u32,
    height: u32,
    grid_spacing: f32,
    fixed_scale: i32,

    // GPU buffers
    phi_buffer: wgpu::Buffer,
    fuel_type_buffer: wgpu::Buffer,
    wind_field_buffer: wgpu::Buffer,
    slope_buffer: wgpu::Buffer,
    vorticity_buffer: wgpu::Buffer,
    spread_rate_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,
}

impl GpuRothermelSolver {
    /// Create a new GPU Rothermel solver
    ///
    /// # Arguments
    /// * `context` - GPU context for compute operations
    /// * `width` - Grid width (number of cells)
    /// * `height` - Grid height (number of cells)
    /// * `grid_spacing` - Physical size of each grid cell in meters
    ///
    /// # Errors
    /// Returns error if GPU resources cannot be allocated
    pub fn new(
        context: Arc<GpuContext>,
        width: u32,
        height: u32,
        grid_spacing: f32,
    ) -> Result<Self, String> {
        let fixed_scale = 1000_i32; // Same as level set solver

        // Check memory requirements
        if !context.has_sufficient_memory(width, height, 4) {
            return Err(format!(
                "Insufficient GPU memory for {width}×{height} Rothermel grid"
            ));
        }

        // Load compute shader
        let shader_source = include_str!("rothermel_compute.wgsl");
        let shader = context
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Rothermel Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        // Create bind group layout
        let bind_group_layout =
            context
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Rothermel Bind Group Layout"),
                    entries: &[
                        // phi (level set field)
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
                        // fuel_type_grid
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
                        // wind_field
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
                        // slope_grid
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
                        // vorticity
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
                        // spread_rate (output)
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
                        // params (uniform)
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

        // Create compute pipeline
        let pipeline_layout =
            context
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Rothermel Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            context
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Rothermel Compute Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "main",
                });

        // Create GPU buffers
        let cell_count = (width * height) as usize;
        let i32_buffer_size = (cell_count * std::mem::size_of::<i32>()) as u64;
        let u32_buffer_size = (cell_count * std::mem::size_of::<u32>()) as u64;
        let vec2_buffer_size = (cell_count * std::mem::size_of::<[f32; 2]>()) as u64;
        let f32_buffer_size = (cell_count * std::mem::size_of::<f32>()) as u64;

        let phi_buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Phi Buffer"),
            size: i32_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let fuel_type_buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fuel Type Buffer"),
            size: u32_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let wind_field_buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Wind Field Buffer"),
            size: vec2_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let slope_buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Slope Buffer"),
            size: vec2_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let vorticity_buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vorticity Buffer"),
            size: f32_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let spread_rate_buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Spread Rate Buffer"),
            size: u32_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fuel Params Buffer"),
            size: std::mem::size_of::<FuelParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            context,
            compute_pipeline,
            bind_group_layout,
            width,
            height,
            grid_spacing,
            fixed_scale,
            phi_buffer,
            fuel_type_buffer,
            wind_field_buffer,
            slope_buffer,
            vorticity_buffer,
            spread_rate_buffer,
            params_buffer,
        })
    }

    /// Update input fields and compute spread rates
    ///
    /// # Arguments
    /// * `phi` - Level set field (for curvature calculation)
    /// * `fuel_types` - Fuel type ID per grid cell (0-7)
    /// * `wind_field` - Wind velocity (u, v) per cell in m/s
    /// * `slope` - Terrain slope (dz/dx, dz/dy) per cell
    /// * `vorticity` - Vorticity field (s⁻¹) per cell
    ///
    /// # Returns
    /// Spread rate field in m/s
    pub fn compute_spread_rates(
        &self,
        phi: &[f32],
        fuel_types: &[u32],
        wind_field: &[[f32; 2]],
        slope: &[[f32; 2]],
        vorticity: &[f32],
    ) -> Vec<f32> {
        let cell_count = (self.width * self.height) as usize;
        assert_eq!(phi.len(), cell_count);
        assert_eq!(fuel_types.len(), cell_count);
        assert_eq!(wind_field.len(), cell_count);
        assert_eq!(slope.len(), cell_count);
        assert_eq!(vorticity.len(), cell_count);

        // Convert phi to fixed-point
        #[expect(
            clippy::cast_precision_loss,
            reason = "Fixed-point conversion - precision loss acceptable for GPU compute"
        )]
        let phi_fixed: Vec<i32> = phi
            .iter()
            .map(|&val| (val * self.fixed_scale as f32) as i32)
            .collect();

        // Upload to GPU
        self.context
            .queue()
            .write_buffer(&self.phi_buffer, 0, bytemuck::cast_slice(&phi_fixed));
        self.context.queue().write_buffer(
            &self.fuel_type_buffer,
            0,
            bytemuck::cast_slice(fuel_types),
        );
        self.context.queue().write_buffer(
            &self.wind_field_buffer,
            0,
            bytemuck::cast_slice(wind_field),
        );
        self.context
            .queue()
            .write_buffer(&self.slope_buffer, 0, bytemuck::cast_slice(slope));
        self.context.queue().write_buffer(
            &self.vorticity_buffer,
            0,
            bytemuck::cast_slice(vorticity),
        );

        // Setup fuel parameters (using default values for demonstration)
        let params = FuelParams {
            sigma: [
                2000.0, 1500.0, 1800.0, 1200.0, 2500.0, 1000.0, 3000.0, 2200.0,
            ],
            delta: [1.0, 2.0, 1.5, 3.0, 0.5, 2.5, 0.8, 1.2],
            mx_dead: [0.05, 0.07, 0.06, 0.08, 0.04, 0.09, 0.05, 0.06],
            mx_extinction: [0.25, 0.30, 0.28, 0.35, 0.20, 0.40, 0.22, 0.27],
            heat_content: [
                8000.0, 8500.0, 8200.0, 9000.0, 7500.0, 9500.0, 7800.0, 8300.0,
            ],
            fuel_load: [0.05, 0.10, 0.08, 0.15, 0.03, 0.20, 0.04, 0.09],
            width: self.width,
            height: self.height,
            dx: self.grid_spacing,
            fixed_scale: self.fixed_scale,
        };

        self.context
            .queue()
            .write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));

        // Create bind group
        let bind_group = self
            .context
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Rothermel Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.phi_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.fuel_type_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.wind_field_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: self.slope_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.vorticity_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: self.spread_rate_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                ],
            });

        // Dispatch compute shader
        let mut encoder =
            self.context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Rothermel Compute Encoder"),
                });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Rothermel Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let workgroup_size_x = 16_u32;
            let workgroup_size_y = 16_u32;
            let num_workgroups_x = self.width.div_ceil(workgroup_size_x);
            let num_workgroups_y = self.height.div_ceil(workgroup_size_y);

            compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);
        }

        self.context.queue().submit(Some(encoder.finish()));

        // Read back results
        self.read_spread_rates()
    }

    /// Read spread rates from GPU (blocking)
    fn read_spread_rates(&self) -> Vec<f32> {
        let buffer_size = u64::from(self.width * self.height * std::mem::size_of::<u32>() as u32);

        let staging_buffer = self
            .context
            .device()
            .create_buffer(&wgpu::BufferDescriptor {
                label: Some("Spread Rate Readback"),
                size: buffer_size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

        let mut encoder =
            self.context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Spread Rate Copy Encoder"),
                });

        encoder.copy_buffer_to_buffer(&self.spread_rate_buffer, 0, &staging_buffer, 0, buffer_size);
        self.context.queue().submit(Some(encoder.finish()));

        // Map and read
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });

        self.context.device().poll(wgpu::Maintain::Wait);
        receiver.recv().ok();

        let data = buffer_slice.get_mapped_range();
        let rates_fixed: &[u32] = bytemuck::cast_slice(&data);

        // Convert back to f32
        #[expect(
            clippy::cast_precision_loss,
            reason = "Fixed-point to f32 conversion - precision loss acceptable for final output"
        )]
        let rates: Vec<f32> = rates_fixed
            .iter()
            .map(|&val| val as f32 / self.fixed_scale as f32)
            .collect();

        drop(data);
        staging_buffer.unmap();

        rates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rothermel_solver_creation() {
        // Try to create solver (may fail on systems without GPU)
        let result = pollster::block_on(GpuContext::new());

        if let Ok(context) = result {
            let solver = GpuRothermelSolver::new(Arc::new(context), 64, 64, 5.0);

            match solver {
                Ok(_) => {
                    // Solver created successfully
                }
                Err(e) => {
                    eprintln!(
                        "Rothermel solver creation failed (acceptable on low-memory systems): {e}"
                    );
                }
            }
        }
    }
}
