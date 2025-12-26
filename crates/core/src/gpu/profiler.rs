//! GPU performance profiling for level set fire front simulation
//!
//! This module provides GPU profiling capabilities to track compute shader
//! dispatch times and optimize performance across different GPU vendors.
//!
//! Target budget: 8ms total GPU time for 60 FPS operation.

use std::collections::HashMap;
use std::time::Instant;

/// GPU performance statistics for a single shader dispatch
#[derive(Debug, Clone)]
pub struct DispatchStats {
    /// Name of the shader/operation
    pub name: String,
    /// Time taken for the dispatch in microseconds
    pub time_us: u64,
    /// Number of workgroups dispatched
    pub workgroup_count: (u32, u32, u32),
    /// Grid dimensions being processed
    pub grid_size: (u32, u32),
}

/// GPU profiler for tracking performance of compute shader dispatches
pub struct GpuProfiler {
    /// Current frame dispatch stats
    frame_stats: Vec<DispatchStats>,
    /// Historical average times per operation (name -> avg_us)
    averages: HashMap<String, f64>,
    /// Number of samples for each operation
    sample_counts: HashMap<String, usize>,
    /// Total GPU time for current frame
    frame_gpu_time_us: u64,
    /// Target GPU time budget in microseconds (8ms = 8000us)
    target_budget_us: u64,
    /// Current quality preset
    quality_preset: QualityPreset,
}

/// Quality presets for adaptive resolution scaling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityPreset {
    /// Ultra: 2048×2048 grid
    Ultra,
    /// High: 2048×2048 grid
    High,
    /// Medium: 1024×1024 grid
    Medium,
    /// Low: 512×512 grid
    Low,
}

impl QualityPreset {
    /// Get grid resolution for this preset
    pub fn grid_resolution(&self) -> u32 {
        match self {
            QualityPreset::Ultra => 2048,
            QualityPreset::High => 2048,
            QualityPreset::Medium => 1024,
            QualityPreset::Low => 512,
        }
    }

    /// Get texture quality for this preset
    pub fn texture_quality(&self) -> TextureQuality {
        match self {
            QualityPreset::Ultra => TextureQuality::Uncompressed,
            QualityPreset::High => TextureQuality::BC4,
            QualityPreset::Medium => TextureQuality::BC4,
            QualityPreset::Low => TextureQuality::BC4,
        }
    }
}

/// Texture compression quality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureQuality {
    /// No compression (4 bytes per pixel for f32)
    Uncompressed,
    /// BC4 compression (1 byte per pixel for single channel)
    BC4,
}

impl GpuProfiler {
    /// Create a new GPU profiler with default 8ms budget
    pub fn new() -> Self {
        Self::with_budget(8000) // 8ms default
    }

    /// Create a GPU profiler with custom budget in microseconds
    pub fn with_budget(budget_us: u64) -> Self {
        Self {
            frame_stats: Vec::new(),
            averages: HashMap::new(),
            sample_counts: HashMap::new(),
            frame_gpu_time_us: 0,
            target_budget_us: budget_us,
            quality_preset: QualityPreset::High,
        }
    }

    /// Begin profiling a new frame
    pub fn begin_frame(&mut self) {
        self.frame_stats.clear();
        self.frame_gpu_time_us = 0;
    }

    /// Record a dispatch operation (call this before dispatch)
    pub fn begin_dispatch(&self, name: &str) -> DispatchTimer {
        DispatchTimer {
            name: name.to_string(),
            start: Instant::now(),
        }
    }

    /// End a dispatch operation and record stats
    pub fn end_dispatch(
        &mut self,
        timer: DispatchTimer,
        workgroup_count: (u32, u32, u32),
        grid_size: (u32, u32),
    ) {
        let elapsed = timer.start.elapsed();
        let time_us = elapsed.as_micros() as u64;

        let stats = DispatchStats {
            name: timer.name.clone(),
            time_us,
            workgroup_count,
            grid_size,
        };

        self.frame_gpu_time_us += time_us;
        self.frame_stats.push(stats);

        // Update running average
        let count = self.sample_counts.entry(timer.name.clone()).or_insert(0);
        *count += 1;
        let avg = self.averages.entry(timer.name).or_insert(0.0);
        *avg = (*avg * (*count - 1) as f64 + time_us as f64) / *count as f64;
    }

    /// End frame and return performance stats
    pub fn end_frame(&mut self) -> GpuStats {
        let stats = GpuStats {
            total_gpu_time_us: self.frame_gpu_time_us,
            target_budget_us: self.target_budget_us,
            dispatches: self.frame_stats.clone(),
            quality_preset: self.quality_preset,
            over_budget: self.frame_gpu_time_us > self.target_budget_us,
        };

        // Auto-adjust quality if consistently over budget
        if stats.over_budget {
            self.adjust_quality_down();
        } else if self.frame_gpu_time_us < self.target_budget_us / 2 {
            self.adjust_quality_up();
        }

        stats
    }

    /// Get current quality preset
    pub fn quality_preset(&self) -> QualityPreset {
        self.quality_preset
    }

    /// Set quality preset manually
    pub fn set_quality_preset(&mut self, preset: QualityPreset) {
        self.quality_preset = preset;
    }

    /// Adjust quality down if over budget
    fn adjust_quality_down(&mut self) {
        self.quality_preset = match self.quality_preset {
            QualityPreset::Ultra => QualityPreset::High,
            QualityPreset::High => QualityPreset::Medium,
            QualityPreset::Medium => QualityPreset::Low,
            QualityPreset::Low => QualityPreset::Low, // Can't go lower
        };
    }

    /// Adjust quality up if under budget
    fn adjust_quality_up(&mut self) {
        self.quality_preset = match self.quality_preset {
            QualityPreset::Low => QualityPreset::Medium,
            QualityPreset::Medium => QualityPreset::High,
            QualityPreset::High => QualityPreset::Ultra,
            QualityPreset::Ultra => QualityPreset::Ultra, // Already max
        };
    }

    /// Get average time for a specific operation
    pub fn get_average(&self, name: &str) -> Option<f64> {
        self.averages.get(name).copied()
    }
}

impl Default for GpuProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer for a single dispatch operation
pub struct DispatchTimer {
    name: String,
    start: Instant,
}

/// GPU performance statistics for a frame
#[derive(Debug, Clone)]
pub struct GpuStats {
    /// Total GPU time for the frame in microseconds
    pub total_gpu_time_us: u64,
    /// Target budget in microseconds
    pub target_budget_us: u64,
    /// Individual dispatch stats
    pub dispatches: Vec<DispatchStats>,
    /// Current quality preset
    pub quality_preset: QualityPreset,
    /// Whether frame was over budget
    pub over_budget: bool,
}

impl GpuStats {
    /// Get total GPU time in milliseconds
    pub fn total_gpu_time_ms(&self) -> f64 {
        self.total_gpu_time_us as f64 / 1000.0
    }

    /// Get target budget in milliseconds
    pub fn target_budget_ms(&self) -> f64 {
        self.target_budget_us as f64 / 1000.0
    }

    /// Get budget utilization as percentage
    pub fn budget_utilization(&self) -> f64 {
        (self.total_gpu_time_us as f64 / self.target_budget_us as f64) * 100.0
    }

    /// Check if performance is acceptable
    pub fn is_acceptable(&self) -> bool {
        !self.over_budget
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_profiler_basic() {
        let mut profiler = GpuProfiler::new();

        profiler.begin_frame();

        let timer = profiler.begin_dispatch("test_shader");
        thread::sleep(Duration::from_micros(100));
        profiler.end_dispatch(timer, (64, 64, 1), (1024, 1024));

        let stats = profiler.end_frame();

        assert_eq!(stats.dispatches.len(), 1);
        assert!(stats.total_gpu_time_us >= 100);
        assert_eq!(stats.target_budget_us, 8000);
    }

    #[test]
    fn test_quality_preset_resolution() {
        assert_eq!(QualityPreset::Ultra.grid_resolution(), 2048);
        assert_eq!(QualityPreset::High.grid_resolution(), 2048);
        assert_eq!(QualityPreset::Medium.grid_resolution(), 1024);
        assert_eq!(QualityPreset::Low.grid_resolution(), 512);
    }

    #[test]
    fn test_budget_utilization() {
        let stats = GpuStats {
            total_gpu_time_us: 4000,
            target_budget_us: 8000,
            dispatches: vec![],
            quality_preset: QualityPreset::High,
            over_budget: false,
        };

        assert_eq!(stats.budget_utilization(), 50.0);
        assert!(stats.is_acceptable());
    }

    #[test]
    fn test_auto_quality_adjustment() {
        let mut profiler = GpuProfiler::with_budget(1000);
        profiler.set_quality_preset(QualityPreset::High);

        profiler.begin_frame();
        let timer = profiler.begin_dispatch("expensive");
        thread::sleep(Duration::from_micros(2000));
        profiler.end_dispatch(timer, (128, 128, 1), (2048, 2048));

        let _stats = profiler.end_frame();

        // Should have adjusted down due to being over budget
        assert_eq!(profiler.quality_preset(), QualityPreset::Medium);
    }
}
