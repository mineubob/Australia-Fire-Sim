//! Suppression coverage tracking per fuel element
//!
//! Tracks the suppression agent coverage on individual fuel elements,
//! including evaporation, degradation, and effectiveness over time.
//!
//! # Scientific References
//!
//! - NFPA 1150: Standard on Foam Chemicals for Fires in Class A Fuels
//! - USFS MTDC: Long-Term Fire Retardant Effectiveness Studies
//! - Penman-Monteith equation (FAO Paper 56)

use super::agent::{SuppressionAgentProperties, SuppressionAgentType};
use serde::{Deserialize, Serialize};

/// Represents suppression agent coverage on a fuel element
///
/// This struct tracks the state of suppression coverage over time,
/// including mass remaining, effectiveness, and degradation.
///
/// # Physics Model
///
/// 1. **Heat absorption**: Evaporating agent absorbs latent heat (2260 kJ/kg for water)
/// 2. **Combustion inhibition**: Chemical retardants reduce reaction rate
/// 3. **Oxygen displacement**: Foam blankets exclude oxygen from fuel surface
/// 4. **Evaporation**: Agent mass decreases over time based on weather
/// 5. **UV degradation**: Foam/retardant effectiveness decreases in sunlight
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SuppressionCoverage {
    /// Type of suppression agent
    pub agent_type: SuppressionAgentType,

    /// Mass per unit area (kg/m²)
    /// Decreases over time due to evaporation
    pub mass_per_area: f32,

    /// Simulation time when coverage was applied
    pub application_time: f32,

    /// Fraction of fuel surface covered (0-1)
    /// Decreases due to UV degradation and evaporation
    pub coverage_fraction: f32,

    /// Depth of penetration into fuel bed (meters)
    /// Set at application, doesn't change over time
    pub penetration_depth: f32,

    /// Whether coverage is still effective
    pub active: bool,

    /// Remaining effectiveness of chemical inhibition (0-1)
    /// Decreases due to UV degradation
    pub chemical_effectiveness: f32,
}

impl SuppressionCoverage {
    /// Create new suppression coverage
    ///
    /// # Parameters
    /// - `agent_type`: Type of suppression agent
    /// - `mass_kg`: Total mass applied to the element
    /// - `fuel_surface_area`: Surface area of the fuel element (m²)
    /// - `simulation_time`: Current simulation time
    pub fn new(
        agent_type: SuppressionAgentType,
        mass_kg: f32,
        fuel_surface_area: f32,
        simulation_time: f32,
    ) -> Self {
        let props = SuppressionAgentProperties::for_type(agent_type);

        // Calculate mass per area
        let mass_per_area = mass_kg / fuel_surface_area.max(0.01);

        // Coverage fraction based on application rate
        // If applied at recommended rate, coverage = 1.0
        let coverage_fraction = (mass_per_area / props.application_rate).min(1.0);

        Self {
            agent_type,
            mass_per_area,
            application_time: simulation_time,
            coverage_fraction,
            penetration_depth: props.penetration_depth,
            active: true,
            chemical_effectiveness: 1.0,
        }
    }

    /// Modify incoming heat transfer based on suppression coverage
    ///
    /// Returns the effective heat after suppression effects are applied.
    ///
    /// # Physics
    ///
    /// 1. Heat absorbed by evaporating agent (latent heat)
    /// 2. Chemical combustion inhibition (retardants)
    /// 3. Oxygen displacement (foam blanketing)
    ///
    /// # Parameters
    /// - `incoming_heat_kj`: Heat energy arriving at fuel element
    /// - `fuel_surface_area`: Surface area of fuel element (m²)
    /// - `dt`: Time step (seconds)
    ///
    /// # Returns
    /// Tuple of (effective_heat_kj, heat_absorbed_kj)
    pub fn modify_heat_transfer(
        &mut self,
        incoming_heat_kj: f32,
        fuel_surface_area: f32,
        dt: f32,
    ) -> (f32, f32) {
        if !self.active || self.mass_per_area <= 0.0 {
            return (incoming_heat_kj, 0.0);
        }

        let props = SuppressionAgentProperties::for_type(self.agent_type);

        // 1. Heat absorbed by evaporating suppression agent
        let agent_mass = self.mass_per_area * fuel_surface_area;

        // How much heat can be absorbed this timestep
        // Limit evaporation rate to physical maximum
        let max_evap_rate_kg = agent_mass * 0.1 * dt; // Max 10% per second
        let max_heat_absorbed = max_evap_rate_kg * props.latent_heat_vaporization;

        let heat_for_evaporation = incoming_heat_kj.min(max_heat_absorbed);
        let agent_evaporated = heat_for_evaporation / props.latent_heat_vaporization;

        // Update mass (evaporation from heat)
        self.mass_per_area -= agent_evaporated / fuel_surface_area.max(0.01);
        self.mass_per_area = self.mass_per_area.max(0.0);

        // Remaining heat after evaporation
        let remaining_heat = incoming_heat_kj - heat_for_evaporation;

        // 2. Chemical combustion inhibition (retardants)
        let inhibition_factor =
            1.0 - (props.combustion_inhibition * self.coverage_fraction * self.chemical_effectiveness);

        // 3. Oxygen displacement (foam blanketing)
        let oxygen_factor = 1.0 - (props.oxygen_displacement * self.coverage_fraction);

        // Combined effectiveness
        let effective_heat = remaining_heat * inhibition_factor * oxygen_factor;

        (effective_heat, heat_for_evaporation)
    }

    /// Update coverage state over time (evaporation, degradation)
    ///
    /// # Parameters
    /// - `temperature`: Air temperature (°C)
    /// - `humidity`: Relative humidity (0-1)
    /// - `wind_speed`: Wind speed (m/s)
    /// - `solar_radiation`: Solar radiation (W/m²)
    /// - `dt`: Time step (seconds)
    pub fn update(
        &mut self,
        temperature: f32,
        humidity: f32,
        wind_speed: f32,
        solar_radiation: f32,
        dt: f32,
    ) {
        if !self.active {
            return;
        }

        let props = SuppressionAgentProperties::for_type(self.agent_type);

        // 1. Natural evaporation (Penman-Monteith based)
        let evap_rate = props.evaporation_rate(temperature, humidity, wind_speed, solar_radiation);
        self.mass_per_area -= evap_rate * dt;

        // 2. UV degradation of foam/retardant effectiveness
        if solar_radiation > 200.0 {
            // Only significant in bright conditions
            let uv_factor = (solar_radiation / 1000.0).min(1.0);
            let degradation = props.uv_degradation_rate * uv_factor * (dt / 3600.0);
            self.coverage_fraction -= degradation;
            self.chemical_effectiveness -= degradation * 0.5; // Chemical degrades slower
        }

        // 3. Coating duration check (for retardants)
        // This is handled by the simulation checking time since application

        // Clamp values
        self.mass_per_area = self.mass_per_area.max(0.0);
        self.coverage_fraction = self.coverage_fraction.clamp(0.0, 1.0);
        self.chemical_effectiveness = self.chemical_effectiveness.clamp(0.0, 1.0);

        // Deactivate if depleted
        if self.mass_per_area < 0.01 || self.coverage_fraction < 0.05 {
            self.active = false;
        }
    }

    /// Check if suppression can prevent ember ignition
    ///
    /// # Returns
    /// - Ignition probability modifier (0 = blocked, 1 = no effect)
    pub fn ember_ignition_modifier(&self) -> f32 {
        if !self.active {
            return 1.0;
        }

        let props = SuppressionAgentProperties::for_type(self.agent_type);

        // Coverage blocks ember contact with fuel
        let coverage_block = self.coverage_fraction;

        // Chemical inhibition reduces ignition probability
        let chemical_block = props.combustion_inhibition * self.chemical_effectiveness;

        // Moisture content increase from suppression
        let moisture_effect = if self.mass_per_area > 0.5 {
            0.5 // Significant moisture barrier
        } else {
            self.mass_per_area // Proportional effect
        };

        // Combined blocking effect (1 - combined probability of NOT blocking)
        let total_block = 1.0 - ((1.0 - coverage_block) * (1.0 - chemical_block) * (1.0 - moisture_effect));

        // Return modifier (lower = more blocking)
        1.0 - total_block.clamp(0.0, 0.95) // Never completely block (5% minimum chance)
    }

    /// Calculate additional fuel moisture from suppression
    ///
    /// # Returns
    /// Additional moisture fraction to add to fuel element
    pub fn moisture_contribution(&self) -> f32 {
        if !self.active || self.mass_per_area <= 0.0 {
            return 0.0;
        }

        // Water-based agents increase fuel moisture
        // ~0.1 kg/m² agent adds ~0.05 moisture fraction
        (self.mass_per_area * 0.5).min(0.3) // Cap at 30% moisture increase
    }

    /// Check if coating is still within effective duration
    ///
    /// # Parameters
    /// - `current_time`: Current simulation time
    ///
    /// # Returns
    /// True if coating is within effective duration
    pub fn is_within_duration(&self, current_time: f32) -> bool {
        let props = SuppressionAgentProperties::for_type(self.agent_type);
        let elapsed = current_time - self.application_time;
        elapsed <= props.fuel_coating_duration
    }

    /// Get remaining effectiveness as a percentage
    pub fn effectiveness_percent(&self) -> f32 {
        if !self.active {
            return 0.0;
        }

        // Combine coverage and chemical effectiveness
        (self.coverage_fraction * 0.4 + self.chemical_effectiveness * 0.6) * 100.0
    }
}

impl Default for SuppressionCoverage {
    fn default() -> Self {
        Self {
            agent_type: SuppressionAgentType::Water,
            mass_per_area: 0.0,
            application_time: 0.0,
            coverage_fraction: 0.0,
            penetration_depth: 0.0,
            active: false,
            chemical_effectiveness: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_creation() {
        let coverage = SuppressionCoverage::new(
            SuppressionAgentType::Water,
            5.0,  // 5 kg
            2.0,  // 2 m²
            0.0,  // time 0
        );

        assert!(coverage.active);
        assert!((coverage.mass_per_area - 2.5).abs() < 0.01); // 5/2 = 2.5 kg/m²
        assert!(coverage.coverage_fraction > 0.0);
    }

    #[test]
    fn test_heat_absorption() {
        let mut coverage = SuppressionCoverage::new(
            SuppressionAgentType::Water,
            10.0,  // 10 kg
            1.0,   // 1 m²
            0.0,
        );

        let incoming = 5000.0; // 5000 kJ
        let (effective, absorbed) = coverage.modify_heat_transfer(incoming, 1.0, 0.1);

        // Some heat should be absorbed
        assert!(absorbed > 0.0);
        // Effective heat should be reduced
        assert!(effective < incoming);
    }

    #[test]
    fn test_foam_oxygen_displacement() {
        let mut foam_coverage = SuppressionCoverage::new(
            SuppressionAgentType::FoamClassA,
            5.0,  // 5 kg
            1.0,  // 1 m²
            0.0,
        );

        let mut water_coverage = SuppressionCoverage::new(
            SuppressionAgentType::Water,
            5.0,
            1.0,
            0.0,
        );

        let incoming = 1000.0;
        let (foam_effective, _) = foam_coverage.modify_heat_transfer(incoming, 1.0, 0.1);
        let (water_effective, _) = water_coverage.modify_heat_transfer(incoming, 1.0, 0.1);

        // Foam should reduce heat more than water (oxygen displacement)
        assert!(
            foam_effective < water_effective,
            "Foam ({}) should reduce heat more than water ({})",
            foam_effective,
            water_effective
        );
    }

    #[test]
    fn test_evaporation_over_time() {
        let mut coverage = SuppressionCoverage::new(
            SuppressionAgentType::Water,
            5.0,
            1.0,
            0.0,
        );

        let initial_mass = coverage.mass_per_area;

        // Simulate hot, dry conditions for 60 seconds
        for _ in 0..60 {
            coverage.update(
                35.0,   // 35°C
                0.2,    // 20% humidity
                5.0,    // 5 m/s wind
                800.0,  // Bright sun
                1.0,    // 1 second
            );
        }

        // Mass should decrease
        assert!(
            coverage.mass_per_area < initial_mass,
            "Mass should decrease from {} to less",
            initial_mass
        );
    }

    #[test]
    fn test_ember_ignition_blocking() {
        let heavy_coverage = SuppressionCoverage::new(
            SuppressionAgentType::FoamClassA,
            10.0,  // Heavy coverage
            1.0,
            0.0,
        );

        let light_coverage = SuppressionCoverage::new(
            SuppressionAgentType::Water,
            0.1,   // Very light coverage - minimal effect expected
            1.0,
            0.0,
        );

        let no_coverage = SuppressionCoverage::default();

        // Heavy foam should block more
        assert!(heavy_coverage.ember_ignition_modifier() < 0.3);

        // Very light water should have less blocking
        assert!(
            light_coverage.ember_ignition_modifier() > heavy_coverage.ember_ignition_modifier(),
            "Light coverage ({}) should block less than heavy ({})",
            light_coverage.ember_ignition_modifier(),
            heavy_coverage.ember_ignition_modifier()
        );

        // No coverage = no blocking
        assert!((no_coverage.ember_ignition_modifier() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_retardant_long_duration() {
        let retardant = SuppressionCoverage::new(
            SuppressionAgentType::LongTermRetardant,
            5.0,
            1.0,
            0.0,
        );

        // Should be within duration at 4 hours
        assert!(retardant.is_within_duration(4.0 * 3600.0));

        // Should be within duration at 7 hours
        assert!(retardant.is_within_duration(7.0 * 3600.0));

        // Should be outside duration at 10 hours
        assert!(!retardant.is_within_duration(10.0 * 3600.0));
    }

    #[test]
    fn test_moisture_contribution() {
        let wet_coverage = SuppressionCoverage::new(
            SuppressionAgentType::Water,
            5.0,  // Heavy water
            1.0,
            0.0,
        );

        let dry_coverage = SuppressionCoverage::new(
            SuppressionAgentType::Water,
            0.1,  // Light water
            1.0,
            0.0,
        );

        // Heavy coverage should add more moisture
        assert!(wet_coverage.moisture_contribution() > dry_coverage.moisture_contribution());

        // Capped at 30%
        assert!(wet_coverage.moisture_contribution() <= 0.3);
    }

    #[test]
    fn test_coverage_deactivation() {
        let mut coverage = SuppressionCoverage::new(
            SuppressionAgentType::Water,
            0.1,  // Small amount
            1.0,
            0.0,
        );

        // Evaporate rapidly
        for _ in 0..100 {
            coverage.update(45.0, 0.1, 10.0, 1000.0, 1.0);
        }

        // Should be deactivated
        assert!(!coverage.active, "Coverage should deactivate when depleted");
    }
}
