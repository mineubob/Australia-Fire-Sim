//! Legacy simulation modules

pub mod australian;
pub mod legacy_physics;
pub mod pyrocumulonimbus;
pub mod simulation;

// Re-export
pub use australian::*;
pub use legacy_physics::*;
pub use pyrocumulonimbus::*;
pub use simulation::*;
