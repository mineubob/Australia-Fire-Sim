//! GPU-Accelerated Rothermel Spread Rate with Curvature and Vorticity
//!
//! Calculates composite fire spread rate field using:
//! - Rothermel (1972) base spread rate model
//! - Margerit & Séro-Guillaume (2002) curvature effects
//! - Countryman (1971) fire whirl vorticity physics
//!
//! Formula: `R(x,y,t) = R_base × wind_factor × slope_factor × (1 + 0.25×κ) × vortex_boost`

use super::GpuContext;
use crate::core_types::fuel::Fuel;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

/// Extract Rothermel parameters from Fuel instances for GPU upload
/// Converts SI units to Rothermel's imperial units (ft, BTU/lb, etc.)
fn extract_rothermel_params(fuel: &Fuel) -> (f32, f32, f32, f32, f32, f32) {
    // sigma: surface area to volume ratio (m²/m³ → 1/ft by dividing by 3.28084)
    let sigma = *fuel.surface_area_to_volume / 3.28084;

    // delta: fuel bed depth (m → ft)
    let delta = *fuel.fuel_bed_depth * 3.28084;

    // mx_dead: base moisture content (already fraction 0-1)
    let mx_dead = *fuel.base_moisture;

    // mx_extinction: moisture of extinction (already fraction 0-1)
    let mx_extinction = *fuel.moisture_of_extinction;

    // heat_content: (kJ/kg → BTU/lb)
    // 1 kJ/kg = 0.429923 BTU/lb
    let heat_content = *fuel.heat_content * 0.429923;

    // fuel_load: bulk density × fuel bed depth (kg/m³ · m → lb/ft²)
    // kg/m³ · m = kg/m² → lb/ft² (multiply by 0.204816)
    let fuel_load = (*fuel.bulk_density * *fuel.fuel_bed_depth) * 0.204816;

    (
        sigma,
        delta,
        mx_dead,
        mx_extinction,
        heat_content,
        fuel_load,
    )
}

/// Fuel properties for Rothermel calculation (matches shader layout)
/// Aligned to 16-byte boundaries for WGSL uniform requirements
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct FuelParams {
    // Packed vec4<f32> arrays for alignment
    sigma_pack0: [f32; 4],         // sigma[0-3]
    sigma_pack1: [f32; 4],         // sigma[4-7]
    delta_pack0: [f32; 4],         // delta[0-3]
    delta_pack1: [f32; 4],         // delta[4-7]
    mx_dead_pack0: [f32; 4],       // mx_dead[0-3]
    mx_dead_pack1: [f32; 4],       // mx_dead[4-7]
    mx_extinction_pack0: [f32; 4], // mx_extinction[0-3]
    mx_extinction_pack1: [f32; 4], // mx_extinction[4-7]
    heat_content_pack0: [f32; 4],  // heat_content[0-3]
    heat_content_pack1: [f32; 4],  // heat_content[4-7]
    fuel_load_pack0: [f32; 4],     // fuel_load[0-3]
    fuel_load_pack1: [f32; 4],     // fuel_load[4-7]
    // Global parameters
    dimensions: [u32; 2], // width, height
    dx: f32,              // Grid spacing
    fixed_scale: i32,     // Fixed-point scale
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
    /// * `fuels` - Array of 8 Fuel instances corresponding to IDs 0-7
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
        fuels: &[Fuel; 8],
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

        // Extract and destructure fuel parameters from provided Fuel instances
        let [(sigma0, delta0, mx_dead0, mx_ext0, heat0, load0), (sigma1, delta1, mx_dead1, mx_ext1, heat1, load1), (sigma2, delta2, mx_dead2, mx_ext2, heat2, load2), (sigma3, delta3, mx_dead3, mx_ext3, heat3, load3), (sigma4, delta4, mx_dead4, mx_ext4, heat4, load4), (sigma5, delta5, mx_dead5, mx_ext5, heat5, load5), (sigma6, delta6, mx_dead6, mx_ext6, heat6, load6), (sigma7, delta7, mx_dead7, mx_ext7, heat7, load7)] =
            fuels.each_ref().map(extract_rothermel_params);

        // Pack into vec4-aligned arrays for GPU upload
        let params = FuelParams {
            sigma_pack0: [sigma0, sigma1, sigma2, sigma3],
            sigma_pack1: [sigma4, sigma5, sigma6, sigma7],
            delta_pack0: [delta0, delta1, delta2, delta3],
            delta_pack1: [delta4, delta5, delta6, delta7],
            mx_dead_pack0: [mx_dead0, mx_dead1, mx_dead2, mx_dead3],
            mx_dead_pack1: [mx_dead4, mx_dead5, mx_dead6, mx_dead7],
            mx_extinction_pack0: [mx_ext0, mx_ext1, mx_ext2, mx_ext3],
            mx_extinction_pack1: [mx_ext4, mx_ext5, mx_ext6, mx_ext7],
            heat_content_pack0: [heat0, heat1, heat2, heat3],
            heat_content_pack1: [heat4, heat5, heat6, heat7],
            fuel_load_pack0: [load0, load1, load2, load3],
            fuel_load_pack1: [load4, load5, load6, load7],
            dimensions: [self.width, self.height],
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
