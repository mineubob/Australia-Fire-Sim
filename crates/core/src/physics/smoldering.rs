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
    pub phase: CombustionPhase,
    /// Heat release rate multiplier (relative to flaming)
    /// Flaming = 1.0, Smoldering = 0.01-0.1
    pub heat_release_multiplier: f32,
    /// Burn rate multiplier (slower in smoldering)
    pub burn_rate_multiplier: f32,
    /// Time spent in current phase (seconds)
    pub phase_duration: f32,
}

impl SmolderingState {
    /// Create new unignited state
    pub fn new() -> Self {
        SmolderingState {
            phase: CombustionPhase::Unignited,
            heat_release_multiplier: 0.0,
            burn_rate_multiplier: 0.0,
            phase_duration: 0.0,
        }
    }
    
    /// Create flaming state
    pub fn flaming() -> Self {
        SmolderingState {
            phase: CombustionPhase::Flaming,
            heat_release_multiplier: 1.0,
            burn_rate_multiplier: 1.0,
            phase_duration: 0.0,
        }
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
/// - Fuel has been burning for some time
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
pub fn should_transition_to_smoldering(
    temperature: f32,
    oxygen_fraction: f32,
    time_burning: f32,
) -> bool {
    // Temperature in smoldering range (200-700°C)
    let in_smoldering_temp_range = temperature >= 200.0 && temperature < 700.0;
    
    // Oxygen limited (but not depleted)
    let oxygen_limited = oxygen_fraction < 0.15 && oxygen_fraction > 0.05;
    
    // Has been burning for at least 30 seconds (fuel well-established)
    let sufficient_duration = time_burning > 30.0;
    
    in_smoldering_temp_range && (oxygen_limited || sufficient_duration)
}

/// Calculate smoldering heat release rate multiplier
///
/// Smoldering has much lower heat release than flaming:
/// - Flaming: 1.0 (baseline)
/// - Smoldering: 0.01-0.1 (1-10% of flaming)
///
/// # Arguments
/// * `temperature` - Current temperature (°C)
/// * `oxygen_fraction` - Oxygen concentration (fraction 0-1)
///
/// # Returns
/// Heat release multiplier (0-1)
///
/// # References
/// Rein (2009) - Smoldering heat release is 10-100x lower than flaming
pub fn calculate_smoldering_heat_multiplier(
    temperature: f32,
    oxygen_fraction: f32,
) -> f32 {
    if temperature < 200.0 {
        return 0.0; // Too cold for smoldering
    }
    
    // Base multiplier depends on temperature
    let temp_factor = if temperature < 400.0 {
        // Low temp smoldering: 1-3% of flaming
        0.01 + (temperature - 200.0) / 200.0 * 0.02
    } else if temperature < 700.0 {
        // High temp smoldering: 3-10% of flaming
        0.03 + (temperature - 400.0) / 300.0 * 0.07
    } else {
        // Approaching flaming
        0.1
    };
    
    // Oxygen availability factor
    let oxygen_factor = (oxygen_fraction / 0.21).min(1.0);
    
    temp_factor * oxygen_factor
}

/// Calculate smoldering burn rate multiplier
///
/// Smoldering burns 5-20x slower than flaming
///
/// # Arguments
/// * `temperature` - Current temperature (°C)
///
/// # Returns
/// Burn rate multiplier (0-1)
///
/// # References
/// Ohlemiller (1985)
pub fn calculate_smoldering_burn_rate_multiplier(temperature: f32) -> f32 {
    if temperature < 200.0 {
        0.0 // No smoldering
    } else if temperature < 400.0 {
        // Very slow: 5-10% of flaming rate
        0.05 + (temperature - 200.0) / 200.0 * 0.05
    } else if temperature < 700.0 {
        // Faster smoldering: 10-20% of flaming rate
        0.10 + (temperature - 400.0) / 300.0 * 0.10
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
pub fn update_smoldering_state(
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
            state.heat_release_multiplier = calculate_smoldering_heat_multiplier(temperature, oxygen_fraction);
            state.burn_rate_multiplier = calculate_smoldering_burn_rate_multiplier(temperature);
            
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
    fn test_flaming_state() {
        let state = SmolderingState::flaming();
        assert_eq!(state.phase, CombustionPhase::Flaming);
        assert_eq!(state.heat_release_multiplier, 1.0);
    }

    #[test]
    fn test_transition_criteria() {
        // Should transition: low temp, normal oxygen, long duration
        assert!(should_transition_to_smoldering(400.0, 0.21, 60.0));
        
        // Should transition: normal temp, low oxygen
        assert!(should_transition_to_smoldering(500.0, 0.10, 10.0));
        
        // Should NOT transition: high temp
        assert!(!should_transition_to_smoldering(800.0, 0.21, 60.0));
        
        // Should NOT transition: too cold
        assert!(!should_transition_to_smoldering(150.0, 0.21, 60.0));
    }

    #[test]
    fn test_smoldering_heat_multiplier() {
        // Low temp smoldering (1-3% of flaming)
        let mult_low = calculate_smoldering_heat_multiplier(300.0, 0.21);
        assert!(mult_low > 0.01 && mult_low < 0.03);
        
        // High temp smoldering (3-10% of flaming)
        let mult_high = calculate_smoldering_heat_multiplier(600.0, 0.21);
        assert!(mult_high > 0.03 && mult_high < 0.10);
        
        // With low oxygen, should be reduced
        let mult_low_oxygen = calculate_smoldering_heat_multiplier(600.0, 0.10);
        assert!(mult_low_oxygen < mult_high);
    }

    #[test]
    fn test_smoldering_burn_rate() {
        // Low temp: 5-10% of flaming
        let rate_low = calculate_smoldering_burn_rate_multiplier(300.0);
        assert!(rate_low > 0.05 && rate_low < 0.10);
        
        // High temp: 10-20% of flaming
        let rate_high = calculate_smoldering_burn_rate_multiplier(600.0);
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
    fn test_state_update_flaming_to_smoldering() {
        let mut state = SmolderingState::flaming();
        
        // Simulate long flaming period with oxygen depletion
        for _ in 0..40 {
            state = update_smoldering_state(state, 450.0, 0.10, 1.0);
        }
        
        // Should have transitioned to smoldering
        assert!(state.phase == CombustionPhase::Transition || state.phase == CombustionPhase::Smoldering);
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
        let smoldering_rate = calculate_smoldering_burn_rate_multiplier(400.0);
        
        // At least 5x slower (up to 20x slower)
        assert!(smoldering_rate < flaming_rate / 5.0);
    }
}
