//! GPU context and initialization
//!
//! This module handles GPU device initialization and capability detection.
//! It provides proper error handling to distinguish between "no GPU found"
//! (expected on some systems) and "GPU found but failed to initialize"
//! (potential driver issue).

/// Result of GPU initialization attempt
///
/// This enum distinguishes between different failure modes:
/// - `NoGpuFound`: No compatible GPU adapter (silent fallback to CPU)
/// - `InitFailed`: GPU found but initialization failed (log warning)
#[derive(Debug)]
pub enum GpuInitResult {
    /// GPU initialized successfully
    #[cfg(feature = "gpu")]
    Success(GpuContext),
    /// No GPU adapter found (silent fallback to CPU)
    NoGpuFound,
    /// GPU found but initialization failed (log warning, fallback to CPU)
    InitFailed {
        /// Name of the adapter that failed
        adapter_name: String,
        /// Error message
        error: String,
    },
}

// All GPU-specific code is conditionally compiled only when "gpu" feature is enabled
#[cfg(feature = "gpu")]
mod gpu_impl {
    use super::GpuInitResult;
    use tracing::{debug, info};

    /// GPU context managing device and queue
    ///
    /// Wraps wgpu device and queue along with adapter information.
    /// Provides helper methods for capability detection and optimal configuration.
    #[derive(Debug)]
    pub struct GpuContext {
        device: wgpu::Device,
        queue: wgpu::Queue,
        adapter_info: wgpu::AdapterInfo,
    }

    impl GpuContext {
        /// Initialize GPU context
        ///
        /// Attempts to create a wgpu device and queue for high-performance compute.
        ///
        /// # Returns
        ///
        /// - `GpuInitResult::Success` - GPU ready to use
        /// - `GpuInitResult::NoGpuFound` - No compatible GPU adapter
        /// - `GpuInitResult::InitFailed` - GPU found but initialization failed
        ///
        /// The distinction matters: "no GPU" is expected on some systems,
        /// but "GPU found but failed" might indicate a driver issue worth logging.
        #[allow(clippy::new_ret_no_self)]
        pub fn new() -> GpuInitResult {
            info!("Attempting to initialize GPU context");

            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });

            // Try to find a GPU adapter
            let adapter =
                if let Some(a) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })) {
                    debug!("Found GPU adapter: {}", a.get_info().name);
                    a
                } else {
                    debug!("No GPU adapter found");
                    return GpuInitResult::NoGpuFound;
                };

            let adapter_info = adapter.get_info();
            let adapter_name = adapter_info.name.clone();

            // Try to create device - this can fail even with a valid adapter
            match pollster::block_on(adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("FireSim GPU"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )) {
                Ok((device, queue)) => {
                    info!("GPU context initialized successfully: {}", adapter_name);
                    GpuInitResult::Success(Self {
                        device,
                        queue,
                        adapter_info,
                    })
                }
                Err(e) => {
                    debug!("Failed to create GPU device: {}", e);
                    GpuInitResult::InitFailed {
                        adapter_name,
                        error: e.to_string(),
                    }
                }
            }
        }

        /// Get adapter name for logging
        ///
        /// # Returns
        ///
        /// GPU adapter name (e.g., "NVIDIA `GeForce` GTX 1660")
        #[must_use]
        pub fn adapter_name(&self) -> &str {
            &self.adapter_info.name
        }

        /// Get optimal workgroup size for this GPU vendor
        ///
        /// Different GPU vendors have different optimal workgroup sizes for compute shaders.
        ///
        /// # Returns
        ///
        /// Tuple of (width, height) for compute workgroup size
        #[must_use]
        pub fn optimal_workgroup_size(&self) -> (u32, u32) {
            // NVIDIA/AMD prefer 16x16, Intel prefers 8x8
            match self.adapter_info.vendor {
                0x10DE | 0x1002 => (16, 16), // NVIDIA (0x10DE), AMD (0x1002)
                _ => (8, 8),                 // Intel and others
            }
        }

        /// Check if GPU has enough memory for given grid size
        ///
        /// Estimates memory usage for field textures and checks against the actual GPU device limits.
        ///
        /// # Arguments
        ///
        /// * `width` - Grid width in cells
        /// * `height` - Grid height in cells
        ///
        /// # Returns
        ///
        /// `true` if GPU can likely allocate the required memory
        #[must_use]
        pub fn can_allocate(&self, width: u32, height: u32) -> bool {
            // Estimate: ~6 float textures × 4 bytes × width × height × 2 (ping-pong)
            let estimated_bytes = 6 * 4 * u64::from(width) * u64::from(height) * 2;

            // Get actual device limits
            let limits = self.device.limits();
            let max_buffer_size = limits.max_buffer_size;
            let max_texture_dimension_2d = limits.max_texture_dimension_2d;

            // Check if grid dimensions exceed texture limits
            if width > max_texture_dimension_2d || height > max_texture_dimension_2d {
                return false;
            }

            // Check if estimated memory fits within device buffer limit
            // Use 50% of max buffer size to leave headroom for other GPU operations
            estimated_bytes < max_buffer_size / 2
        }

        /// Get reference to wgpu device
        #[must_use]
        pub fn device(&self) -> &wgpu::Device {
            &self.device
        }

        /// Get reference to wgpu queue
        #[must_use]
        pub fn queue(&self) -> &wgpu::Queue {
            &self.queue
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_gpu_init_returns_valid_result() {
            // This test just verifies the function returns one of the valid enum variants
            // We don't assert which one, as that depends on hardware availability
            let result = GpuContext::new();

            match result {
                GpuInitResult::Success(ctx) => {
                    // If we got a GPU, verify basic properties
                    assert!(!ctx.adapter_name().is_empty());
                    let (w, h) = ctx.optimal_workgroup_size();
                    assert!(w > 0 && h > 0);
                }
                GpuInitResult::NoGpuFound => {
                    // This is fine - no GPU available
                }
                GpuInitResult::InitFailed {
                    adapter_name,
                    error,
                } => {
                    // This is also fine - GPU exists but can't initialize
                    assert!(!adapter_name.is_empty());
                    assert!(!error.is_empty());
                }
            }
        }

        #[test]
        fn test_can_allocate() {
            // Create a mock test - we can't guarantee GPU availability
            // If GPU is available, test memory estimation
            if let GpuInitResult::Success(ctx) = GpuContext::new() {
                // Small grid should be allocatable
                assert!(ctx.can_allocate(512, 512));

                // Huge grid should not be allocatable (exceeds 256MB limit)
                assert!(!ctx.can_allocate(8192, 8192));
            }
        }
    }
}

// Re-export GpuContext only when GPU feature is enabled
#[cfg(feature = "gpu")]
pub use gpu_impl::GpuContext;
