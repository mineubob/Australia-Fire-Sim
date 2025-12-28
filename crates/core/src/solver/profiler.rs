/// Performance profiling helpers for tracking simulation timing.
///
/// Provides RAII-style profiling scopes and timing utilities.
use std::time::Instant;

/// A profiling scope that measures elapsed time using RAII.
///
/// Time is automatically measured when dropped.
pub struct ProfilerScope {
    start: Instant,
    name: &'static str,
}

impl ProfilerScope {
    /// Creates a new profiling scope.
    pub fn new(name: &'static str) -> Self {
        Self {
            start: Instant::now(),
            name,
        }
    }

    /// Gets elapsed time in milliseconds.
    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

impl Drop for ProfilerScope {
    fn drop(&mut self) {
        let elapsed_ms = self.elapsed_ms();
        // Could log here if logging is enabled
        // For now, just measure silently
        let _ = elapsed_ms;
        let _ = self.name;
    }
}

/// Simple frame timer for tracking simulation performance.
pub struct FrameTimer {
    last_frame_time_ms: f64,
}

impl FrameTimer {
    /// Creates a new frame timer.
    pub fn new() -> Self {
        Self {
            last_frame_time_ms: 0.0,
        }
    }

    /// Records frame time in milliseconds.
    pub fn record(&mut self, time_ms: f64) {
        self.last_frame_time_ms = time_ms;
    }

    /// Gets the last recorded frame time.
    pub fn last_frame_time_ms(&self) -> f64 {
        self.last_frame_time_ms
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_profiler_scope_measures_time() {
        let scope = ProfilerScope::new("test");
        thread::sleep(Duration::from_millis(10));
        let elapsed = scope.elapsed_ms();
        assert!(elapsed >= 10.0, "Expected at least 10ms, got {elapsed}");
        assert!(elapsed < 50.0, "Expected less than 50ms, got {elapsed}");
    }

    #[test]
    fn test_frame_timer() {
        let mut timer = FrameTimer::new();
        assert_eq!(timer.last_frame_time_ms(), 0.0);

        timer.record(16.7);
        assert_eq!(timer.last_frame_time_ms(), 16.7);

        timer.record(8.3);
        assert_eq!(timer.last_frame_time_ms(), 8.3);
    }
}
