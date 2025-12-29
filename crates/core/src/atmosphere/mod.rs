//! Atmospheric dynamics for extreme fire behavior.
//!
//! This module models pyroconvection phenomena including:
//! - Convection column dynamics (Byram 1959, Briggs 1975)
//! - Atmospheric instability indices (Haines Index)
//! - Downdrafts and gust fronts
//! - Fire whirl formation conditions
//! - Pyrocumulonimbus (pyroCb) detection
//!
//! # Scientific Background
//!
//! Extreme bushfires generate their own weather through pyroconvection:
//! - Convection columns can rise 5-15 km into the atmosphere
//! - Pyrocumulonimbus (pyroCb) clouds can generate lightning and downbursts
//! - Downdrafts create erratic, dangerous fire behavior
//! - Fire whirls occur at the base of intense convection columns
//!
//! # References
//!
//! - Byram, G.M. (1959). "Combustion of forest fuels." Forest Fires: Control and Use.
//! - Briggs, G.A. (1975). "Plume rise predictions." NOAA.
//! - Byers, H.R. & Braham, R.R. (1949). "The Thunderstorm." U.S. Weather Bureau.
//! - Clark, T.L. et al. (1996). "Coupled atmosphere-fire model." IJWF.
//! - Fromm, M. et al. (2006). "Pyrocumulonimbus injection of smoke to the stratosphere."
//! - `McRae`, R.H.D. et al. (2013). Fire weather and fire danger in the 2003 Canberra fires.

mod convection;
mod downdraft;
mod fire_whirl;
mod instability;
mod pyrocb;

pub use convection::{ConvectionColumn, AIR_DENSITY, GRAVITY, SPECIFIC_HEAT_AIR};
pub use downdraft::Downdraft;
pub use fire_whirl::FireWhirlDetector;
pub use instability::AtmosphericStability;
pub use pyrocb::{PyroCbEvent, PyroCbSystem};
