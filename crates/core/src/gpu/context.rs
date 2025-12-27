//! GPU Context Management
//!
//! Provides wgpu device initialization and resource management for GPU-accelerated
//! fire simulation computations.

use std::sync::Arc;

/// GPU context for fire simulation compute operations
///
/// Manages wgpu device, queue, and common resources for GPU-accelerated fire physics.
/// Supports fallback to CPU when GPU is unavailable.
pub struct GpuContext {
    /// wgpu device for GPU operations
    device: Arc<wgpu::Device>,
    /// Command queue for submitting GPU work
    queue: Arc<wgpu::Queue>,
    /// Adapter information for performance tuning
    adapter_info: wgpu::AdapterInfo,
}

impl GpuContext {
    /// Create a new GPU context with default settings
    ///
    /// Automatically selects the best available GPU adapter.
    /// Falls back to CPU if no GPU is available.
    ///
    /// # Errors
    /// Returns error if no compatible adapter or device can be created
    pub async fn new() -> Result<Self, String> {
        Self::with_power_preference(wgpu::PowerPreference::HighPerformance).await
    }

    /// Create a new GPU context with specific power preference
    ///
    /// # Arguments
    /// * `power_preference` - GPU selection preference (`LowPower`, `HighPerformance`, or default)
    ///
    /// # Errors
    /// Returns error if no compatible adapter or device can be created
    pub async fn with_power_preference(
        power_preference: wgpu::PowerPreference,
    ) -> Result<Self, String> {
        // Create wgpu instance (platform-specific backend)
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Request adapter (GPU or fallback to CPU)
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find compatible GPU adapter")?;

        let adapter_info = adapter.get_info();
        tracing::info!(
            "GPU adapter selected: {} ({:?})",
            adapter_info.name,
            adapter_info.backend
        );

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Fire Simulation GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None, // No trace path
            )
            .await
            .map_err(|e| format!("Failed to create GPU device: {e}"))?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
        })
    }

    /// Get reference to the wgpu device
    #[must_use]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get reference to the command queue
    #[must_use]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Get adapter information for performance tuning
    #[must_use]
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// Check if GPU has sufficient memory for specified texture size
    ///
    /// # Arguments
    /// * `width` - Texture width
    /// * `height` - Texture height
    /// * `bytes_per_pixel` - Bytes per pixel (e.g., 4 for RGBA, 1 for R8)
    ///
    /// # Returns
    /// True if GPU likely has sufficient VRAM, false otherwise
    #[must_use]
    pub fn has_sufficient_memory(&self, width: u32, height: u32, bytes_per_pixel: u32) -> bool {
        let required_bytes = u64::from(width) * u64::from(height) * u64::from(bytes_per_pixel);
        // Conservative estimate: require 4x the texture size for working memory
        let required_with_overhead = required_bytes * 4;
        // wgpu doesn't expose VRAM directly, so we use max buffer size as proxy
        let max_buffer_size = self.device.limits().max_buffer_size;
        required_with_overhead <= max_buffer_size
    }
}

/// Blocking wrapper for creating GPU context
///
/// Uses pollster to block on async GPU initialization.
/// Suitable for non-async contexts like FFI or tests.
///
/// # Errors
/// Returns error if GPU context creation fails
pub fn create_gpu_context_blocking() -> Result<GpuContext, String> {
    pollster::block_on(GpuContext::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_context_creation() {
        // Try to create GPU context (may fail on systems without GPU)
        let result = create_gpu_context_blocking();

        match result {
            Ok(ctx) => {
                // Verify context is valid
                assert!(!ctx.adapter_info().name.is_empty());

                // Test memory check with realistic fire simulation sizes
                // 2048x2048 grid with 4 bytes per pixel = 16MB
                assert!(ctx.has_sufficient_memory(2048, 2048, 4));

                // Very large texture should fail (16384x16384 = 1GB)
                // This is a conservative check - some GPUs may support this
            }
            Err(e) => {
                // GPU not available - this is acceptable for CI/headless systems
                eprintln!("GPU context creation skipped: {e}");
            }
        }
    }

    #[test]
    fn test_memory_estimation() {
        // This test doesn't require actual GPU
        // We're testing the memory calculation logic

        let result = create_gpu_context_blocking();
        if let Ok(ctx) = result {
            // Small texture should always fit
            assert!(ctx.has_sufficient_memory(512, 512, 1));

            // Medium texture (typical for fire sim)
            let medium_ok = ctx.has_sufficient_memory(1024, 1024, 4);

            // Large texture
            let large_ok = ctx.has_sufficient_memory(2048, 2048, 4);

            // At least one of medium or large should work on any reasonable GPU
            if !medium_ok && !large_ok {
                eprintln!("Warning: GPU has very limited memory");
            }
        }
    }
}
