//! Fire Simulation Core Library
//! 
//! A scientifically accurate wildfire simulation system based on Australian bushfire research.
//! Implements physics-based fire spread, ember generation, and Australian-specific behaviors
//! like eucalyptus oil explosions and stringybark ladder fuels.

pub mod fuel;
pub mod element;
pub mod spatial;
pub mod physics;
pub mod weather;
pub mod ember;
pub mod australian;
pub mod simulation;

// Re-export main types
pub use fuel::{Fuel, BarkType};
pub use element::{FuelElement, FuelPart, Vec3};
pub use weather::WeatherSystem;
pub use ember::Ember;
pub use simulation::FireSimulation;
