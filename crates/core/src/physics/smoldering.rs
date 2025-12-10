//! Smoldering Combustion Phase Model
//!
//! Implements transition from flaming to smoldering combustion for fuel elements.
//! Important for extended burning duration and complete fuel consumption modeling.
//!
//! # Scientific References
//!
//! - Rein, G. (2009). "Smouldering combustion phenomena in science and technology"
//!   Progress in Energy and Combustion Science, 35(2), 141-200
//! - Ohlemiller, T.J. (1985). "Modeling of smoldering combustion propagation"
//!   Progress in Energy and Combustion Science, 11(4), 277-310
//! - Drysdale, D. (2011). "An Introduction to Fire Dynamics" 3rd Edition, Chapter 7
//!
//! # Model Overview
//!
//! Smoldering occurs when:
//! - Flames extinguish due to low temperature or insufficient oxygen
//! - Fuel continues to oxidize at lower temperatures (typically 200-700°C)
//! - Heat release rate is 10-100x lower than flaming
//! - Burning duration extends significantly (hours vs minutes)

use crate::core_types::units::Celsius;
use serde::{Deserialize, Serialize};

/// Combustion phase classification
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CombustionPhase {
    /// Not ignited yet
    Unignited,
    /// Active flaming combustion (>700°C typically)
    Flaming,
    /// Transition from flaming to smoldering
    Transition,
    /// Smoldering combustion (200-700°C)
    Smoldering,
    /// Burned out, no combustion
    Extinguished,
}

/// Smoldering combustion parameters
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SmolderingState {
    /// Current combustion phase
    phase: CombustionPhase,
    /// Heat release rate multiplier (relative to flaming)
    /// Flaming = 1.0, Smoldering = 0.01-0.1
    heat_release_multiplier: f32,
    /// Burn rate multiplier (slower in smoldering)
    burn_rate_multiplier: f32,
    /// Time spent in current phase (seconds)
    phase_duration: f32,
}

impl SmolderingState {
    /// Create new unignited state
    pub(crate) fn new() -> Self {
        SmolderingState {
            phase: CombustionPhase::Unignited,
            heat_release_multiplier: 0.0,
            burn_rate_multiplier: 0.0,
            phase_duration: 0.0,
        }
    }

    /// Create new flaming state (for explicit ignition)
    /// Use this when an element is directly ignited (e.g., by user action or ember)
    /// to bypass the Unignited phase
    pub(crate) fn new_flaming() -> Self {
        SmolderingState {
            phase: CombustionPhase::Flaming,
            heat_release_multiplier: 1.0,
            burn_rate_multiplier: 1.0,
            phase_duration: 0.0,
        }
    }

    /// Get the current combustion phase
    #[must_use]
    pub fn phase(&self) -> CombustionPhase {
        self.phase
    }

    /// Get the heat release multiplier
    pub(crate) fn heat_release_multiplier(&self) -> f32 {
        self.heat_release_multiplier
    }

    /// Get the burn rate multiplier
    pub(crate) fn burn_rate_multiplier(&self) -> f32 {
        self.burn_rate_multiplier
    }
}

impl Default for SmolderingState {
    fn default() -> Self {
        Self::new()
    }
}

/// Determine if fuel should transition to smoldering
///
/// Transition occurs when:
/// - Temperature drops below flaming threshold but above smoldering minimum
/// - Oxygen concentration is low (but not zero)
/// - Fuel has been burning for some time (allows initial flame establishment)
///
/// CRITICAL: A fire should only transition to smoldering when oxygen-limited.
/// Time alone is NOT sufficient - a well-ventilated fire will keep flaming!
///
/// # Arguments
/// * `temperature` - Current fuel temperature (°C)
/// * `oxygen_fraction` - Oxygen concentration (fraction 0-1, normal = 0.21)
/// * `time_burning` - Time fuel has been burning (seconds)
///
/// # Returns
/// True if should transition to smoldering
///
/// # References
/// Rein (2009), Drysdale (2011)
pub(crate) fn should_transition_to_smoldering(
    temperature: f32,
    oxygen_fraction: f32,
    time_burning: f32,
) -> bool {
    // Temperature in smoldering range (200-700°C)
    let in_smoldering_temp_range = (200.0..700.0).contains(&temperature);

    // Oxygen limited (but not depleted) - PRIMARY condition for smoldering transition
    let oxygen_limited = oxygen_fraction < 0.15 && oxygen_fraction > 0.05;

    // Has been burning long enough for transition to be possible
    // (initial 30s allows flame establishment before checking oxygen)
    let sufficient_duration = time_burning > 30.0;

    // CRITICAL: Require BOTH conditions - oxygen must be limited!
    // Previously used || which caused fires to smolder after 30s regardless of oxygen
    in_smoldering_temp_range && oxygen_limited && sufficient_duration
}

/// Calculate smoldering heat release rate multiplier
///
/// Smoldering has much lower heat release than flaming:
/// - Flaming: 1.0 (baseline)
/// - Smoldering: 0.01-0.1 (1-10% of flaming)
///
/// # Arguments
/// * `temperature` - Current temperature
/// * `oxygen_fraction` - Oxygen concentration (fraction 0-1)
///
/// # Returns
/// Heat release multiplier (0-1)
///
/// # References
/// Rein (2009) - Smoldering heat release is 10-100x lower than flaming
pub(crate) fn calculate_smoldering_heat_multiplier(
    temperature: Celsius,
    oxygen_fraction: f32,
) -> f32 {
    let temp = *temperature;
    if temp < 200.0 {
        return 0.0; // Too cold for smoldering
    }

    // Base multiplier depends on temperature
    let temp_factor = if temp < 400.0 {
        // Low temp smoldering: 1-3% of flaming
        0.01 + (temp - 200.0) / 200.0 * 0.02
    } else if temp < 700.0 {
        // High temp smoldering: 3-10% of flaming
        0.03 + (temp - 400.0) / 300.0 * 0.07
    } else {
        // Approaching flaming
        0.1
    };

    // Oxygen availability factor
    let oxygen_factor = (f64::from(oxygen_fraction) / 0.21).min(1.0);

    (temp_factor * oxygen_factor) as f32
}

/// Calculate smoldering burn rate multiplier
///
/// Smoldering burns 5-20x slower than flaming
///
/// # Arguments
/// * `temperature` - Current temperature
///
/// # Returns
/// Burn rate multiplier (0-1)
///
/// # References
/// Ohlemiller (1985)
pub(crate) fn calculate_smoldering_burn_rate_multiplier(temperature: Celsius) -> f32 {
    let temp = *temperature;
    if temp < 200.0 {
        0.0 // No smoldering
    } else if temp < 400.0 {
        // Very slow: 5-10% of flaming rate
        (0.05 + (temp - 200.0) / 200.0 * 0.05) as f32
    } else if temp < 700.0 {
        // Faster smoldering: 10-20% of flaming rate
        (0.10 + (temp - 400.0) / 300.0 * 0.10) as f32
    } else {
        // Near flaming transition
        0.20
    }
}

/// Update smoldering state based on current conditions
///
/// # Arguments
/// * `state` - Current smoldering state
/// * `temperature` - Current temperature (°C)
/// * `oxygen_fraction` - Oxygen concentration (fraction 0-1)
/// * `dt` - Time step (seconds)
///
/// # Returns
/// Updated smoldering state
pub(crate) fn update_smoldering_state(
    mut state: SmolderingState,
    temperature: f32,
    oxygen_fraction: f32,
    dt: f32,
) -> SmolderingState {
    state.phase_duration += dt;

    match state.phase {
        CombustionPhase::Unignited => {
            // Check for ignition (handled elsewhere)
            if temperature > 250.0 {
                state.phase = CombustionPhase::Flaming;
                state.heat_release_multiplier = 1.0;
                state.burn_rate_multiplier = 1.0;
                state.phase_duration = 0.0;
            }
        }

        CombustionPhase::Flaming => {
            // Check for transition to smoldering
            if should_transition_to_smoldering(temperature, oxygen_fraction, state.phase_duration) {
                state.phase = CombustionPhase::Transition;
                state.phase_duration = 0.0;
            }
            state.heat_release_multiplier = 1.0;
            state.burn_rate_multiplier = 1.0;
        }

        CombustionPhase::Transition => {
            // Transition phase lasts ~10 seconds
            if state.phase_duration > 10.0 {
                if temperature >= 200.0 {
                    state.phase = CombustionPhase::Smoldering;
                } else {
                    state.phase = CombustionPhase::Extinguished;
                }
                state.phase_duration = 0.0;
            }
            // Gradually reduce heat release
            let transition_factor = 1.0 - (state.phase_duration / 10.0);
            state.heat_release_multiplier = transition_factor;
            state.burn_rate_multiplier = transition_factor * 0.5;
        }

        CombustionPhase::Smoldering => {
            // Calculate smoldering parameters
            state.heat_release_multiplier = calculate_smoldering_heat_multiplier(
                Celsius::new(f64::from(temperature)),
                oxygen_fraction,
            );
            state.burn_rate_multiplier =
                calculate_smoldering_burn_rate_multiplier(Celsius::new(f64::from(temperature)));

            // Check for extinction
            if temperature < 200.0 || oxygen_fraction < 0.05 {
                state.phase = CombustionPhase::Extinguished;
                state.heat_release_multiplier = 0.0;
                state.burn_rate_multiplier = 0.0;
                state.phase_duration = 0.0;
            }

            // Check for re-ignition to flaming
            if temperature > 700.0 && oxygen_fraction > 0.15 {
                state.phase = CombustionPhase::Flaming;
                state.heat_release_multiplier = 1.0;
                state.burn_rate_multiplier = 1.0;
                state.phase_duration = 0.0;
            }
        }

        CombustionPhase::Extinguished => {
            // Stays extinguished
            state.heat_release_multiplier = 0.0;
            state.burn_rate_multiplier = 0.0;
        }
    }

    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_unignited() {
        let state = SmolderingState::new();
        assert_eq!(state.phase, CombustionPhase::Unignited);
        assert_eq!(state.heat_release_multiplier, 0.0);
    }

    #[test]
    fn test_transition_criteria() {
        // Should NOT transition: normal oxygen - fire stays flaming even with long duration
        // A well-ventilated fire will continue flaming, not smolder
        assert!(!should_transition_to_smoldering(400.0, 0.21, 60.0));

        // Should transition: oxygen limited (< 0.15) AND long duration
        assert!(should_transition_to_smoldering(400.0, 0.10, 60.0));

        // Should NOT transition: oxygen limited but short duration
        assert!(!should_transition_to_smoldering(400.0, 0.10, 10.0));

        // Should NOT transition: high temp (above smoldering range)
        assert!(!should_transition_to_smoldering(800.0, 0.10, 60.0));

        // Should NOT transition: too cold (below smoldering range)
        assert!(!should_transition_to_smoldering(150.0, 0.10, 60.0));

        // Should NOT transition: oxygen too depleted (below 0.05)
        assert!(!should_transition_to_smoldering(400.0, 0.03, 60.0));
    }

    #[test]
    fn test_smoldering_heat_multiplier() {
        // Low temp smoldering (1-3% of flaming)
        let mult_low = calculate_smoldering_heat_multiplier(Celsius::new(300.0), 0.21);
        assert!(mult_low > 0.01 && mult_low < 0.03);

        // High temp smoldering (3-10% of flaming)
        let mult_high = calculate_smoldering_heat_multiplier(Celsius::new(600.0), 0.21);
        assert!(mult_high > 0.03 && mult_high < 0.10);

        // With low oxygen, should be reduced
        let mult_low_oxygen = calculate_smoldering_heat_multiplier(Celsius::new(600.0), 0.10);
        assert!(mult_low_oxygen < mult_high);
    }

    #[test]
    fn test_smoldering_burn_rate() {
        // Low temp: 5-10% of flaming
        let rate_low = calculate_smoldering_burn_rate_multiplier(Celsius::new(300.0));
        assert!(rate_low > 0.05 && rate_low < 0.10);

        // High temp: 10-20% of flaming
        let rate_high = calculate_smoldering_burn_rate_multiplier(Celsius::new(600.0));
        assert!(rate_high > 0.10 && rate_high < 0.20);
    }

    #[test]
    fn test_state_update_ignition() {
        let mut state = SmolderingState::new();

        // Heat up to ignition
        state = update_smoldering_state(state, 300.0, 0.21, 1.0);

        assert_eq!(state.phase, CombustionPhase::Flaming);
        assert_eq!(state.heat_release_multiplier, 1.0);
    }

    #[test]
    fn test_state_update_smoldering_extinction() {
        let mut state = SmolderingState {
            phase: CombustionPhase::Smoldering,
            heat_release_multiplier: 0.05,
            burn_rate_multiplier: 0.10,
            phase_duration: 100.0,
        };

        // Cool down to extinction
        state = update_smoldering_state(state, 150.0, 0.21, 1.0);

        assert_eq!(state.phase, CombustionPhase::Extinguished);
        assert_eq!(state.heat_release_multiplier, 0.0);
    }

    #[test]
    fn test_smoldering_extends_burn_duration() {
        // Smoldering burn rate should be much lower than flaming
        let flaming_rate = 1.0;
        let smoldering_rate = calculate_smoldering_burn_rate_multiplier(Celsius::new(400.0));

        // At least 5x slower (up to 20x slower)
        assert!(smoldering_rate < flaming_rate / 5.0);
    }
}
