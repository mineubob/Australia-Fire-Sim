//! Advanced Weather Phenomena Module (Phase 2)
//!
//! Implements atmospheric instability modeling, pyrocumulus cloud formation,
//! and fire-weather feedback loops for realistic extreme fire behavior.
//!
//! # Scientific References
//!
//! - Haines, D.A. (1988). "A lower atmosphere severity index for wildland fires."
//!   National Weather Digest, 13(2), 23-27.
//! - Fromm, M., et al. (2010). "Pyro-cumulonimbus injection of smoke to the
//!   stratosphere." Annales Geophysicae, 28(4), 937-963.
//! - ICAO Standard Atmosphere (1993) for atmospheric profile modeling.

pub(crate) mod atmosphere;
pub(crate) mod pyrocumulus;

pub(crate) use atmosphere::AtmosphericProfile;
pub(crate) use pyrocumulus::PyrocumulusCloud;
