//! Fire Simulation Core Library
//!
//! A scientifically accurate wildfire simulation system based on Australian bushfire research.
//! Implements physics-based fire spread, ember generation, and Australian-specific behaviors
//! like eucalyptus oil explosions and stringybark ladder fuels.

pub mod australian;
pub mod element;
pub mod ember;
pub mod fuel;
pub mod physics;
pub mod pyrocumulonimbus;
pub mod simulation;
pub mod spatial;
pub mod weather;

// Re-export main types
pub use element::{FuelElement, FuelPart, Vec3};
pub use ember::Ember;
pub use fuel::{BarkProperties, Fuel};
pub use pyrocumulonimbus::{Downdraft, LightningStrike, PyroCb, PyroCbSystem};
pub use simulation::FireSimulation;
pub use weather::{ClimatePattern, WeatherPreset, WeatherSystem};
