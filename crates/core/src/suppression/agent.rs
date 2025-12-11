//! Suppression agent types and their physical properties
//!
//! # Scientific References
//!
//! - NFPA 1150: Standard on Foam Chemicals for Fires in Class A Fuels (2022)
//! - USFS MTDC: Long-Term Fire Retardant Effectiveness Studies (2019)
//! - FAO Irrigation Paper 56: Penman-Monteith evaporation equation

use crate::core_types::units::{Celsius, Percent};
use serde::{Deserialize, Serialize};

/// Types of suppression agents with different physical properties
///
/// Each agent type has unique characteristics that affect firefighting effectiveness:
/// - **Water**: Basic suppression through evaporative cooling
/// - **`FoamClassA`**: Enhanced penetration and coverage for wildland fuels
/// - **`ShortTermRetardant`**: Water-based gel with temporary retardant effect
/// - **`LongTermRetardant`**: Phosphate-based coating with long-lasting protection
/// - **`WettingAgent`**: Surfactant-enhanced water for better fuel penetration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuppressionAgentType {
    /// Pure water - basic suppression
    Water = 0,
    /// Class A foam for wildland fires (NFPA 1150)
    FoamClassA = 1,
    /// Short-term retardant (water-based gel)
    ShortTermRetardant = 2,
    /// Long-term retardant (Phos-Chek, etc.)
    LongTermRetardant = 3,
    /// Wetting agent (surfactant-enhanced water)
    WettingAgent = 4,
}

impl SuppressionAgentType {
    /// Convert from u8 for FFI
    #[must_use]
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Water),
            1 => Some(Self::FoamClassA),
            2 => Some(Self::ShortTermRetardant),
            3 => Some(Self::LongTermRetardant),
            4 => Some(Self::WettingAgent),
            _ => None,
        }
    }

    /// Convert to u8 for FFI
    #[must_use]
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Physical properties of suppression agents
///
/// All values based on peer-reviewed research and industry standards.
///
/// # Scientific References
///
/// - Water latent heat: 2260 kJ/kg (standard physics)
/// - Foam effectiveness: NFPA 1150, USFS studies (3-5x water)
/// - Retardant duration: USFS MTDC studies (4-8 hours)
/// - Evaporation: FAO Penman-Monteith equation
#[derive(Debug, Clone, Copy)]
pub struct SuppressionAgentProperties {
    // ═══════════════════════════════════════════════════════════════════
    // THERMAL PROPERTIES
    // ═══════════════════════════════════════════════════════════════════
    /// Specific heat capacity (kJ/(kg·K))
    /// Water: 4.18, Foam: ~4.0, Retardant: ~3.5
    specific_heat: f32,

    /// Latent heat of vaporization (kJ/kg)
    /// Water: 2260 kJ/kg at 100°C
    /// Foam/retardant have similar base water properties
    latent_heat_vaporization: f32,

    /// Boiling point (°C)
    /// Water: 100°C, may be slightly higher for retardants
    boiling_point: f32,

    // ═══════════════════════════════════════════════════════════════════
    // COVERAGE PROPERTIES
    // ═══════════════════════════════════════════════════════════════════
    /// Recommended application rate (kg/m²)
    /// Water: 2-4 kg/m², Foam: 0.5-1 kg/m², Retardant: 1-3 kg/m²
    application_rate: f32,

    /// Penetration depth into fuel bed (meters)
    /// How deep the agent penetrates into litter/duff
    penetration_depth: f32,

    // ═══════════════════════════════════════════════════════════════════
    // CHEMICAL PROPERTIES
    // ═══════════════════════════════════════════════════════════════════
    /// Combustion inhibition factor (0-1)
    /// Retardants chemically inhibit combustion reactions
    /// Water: 0.0, Short-term: 0.3, Long-term: 0.6
    combustion_inhibition: f32,

    /// Oxygen displacement factor (0-1)
    /// Foam blankets exclude oxygen from fuel surface
    /// Water: 0.0, Foam: 0.7-0.8
    oxygen_displacement: f32,

    /// Fuel coating duration (seconds)
    /// How long the protective coating remains effective
    /// Short-term: 1800s (30 min), Long-term: 28800s (8 hours)
    fuel_coating_duration: f32,

    // ═══════════════════════════════════════════════════════════════════
    // EVAPORATION & DEGRADATION
    // ═══════════════════════════════════════════════════════════════════
    /// Evaporation rate modifier relative to water (0-2)
    /// Water: 1.0, Foam: 0.3 (slower), Retardant: 0.5
    evaporation_rate_modifier: f32,

    /// UV degradation rate (fraction per hour under full sun)
    /// Foam/retardant degrade under UV exposure
    /// Water: 0.0, Foam: 0.15, Retardant: 0.05
    uv_degradation_rate: f32,
}

impl SuppressionAgentProperties {
    /// Get properties for Water
    ///
    /// Basic fire suppression through evaporative cooling.
    /// 2260 kJ/kg latent heat of vaporization is the primary mechanism.
    pub const WATER: Self = Self {
        // Thermal
        specific_heat: 4.18,
        latent_heat_vaporization: 2260.0,
        boiling_point: 100.0,
        // Coverage
        application_rate: 3.0,   // kg/m² typical ground application
        penetration_depth: 0.02, // 2cm typical
        // Chemical
        combustion_inhibition: 0.0, // No chemical inhibition
        oxygen_displacement: 0.0,   // No oxygen exclusion
        fuel_coating_duration: 0.0, // No lasting coating
        // Evaporation
        evaporation_rate_modifier: 1.0,
        uv_degradation_rate: 0.0,
    };

    /// Get properties for Class A Foam
    ///
    /// Enhanced wildland fire suppression (NFPA 1150).
    /// 3-5x more effective than plain water due to:
    /// - Better fuel penetration (surfactant lowers surface tension)
    /// - Oxygen exclusion (foam blanket)
    /// - Slower evaporation (insulating layer)
    pub const FOAM_CLASS_A: Self = Self {
        // Thermal
        specific_heat: 4.0,
        latent_heat_vaporization: 2260.0, // Water base
        boiling_point: 100.0,
        // Coverage - much more efficient than water
        application_rate: 0.8,   // kg/m² (less needed)
        penetration_depth: 0.05, // 5cm (surfactant helps)
        // Chemical
        combustion_inhibition: 0.1,    // Slight inhibition
        oxygen_displacement: 0.75,     // 75% oxygen exclusion
        fuel_coating_duration: 1800.0, // 30 min
        // Evaporation - slower due to foam structure
        evaporation_rate_modifier: 0.3, // 70% slower
        uv_degradation_rate: 0.15,      // Degrades in sun
    };

    /// Get properties for Short-Term Retardant
    ///
    /// Water-based gel that provides temporary fire protection.
    /// Effective for 30-60 minutes after application.
    pub const SHORT_TERM_RETARDANT: Self = Self {
        // Thermal
        specific_heat: 3.8,
        latent_heat_vaporization: 2400.0, // Slightly higher (gel)
        boiling_point: 105.0,
        // Coverage
        application_rate: 1.5,
        penetration_depth: 0.03,
        // Chemical
        combustion_inhibition: 0.35,   // Moderate inhibition
        oxygen_displacement: 0.2,      // Some exclusion
        fuel_coating_duration: 3600.0, // 1 hour
        // Evaporation
        evaporation_rate_modifier: 0.5,
        uv_degradation_rate: 0.10,
    };

    /// Get properties for Long-Term Retardant
    ///
    /// Phosphate-based retardant (e.g., Phos-Chek) that provides
    /// long-lasting fire protection (4-8 hours). Used in aerial drops.
    ///
    /// # Scientific Reference
    /// USFS MTDC: Fire retardant remains effective for 4-8 hours
    /// under typical conditions, reducing fire intensity by 40-60%.
    pub const LONG_TERM_RETARDANT: Self = Self {
        // Thermal
        specific_heat: 3.5,
        latent_heat_vaporization: 1800.0, // Lower (phosphate salts)
        boiling_point: 110.0,
        // Coverage
        application_rate: 2.0,
        penetration_depth: 0.04,
        // Chemical - primary mechanism is combustion inhibition
        combustion_inhibition: 0.6,     // 60% combustion reduction
        oxygen_displacement: 0.1,       // Minimal O2 exclusion
        fuel_coating_duration: 28800.0, // 8 hours (USFS research)
        // Evaporation
        evaporation_rate_modifier: 0.4,
        uv_degradation_rate: 0.05, // Slow UV degradation
    };

    /// Get properties for Wetting Agent
    ///
    /// Surfactant-enhanced water for better fuel penetration.
    /// 50-70% reduction in surface tension improves:
    /// - Penetration into deep litter/duff
    /// - Coating of hydrophobic fuels (waxy leaves)
    pub const WETTING_AGENT: Self = Self {
        // Thermal (similar to water)
        specific_heat: 4.15,
        latent_heat_vaporization: 2260.0,
        boiling_point: 100.0,
        // Coverage - better penetration
        application_rate: 2.0,
        penetration_depth: 0.08, // 8cm (deep penetration)
        // Chemical
        combustion_inhibition: 0.05,  // Slight
        oxygen_displacement: 0.05,    // Slight
        fuel_coating_duration: 600.0, // 10 min
        // Evaporation
        evaporation_rate_modifier: 0.9,
        uv_degradation_rate: 0.0,
    };

    /// Get properties for a given agent type
    pub(crate) fn for_type(agent_type: SuppressionAgentType) -> Self {
        match agent_type {
            SuppressionAgentType::Water => Self::WATER,
            SuppressionAgentType::FoamClassA => Self::FOAM_CLASS_A,
            SuppressionAgentType::ShortTermRetardant => Self::SHORT_TERM_RETARDANT,
            SuppressionAgentType::LongTermRetardant => Self::LONG_TERM_RETARDANT,
            SuppressionAgentType::WettingAgent => Self::WETTING_AGENT,
        }
    }

    /// Get application rate
    pub(crate) fn application_rate(&self) -> f32 {
        self.application_rate
    }

    /// Get penetration depth
    pub(crate) fn penetration_depth(&self) -> f32 {
        self.penetration_depth
    }

    /// Get oxygen displacement factor
    #[must_use]
    pub fn oxygen_displacement(&self) -> f32 {
        self.oxygen_displacement
    }

    /// Get combustion inhibition factor
    pub(crate) fn combustion_inhibition(&self) -> f32 {
        self.combustion_inhibition
    }

    /// Get fuel coating duration
    pub(crate) fn fuel_coating_duration(&self) -> f32 {
        self.fuel_coating_duration
    }

    /// Get UV degradation rate
    pub(crate) fn uv_degradation_rate(&self) -> f32 {
        self.uv_degradation_rate
    }

    /// Calculate total cooling capacity (kJ/kg)
    ///
    /// Includes both sensible heat (temperature rise) and latent heat (vaporization).
    /// For water at 20°C: 4.18 × (100-20) + 2260 = 2594 kJ/kg
    ///
    /// # Scientific Reference
    /// NFPA 1145: Guide for the Use of Class A Foams in Fire Fighting
    #[must_use]
    pub fn cooling_capacity(&self, agent_temp: Celsius) -> f32 {
        let sensible = self.specific_heat * (self.boiling_point - *agent_temp as f32).max(0.0);
        self.latent_heat_vaporization + sensible
    }

    /// Calculate evaporation rate using simplified Penman-Monteith
    ///
    /// Returns evaporation rate in kg/(m²·s)
    ///
    /// # Parameters
    /// - `temperature`: Air temperature
    /// - `humidity`: Relative humidity
    /// - `wind_speed`: Wind speed (m/s)
    /// - `solar_radiation`: Solar radiation (W/m²)
    ///
    /// # Scientific Reference
    /// FAO Irrigation and Drainage Paper 56 (1998)
    pub(crate) fn evaporation_rate(
        &self,
        temperature: Celsius,
        humidity: Percent,
        wind_speed: f32,
        solar_radiation: f32,
    ) -> f32 {
        // Simplified Penman-Monteith for water evaporation
        // Based on FAO-56 reference evapotranspiration

        let temp = *temperature;
        let humidity_fraction = *humidity.to_fraction();

        // Saturation vapor pressure (kPa) at temperature
        let e_sat = 0.6108 * ((17.27 * temp) / (temp + 237.3)).exp();

        // Actual vapor pressure (kPa)
        let e_act = e_sat * f64::from(humidity_fraction);

        // Vapor pressure deficit (kPa)
        let vpd = (e_sat - e_act).max(0.0) as f32;

        // Base evaporation rate (kg/(m²·s))
        // Calibrated to give ~1-5 mm/hour under typical conditions
        let solar_factor = (solar_radiation / 500.0).min(1.0);
        let wind_factor = 1.0 + wind_speed * 0.1;
        let base_rate = 0.0001 * vpd * solar_factor * wind_factor;

        // Apply agent-specific modifier
        base_rate * self.evaporation_rate_modifier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_water_cooling_capacity() {
        let water = SuppressionAgentProperties::WATER;
        let cooling = water.cooling_capacity(Celsius::new(20.0));

        // Water at 20°C: 4.18 × 80 + 2260 = 2594.4 kJ/kg
        assert!((cooling - 2594.4).abs() < 1.0);
    }

    #[test]
    fn test_foam_more_effective_than_water() {
        let foam = SuppressionAgentProperties::FOAM_CLASS_A;
        let water = SuppressionAgentProperties::WATER;

        // Foam should have higher cooling capacity due to better properties
        // (Though cooling_capacity is similar, the oxygen displacement makes foam more effective)
        let foam_cooling = foam.cooling_capacity(Celsius::new(20.0));
        let water_cooling = water.cooling_capacity(Celsius::new(20.0));
        // Water and foam have similar latent heat, so cooling capacity is similar
        assert!((foam_cooling - water_cooling).abs() < 500.0);

        // Foam should have oxygen displacement (the key differentiator)
        assert!(foam.oxygen_displacement() > 0.5);
    }

    #[test]
    fn test_retardant_combustion_inhibition() {
        let lt_retardant = SuppressionAgentProperties::LONG_TERM_RETARDANT;

        // Long-term retardant should have high combustion inhibition
        assert!(lt_retardant.combustion_inhibition() >= 0.5);

        // Should last 4-8 hours
        assert!(lt_retardant.fuel_coating_duration() >= 4.0 * 3600.0);
    }

    #[test]
    fn test_evaporation_rate_vpd_effect() {
        let water = SuppressionAgentProperties::WATER;

        // Low humidity = high evaporation
        let evap_dry = water.evaporation_rate(Celsius::new(30.0), Percent::new(20.0), 2.0, 500.0);

        // High humidity = low evaporation
        let evap_humid = water.evaporation_rate(Celsius::new(30.0), Percent::new(80.0), 2.0, 500.0);

        assert!(
            evap_dry > evap_humid * 3.0,
            "Dry conditions should evaporate faster"
        );
    }

    #[test]
    fn test_agent_type_conversion() {
        assert_eq!(
            SuppressionAgentType::from_u8(0),
            Some(SuppressionAgentType::Water)
        );
        assert_eq!(
            SuppressionAgentType::from_u8(1),
            Some(SuppressionAgentType::FoamClassA)
        );
        assert_eq!(
            SuppressionAgentType::from_u8(3),
            Some(SuppressionAgentType::LongTermRetardant)
        );
        assert_eq!(SuppressionAgentType::from_u8(99), None);
    }
}
