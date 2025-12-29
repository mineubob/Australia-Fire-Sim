//! Core types and utilities

pub mod atmospheric;
pub mod ember;
pub mod fuel;
pub mod noise;
pub mod units;
pub mod vec3;
pub mod weather;

// Re-export (atmospheric types for future use)
#[expect(unused_imports)]
pub(crate) use atmospheric::*;
pub use ember::*;
pub use fuel::*;
pub use noise::{FuelVariation, TurbulentWind};
pub use units::*;
pub use vec3::Vec3;
pub use weather::*;
