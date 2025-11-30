//! Core types and utilities

pub mod atmospheric;
pub mod element;
pub mod ember;
pub mod fuel;
pub(crate) mod spatial;
pub mod weather;

// Re-export
pub(crate) use atmospheric::*;
pub use element::*;
pub use ember::*;
pub use fuel::*;
// SpatialIndex is internal only, don't re-export (accessed within crate)
pub use weather::*;
