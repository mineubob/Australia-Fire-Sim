//! Level Set Method for Fire Front Propagation
//!
//! Implements industry-standard level set method using Hamilton-Jacobi equation:
//! ∂φ/∂t + R(x,y,t)|∇φ| = 0
//!
//! where φ is the signed distance function and R is the fire spread rate.
//!
//! # References
//! - Sethian, J.A. (1999) "Level Set Methods and Fast Marching Methods"
//! - Osher & Fedkiw (2003) "Level Set Methods and Dynamic Implicit Surfaces"

use super::GpuContext;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

/// Parameters for level set computation
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct LevelSetParams {
    width: u32,
    height: u32,
    dx: f32,          // Grid spacing in meters
    dt: f32,          // Time step in seconds
    fixed_scale: i32, // Fixed-point scale (e.g., 1000)
}

/// GPU-accelerated level set solver for fire front propagation
///
/// Uses wgpu compute shaders for real-time performance on large grids.
/// Target: 2048×2048 grid at <5ms per timestep.
pub struct GpuLevelSetSolver {
    context: Arc<GpuContext>,
    compute_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    width: u32,
    height: u32,
    grid_spacing: f32,
    fixed_scale: i32,

    // GPU buffers
    phi_buffer_a: wgpu::Buffer,
    phi_buffer_b: wgpu::Buffer,
    spread_rate_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,

    // Ping-pong state (which buffer is current)
    current_is_a: bool,
}

impl GpuLevelSetSolver {
    /// Create a new GPU level set solver
    ///
    /// # Arguments
    /// * `context` - GPU context for compute operations
    /// * `width` - Grid width (number of cells)
    /// * `height` - Grid height (number of cells)
    /// * `grid_spacing` - Physical size of each grid cell in meters
    ///
    /// # Errors
    /// Returns error if GPU resources cannot be allocated
    #[track_caller]
    pub fn new(
        context: Arc<GpuContext>,
        width: u32,
        height: u32,
        grid_spacing: f32,
    ) -> Result<Self, String> {
        // Fixed-point scale factor: 1024 = 2^10 (exact sqrt=32, ~3 decimal precision)
        // Using power-of-2 eliminates sqrt(scale) approximation error
        let fixed_scale = 1024_i32;

        // Check memory requirements
        let buffer_size = u64::from(width * height * 4); // 4 bytes per i32
        if !context.has_sufficient_memory(width, height, 4) {
            return Err(format!(
                "Insufficient GPU memory for {width}×{height} grid (requires ~{} MB)",
                buffer_size * 3 / 1_048_576 // 3 buffers
            ));
        }

        // Load compute shader
        let shader_source = include_str!("level_set_compute.wgsl");
        let shader = context
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Level Set Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        // Create bind group layout
        let bind_group_layout =
            context
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Level Set Bind Group Layout"),
                    entries: &[
                        // phi_in (read-only storage)
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
                        // spread_rate (read-only storage)
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
                        // phi_out (read-write storage)
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
                        // params (uniform)
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
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
                    label: Some("Level Set Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            context
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Level Set Compute Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "main",
                });

        // Create GPU buffers (ping-pong for level set)
        let cell_count = (width * height) as usize;
        let buffer_size_bytes = (cell_count * std::mem::size_of::<i32>()) as u64;

        let phi_buffer_a = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Phi Buffer A"),
            size: buffer_size_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let phi_buffer_b = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Phi Buffer B"),
            size: buffer_size_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let spread_rate_buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Spread Rate Buffer"),
            size: buffer_size_bytes, // u32, same size as i32
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Level Set Params"),
            size: std::mem::size_of::<LevelSetParams>() as u64,
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
            phi_buffer_a,
            phi_buffer_b,
            spread_rate_buffer,
            params_buffer,
            current_is_a: true,
        })
    }

    /// Initialize the level set field with a signed distance function
    ///
    /// # Arguments
    /// * `phi` - Signed distance field (positive outside fire, negative inside, zero at boundary)
    ///   Length must equal width * height
    pub fn initialize_phi(&mut self, phi: &[f32]) {
        assert_eq!(phi.len(), (self.width * self.height) as usize);

        // Convert to fixed-point
        #[expect(
            clippy::cast_precision_loss,
            reason = "Converting i32 fixed_scale to f32 for arithmetic - precision loss acceptable as fixed_scale is small (1000)"
        )]
        let phi_fixed: Vec<i32> = phi
            .iter()
            .map(|&val| (val * self.fixed_scale as f32) as i32)
            .collect();

        // Upload to GPU (both buffers for ping-pong)
        self.context
            .queue()
            .write_buffer(&self.phi_buffer_a, 0, bytemuck::cast_slice(&phi_fixed));
        self.context
            .queue()
            .write_buffer(&self.phi_buffer_b, 0, bytemuck::cast_slice(&phi_fixed));
    }

    /// Update spread rate field R(x,y,t)
    ///
    /// # Arguments
    /// * `spread_rates` - Fire spread rates in m/s for each grid cell
    ///   Length must equal width * height
    pub fn update_spread_rates(&mut self, spread_rates: &[f32]) {
        assert_eq!(spread_rates.len(), (self.width * self.height) as usize);

        // Convert to fixed-point unsigned (spread rates are always positive)
        #[expect(
            clippy::cast_precision_loss,
            reason = "Converting i32 fixed_scale to f32 for arithmetic - precision loss acceptable as fixed_scale is small (1000)"
        )]
        let rates_fixed: Vec<u32> = spread_rates
            .iter()
            .map(|&val| (val * self.fixed_scale as f32).max(0.0) as u32)
            .collect();

        self.context.queue().write_buffer(
            &self.spread_rate_buffer,
            0,
            bytemuck::cast_slice(&rates_fixed),
        );
    }

    /// Perform one timestep of level set evolution
    ///
    /// # Arguments
    /// * `dt` - Time step in seconds
    pub fn step(&mut self, dt: f32) {
        // Update params
        let params = LevelSetParams {
            width: self.width,
            height: self.height,
            dx: self.grid_spacing,
            dt,
            fixed_scale: self.fixed_scale,
        };

        self.context
            .queue()
            .write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));

        // Create bind group for this pass (ping-pong buffers)
        let (phi_in, phi_out) = if self.current_is_a {
            (&self.phi_buffer_a, &self.phi_buffer_b)
        } else {
            (&self.phi_buffer_b, &self.phi_buffer_a)
        };

        let bind_group = self
            .context
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Level Set Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: phi_in.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.spread_rate_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: phi_out.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                ],
            });

        // Dispatch compute shader
        let mut encoder =
            self.context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Level Set Compute Encoder"),
                });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Level Set Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (16x16 threads per workgroup)
            let workgroup_size_x = 16_u32;
            let workgroup_size_y = 16_u32;
            let num_workgroups_x = self.width.div_ceil(workgroup_size_x);
            let num_workgroups_y = self.height.div_ceil(workgroup_size_y);

            compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);
        }

        self.context.queue().submit(Some(encoder.finish()));

        // Flip ping-pong state
        self.current_is_a = !self.current_is_a;
    }

    /// Read current phi field from GPU (blocking operation)
    ///
    /// # Returns
    /// Signed distance field as f32 values
    pub fn read_phi(&self) -> Vec<f32> {
        let buffer_size = u64::from(self.width * self.height * std::mem::size_of::<i32>() as u32);

        // Create staging buffer for readback
        let staging_buffer = self
            .context
            .device()
            .create_buffer(&wgpu::BufferDescriptor {
                label: Some("Phi Readback Buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

        // Copy current phi buffer to staging
        let current_buffer = if self.current_is_a {
            &self.phi_buffer_a
        } else {
            &self.phi_buffer_b
        };

        let mut encoder =
            self.context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Phi Copy Encoder"),
                });

        encoder.copy_buffer_to_buffer(current_buffer, 0, &staging_buffer, 0, buffer_size);

        self.context.queue().submit(Some(encoder.finish()));

        // Map buffer and read data (blocking)
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });

        // Wait for mapping to complete
        self.context.device().poll(wgpu::Maintain::Wait);
        receiver.recv().ok();

        // Read data
        let data = buffer_slice.get_mapped_range();
        let phi_fixed: &[i32] = bytemuck::cast_slice(&data);

        // Convert back to f32
        #[expect(
            clippy::cast_precision_loss,
            reason = "Fixed-point i32 to f32 conversion - precision loss is acceptable for visualization (values stay within valid range)"
        )]
        let phi: Vec<f32> = phi_fixed
            .iter()
            .map(|&val| val as f32 / self.fixed_scale as f32)
            .collect();

        drop(data);
        staging_buffer.unmap();

        phi
    }

    /// Get grid dimensions
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get grid spacing in meters
    #[must_use]
    pub fn grid_spacing(&self) -> f32 {
        self.grid_spacing
    }
}

/// CPU fallback implementation of level set solver
///
/// Uses IDENTICAL fixed-point algorithm as GPU shader for bit-exact determinism.
/// This is NOT a band-aid - it's required for multiplayer validation where
/// GPU and CPU implementations must produce identical results.
pub struct CpuLevelSetSolver {
    width: u32,
    height: u32,
    grid_spacing: f32,
    fixed_scale: i32,
    phi: Vec<i32>,          // Store in fixed-point (matching GPU)
    phi_temp: Vec<i32>,     // Store in fixed-point (matching GPU)
    spread_rates: Vec<i32>, // Store in fixed-point (matching GPU)
}

impl CpuLevelSetSolver {
    /// Create a new CPU level set solver with fixed-point arithmetic
    #[must_use]
    pub fn new(width: u32, height: u32, grid_spacing: f32) -> Self {
        let cell_count = (width * height) as usize;
        Self {
            width,
            height,
            grid_spacing,
            fixed_scale: 1024, // 2^10: exact sqrt=32, eliminates approximation error
            phi: vec![0; cell_count],
            phi_temp: vec![0; cell_count],
            spread_rates: vec![0; cell_count],
        }
    }

    /// Initialize the level set field (converts from float to fixed-point)
    pub fn initialize_phi(&mut self, phi: &[f32]) {
        assert_eq!(phi.len(), (self.width * self.height) as usize);
        for (i, &val) in phi.iter().enumerate() {
            self.phi[i] = (val * f64::from(self.fixed_scale) as f32).round() as i32;
        }
    }

    /// Update spread rate field (converts from float to fixed-point)
    pub fn update_spread_rates(&mut self, spread_rates: &[f32]) {
        assert_eq!(spread_rates.len(), (self.width * self.height) as usize);
        for (i, &val) in spread_rates.iter().enumerate() {
            self.spread_rates[i] = (val * f64::from(self.fixed_scale) as f32).round() as i32;
        }
    }

    /// Perform one timestep using EXACT GPU shader algorithm
    /// Fixed-point arithmetic with integer sqrt for bit-exact determinism
    pub fn step(&mut self, dt: f32) {
        let w = self.width as i32;
        let h = self.height as i32;
        let scale = self.fixed_scale;
        let scale_f = f64::from(scale) as f32;

        // Convert dt and dx to fixed-point (matching shader)
        let dt_fixed = (dt * scale_f).round() as i32;
        let dx_fixed = (self.grid_spacing * scale_f).round() as i32;

        // Fixed-point multiply helper (matches GPU shader exactly)
        let fixed_mul = |a: i32, b: i32| -> i32 {
            let a_f = f64::from(a) as f32;
            let b_f = f64::from(b) as f32;
            let result = (a_f * b_f) / scale_f;
            result.round() as i32
        };

        // Integer sqrt helper (Babylonian method, 10 iterations - matches GPU)
        let int_sqrt = |val: i32| -> i32 {
            if val <= 0 {
                return 0;
            }
            let mut sqrt_val = val / 2;
            for _ in 0..10 {
                if sqrt_val == 0 {
                    break;
                }
                sqrt_val = i32::midpoint(sqrt_val, val / sqrt_val);
            }
            sqrt_val
        };

        for j in 0..h {
            for i in 0..w {
                let idx = (j * w + i) as usize;

                // Get phi values (already in fixed-point)
                let phi_c = self.phi[idx];

                // Get neighbors with bounds checking (clamping like GPU shader)
                let get_phi = |x: i32, y: i32| -> i32 {
                    let x_clamped = x.clamp(0, w - 1);
                    let y_clamped = y.clamp(0, h - 1);
                    let idx = (y_clamped * w + x_clamped) as usize;
                    self.phi[idx]
                };

                let phi_xm = get_phi(i - 1, j);
                let phi_xp = get_phi(i + 1, j);
                let phi_ym = get_phi(i, j - 1);
                let phi_yp = get_phi(i, j + 1);

                // Forward/backward differences (NOT divided by dx yet)
                let d_xm = phi_c - phi_xm;
                let d_xp = phi_xp - phi_c;
                let d_ym = phi_c - phi_ym;
                let d_yp = phi_yp - phi_c;

                // Gradient magnitude using max absolute value from one-sided differences
                // This captures sharp discontinuities correctly (matches GPU shader)
                let grad_x = if d_xm.abs() > d_xp.abs() { d_xm } else { d_xp };
                let grad_y = if d_ym.abs() > d_yp.abs() { d_ym } else { d_yp };

                // |∇φ|² using fixed_mul to prevent overflow (matches GPU shader)
                let dx2 = fixed_mul(grad_x, grad_x);
                let dy2 = fixed_mul(grad_y, grad_y);
                let grad_mag_sq = dx2 + dy2;

                // Integer sqrt (matches GPU exactly)
                let sqrt_val = int_sqrt(grad_mag_sq);

                // Gradient magnitude: d/dx in fixed-point
                // Use i64 intermediate to prevent overflow with large gradients
                // With scale=1024=2^10: sqrt(scale)=32 exactly (no approximation!)
                let sqrt_scale = 32_i64; // sqrt(1024) = 32 exactly
                let grad_mag_fixed = if dx_fixed != 0 {
                    ((i64::from(sqrt_val) * sqrt_scale * i64::from(scale)) / i64::from(dx_fixed))
                        as i32
                } else {
                    0
                };

                // Level set update: φ_new = φ_old - dt * R * |∇φ|
                let r_fixed = self.spread_rates[idx];
                let r_grad = fixed_mul(r_fixed, grad_mag_fixed);
                let dphi = fixed_mul(dt_fixed, r_grad);

                self.phi_temp[idx] = phi_c - dphi;
            }
        }

        // Swap buffers
        std::mem::swap(&mut self.phi, &mut self.phi_temp);
    }

    /// Read current phi field (converts from fixed-point to float)
    #[must_use]
    pub fn read_phi(&self) -> Vec<f32> {
        self.phi
            .iter()
            .map(|&val| f64::from(val) as f32 / f64::from(self.fixed_scale) as f32)
            .collect()
    }

    /// Get grid dimensions
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get grid spacing
    #[must_use]
    pub fn grid_spacing(&self) -> f32 {
        self.grid_spacing
    }
}

/// Level set solver supporting both GPU and CPU backends
pub enum LevelSetSolver {
    /// GPU-accelerated solver (high performance)
    /// Boxed to reduce enum size (GPU solver is large ~600 bytes)
    Gpu(Box<GpuLevelSetSolver>),
    /// CPU fallback solver (for validation and compatibility)
    Cpu(CpuLevelSetSolver),
}

impl LevelSetSolver {
    /// Create a GPU solver if available, otherwise fallback to CPU
    ///
    /// # Arguments
    /// * `width` - Grid width
    /// * `height` - Grid height
    /// * `grid_spacing` - Physical size of grid cells in meters
    #[must_use]
    pub fn new(width: u32, height: u32, grid_spacing: f32) -> Self {
        // Try to create GPU solver
        match pollster::block_on(GpuContext::new()) {
            Ok(context) => {
                match GpuLevelSetSolver::new(Arc::new(context), width, height, grid_spacing) {
                    Ok(solver) => {
                        tracing::info!("Using GPU-accelerated level set solver");
                        Self::Gpu(Box::new(solver))
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create GPU solver, falling back to CPU: {e}");
                        Self::Cpu(CpuLevelSetSolver::new(width, height, grid_spacing))
                    }
                }
            }
            Err(e) => {
                tracing::warn!("No GPU available, using CPU solver: {e}");
                Self::Cpu(CpuLevelSetSolver::new(width, height, grid_spacing))
            }
        }
    }

    /// Initialize phi field
    pub fn initialize_phi(&mut self, phi: &[f32]) {
        match self {
            Self::Gpu(solver) => solver.initialize_phi(phi),
            Self::Cpu(solver) => solver.initialize_phi(phi),
        }
    }

    /// Update spread rates
    pub fn update_spread_rates(&mut self, spread_rates: &[f32]) {
        match self {
            Self::Gpu(solver) => solver.update_spread_rates(spread_rates),
            Self::Cpu(solver) => solver.update_spread_rates(spread_rates),
        }
    }

    /// Perform timestep
    pub fn step(&mut self, dt: f32) {
        match self {
            Self::Gpu(solver) => solver.step(dt),
            Self::Cpu(solver) => solver.step(dt),
        }
    }

    /// Read phi field
    #[must_use]
    pub fn read_phi(&self) -> Vec<f32> {
        match self {
            Self::Gpu(solver) => solver.read_phi(),
            Self::Cpu(solver) => solver.read_phi(),
        }
    }

    /// Get dimensions
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Gpu(solver) => solver.dimensions(),
            Self::Cpu(solver) => solver.dimensions(),
        }
    }

    /// Get grid spacing
    #[must_use]
    pub fn grid_spacing(&self) -> f32 {
        match self {
            Self::Gpu(solver) => solver.grid_spacing(),
            Self::Cpu(solver) => solver.grid_spacing(),
        }
    }

    /// Check if using GPU backend
    #[must_use]
    pub fn is_gpu(&self) -> bool {
        matches!(self, Self::Gpu(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_solver_initialization() {
        let mut solver = CpuLevelSetSolver::new(64, 64, 1.0);

        // Initialize with circular fire
        let mut phi = vec![10.0; 64 * 64];
        for j in 0..64_i32 {
            for i in 0..64_i32 {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "Small test coordinates (0-64) to f32 - precision loss acceptable for test setup"
                )]
                let x = i as f32 - 32.0;
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "Small test coordinates (0-64) to f32 - precision loss acceptable for test setup"
                )]
                let y = j as f32 - 32.0;
                let dist = (x * x + y * y).sqrt();
                phi[(j * 64 + i) as usize] = dist - 10.0; // Circle radius 10
            }
        }

        solver.initialize_phi(&phi);

        // Set uniform spread rate
        let spread_rates = vec![1.0; 64 * 64]; // 1 m/s
        solver.update_spread_rates(&spread_rates);

        // Step forward
        solver.step(1.0); // 1 second

        // Check that fire advanced (phi values inside should be more negative)
        let phi_new = solver.read_phi();

        // Center should still be negative (inside fire)
        let center_idx = 32 * 64 + 32;
        assert!(phi_new[center_idx] < 0.0);
    }

    #[test]
    fn test_level_set_solver_enum() {
        // Test that enum can be created (will use CPU or GPU depending on system)
        let mut solver = LevelSetSolver::new(32, 32, 1.0);

        // Basic smoke test
        let phi = vec![1.0; 32 * 32];
        solver.initialize_phi(&phi);

        let spread_rates = vec![0.5; 32 * 32];
        solver.update_spread_rates(&spread_rates);

        solver.step(0.1);

        let phi_out = solver.read_phi();
        assert_eq!(phi_out.len(), 32 * 32);
    }
}
