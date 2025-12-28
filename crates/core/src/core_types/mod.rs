//! Core types and utilities

pub mod atmospheric;
pub mod element;
pub mod ember;
pub mod fuel;
pub mod noise;
pub(crate) mod spatial;
pub mod units;
pub mod weather;

// Re-export (atmospheric types for future use)
#[expect(unused_imports)]
pub(crate) use atmospheric::*;
pub use element::*;
pub use ember::*;
pub use fuel::*;
pub use noise::{FuelVariation, TurbulentWind};
// SpatialIndex is internal only, don't re-export (accessed within crate)
pub use units::*;
pub use weather::*;
