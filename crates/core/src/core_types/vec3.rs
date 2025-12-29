//! Vector type alias for 3D positions and directions.

use nalgebra::Vector3;

/// 3D vector type for positions, velocities, and directions.
///
/// This is a simple alias for `nalgebra::Vector3<f32>`, used throughout
/// the simulation for world positions, wind vectors, and ember trajectories.
pub type Vec3 = Vector3<f32>;
