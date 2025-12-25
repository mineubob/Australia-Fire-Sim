//! GPU-Accelerated Fire Front Computation
//!
//! This module provides GPU-accelerated level set method for real-time fire front propagation.
//! Uses wgpu for cross-platform GPU compute (Vulkan/Metal/DX12).
//!
//! # Performance Targets
//! - 2048×2048 grid at 5m resolution: <5ms GPU time
//! - 60 FPS with 10km² fire area
//! - <256MB GPU memory usage
//!
//! # Determinism
//! - Fixed-point arithmetic in shaders for multiplayer consistency
//! - Same inputs produce same outputs across different GPU vendors
//! - Validated against CPU reference implementation

pub mod arrival_time;
pub mod context;
pub mod level_set;
pub mod rothermel;

pub use arrival_time::{predict_arrival_time, ArrivalPrediction};
pub use context::GpuContext;
pub use level_set::{CpuLevelSetSolver, LevelSetSolver};
pub use rothermel::GpuRothermelSolver;
