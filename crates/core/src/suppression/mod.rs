//! Fire suppression physics module
//!
//! Implements comprehensive suppression agent physics including:
//! - Multiple agent types with research-based properties
//! - Evaporation and degradation over time
//! - Fuel element coverage tracking
//! - Combustion inhibition and oxygen displacement
//!
//! # Scientific References
//!
//! - NFPA 1150: Standard on Foam Chemicals for Fires in Class A Fuels (2022)
//! - USFS MTDC: Long-Term Fire Retardant Effectiveness Studies (2019)
//! - FAO Irrigation and Drainage Paper 56: Penman-Monteith Evaporation
//! - George & Johnson (2009): "Effectiveness of Aerial Fire Retardant"

pub mod agent;
pub mod coverage; // Made pub for FFI access to SuppressionCoverage type

// Re-export SuppressionAgentType and SuppressionAgentProperties as public for FFI
pub use agent::{SuppressionAgentProperties, SuppressionAgentType};
pub use coverage::SuppressionCoverage;
