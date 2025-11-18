//! Legacy simulation modules

pub mod simulation;
pub mod legacy_physics;
pub mod australian;
pub mod pyrocumulonimbus;

// Re-export
pub use simulation::*;
pub use legacy_physics::*;
pub use australian::*;
pub use pyrocumulonimbus::*;
