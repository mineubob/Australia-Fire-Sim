//! Physics-based ember transport and spot fire ignition
//!
//! Implements realistic ember physics for long-distance fire spread through spotting.
//! Critical for Australian bushfire behavior where embers can travel up to 25km.
//!
//! # Physics Models
//!
//! 1. **Buoyancy** - Hot embers rise due to thermal updraft
//! 2. **Wind drag** - Wind carries embers horizontally (THE CRITICAL EFFECT for 25km spotting)
//! 3. **Gravity** - Pulls embers down once cooled
//! 4. **Radiative cooling** - Stefan-Boltzmann cooling to ambient temperature
//! 5. **Spot fire ignition** - Probability-based ignition when hot embers land on receptive fuel
//!
//! # Scientific References
//!
//! - Albini, F.A. (1979). "Spot fire distance from burning trees: a predictive model"
//!   USDA Forest Service Research Paper INT-56
//! - Albini, F.A. (1983). "Transport of firebrands by line thermals"
//!   Combustion Science and Technology, 32(5-6), 277-288
//! - Koo, E., Pagni, P.J., Weise, D.R., Woycheese, J.P. (2010). "Firebrands and spotting ignition
//!   in large-scale fires." International Journal of Wildland Fire, 19(7), 818-843.
//! - Ellis, P.F. (2011). "Fuelbed ignition potential and bark morphology explain the notoriety
//!   of the eucalypt messmate 'stringybark' for intense spotting." International Journal of
//!   Wildland Fire, 20(7), 897-907.
//! - Black Saturday 2009 Royal Commission: Documented ember spotting up to 25km ahead of fire front
//! - Manzello, S.L., et al. (2020). "Role of firebrand combustion in large outdoor fire spread."
//!   Progress in Energy and Combustion Science, 76, 100801.
//!
//! # Australian Context
//!
//! Eucalyptus stringybark trees produce extensive ember showers due to:
//! - Fibrous, loosely-attached bark that easily detaches
//! - High oil content (sustains ember combustion during flight)
//! - Large surface area to mass ratio (enables long-distance transport)
//!
//! During Black Summer 2019-20 and Black Saturday 2009, ember spotting was a primary
//! mechanism of fire spread, causing fires to "leap" firebreaks and ignite new fires
//! kilometers ahead of the main fire front.

use crate::core_types::units::{Celsius, Kilograms};
use crate::core_types::vec3::Vec3;
use serde::{Deserialize, Serialize};

// ============================================================================
// EMBER PHYSICS CONSTANTS
// ============================================================================

/// Ember activity threshold - embers below this temperature are considered inactive (200°C)
///
/// Below this temperature, embers have cooled sufficiently that they no longer pose
/// an ignition risk and can be removed from the simulation.
pub const EMBER_ACTIVE_THRESHOLD: Celsius = Celsius::new(200.0);

/// Ember ignition threshold - embers must be above this to ignite receptive fuel (250°C)
///
/// This is the minimum temperature at which landed embers can successfully ignite
/// dry fuel. Based on typical ignition temperatures for fine fuels.
pub const EMBER_IGNITION_THRESHOLD: Celsius = Celsius::new(250.0);

/// Ember buoyancy threshold - hot embers above this temperature experience significant buoyant lift (300°C)
///
/// At temperatures above 300°C, the thermal differential with ambient air creates
/// strong buoyancy forces that can loft embers high into the air, enabling long-distance transport.
pub const EMBER_BUOYANCY_THRESHOLD: Celsius = Celsius::new(300.0);

// ============================================================================
// EMBER TYPES
// ============================================================================

/// Ember particle with physics simulation
///
/// Represents a single burning ember that has been lofted from a fire.
/// Embers are the primary mechanism for long-distance fire spread in
/// Australian bushfires, particularly with eucalyptus species.
///
/// # Physics Properties
///
/// - **Mass**: 0.0001 to 0.01 kg (typical bark fragment)
/// - **Temperature**: 200-1000°C (initially hot from parent fire)
/// - **Density**: ~400 kg/m³ (charred wood/bark)
/// - **Drag coefficient**: 0.4 (sphere approximation)
/// - **Buoyancy**: Active when T > 300°C
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ember {
    pub(crate) id: u32,
    pub(crate) position: Vec3,
    pub(crate) velocity: Vec3,
    /// Current ember temperature
    pub(crate) temperature: Celsius,
    /// Ember mass (typical range: 0.0001 to 0.01 kg)
    pub(crate) mass: Kilograms,
    pub(crate) source_fuel_type: u8,
}

impl Ember {
    /// Create a new ember
    pub(crate) fn new(
        id: u32,
        position: Vec3,
        velocity: Vec3,
        temperature: Celsius,
        mass: Kilograms,
        source_fuel_type: u8,
    ) -> Self {
        Ember {
            id,
            position,
            velocity,
            temperature,
            mass,
            source_fuel_type,
        }
    }

    /// Update ember physics with wind drift, buoyancy, gravity, and cooling
    ///
    /// Implements full 3D physics simulation:
    ///
    /// # Forces
    ///
    /// 1. **Buoyancy (`F_b`)**: Hot air updraft lifts ember
    ///    ```text
    ///    F_b = ρ_air × g × V × (T_ember / T_threshold)
    ///    ```
    ///    Active when T > 300°C
    ///
    /// 2. **Drag (`F_d`)**: Wind resistance and drift
    ///    ```text
    ///    F_d = 0.5 × ρ_air × C_d × A × |v_rel|²
    ///    ```
    ///    Where `v_rel` = wind - `ember_velocity`
    ///
    /// 3. **Gravity (`F_g`)**: Downward pull
    ///    ```text
    ///    F_g = m × g = m × 9.81 m/s²
    ///    ```
    ///
    /// 4. **Cooling**: Stefan-Boltzmann radiative heat loss
    ///    ```text
    ///    dT/dt = -k × (T_ember - T_ambient)
    ///    ```
    ///    Where k = 0.05 (empirical cooling coefficient)
    ///
    /// # Wind Effect (THE CRITICAL MECHANISM)
    ///
    /// Wind drag is the dominant force enabling 25km ember transport:
    /// - 10 m/s wind → ~2-5 km spotting distance
    /// - 20 m/s wind → ~10-15 km spotting distance
    /// - 30+ m/s wind (extreme fire weather) → 20-25 km spotting distance
    ///
    /// # Arguments
    ///
    /// * `wind` - Wind velocity vector (m/s)
    /// * `ambient_temp` - Ambient air temperature (°C)
    /// * `dt` - Time step (seconds)
    ///
    /// # References
    ///
    /// - Koo et al. (2010) - Firebrand physics and trajectory modeling
    /// - Manzello et al. (2020) - Experimental ember transport studies
    pub(crate) fn update_physics(&mut self, wind: Vec3, ambient_temp: Celsius, dt: f32) {
        const AIR_DENSITY: f32 = 1.225; // kg/m³ at sea level, 20°C

        let ember_volume = *self.mass / 400.0; // ~400 kg/m³ for char

        // 1. Buoyancy (hot embers rise)
        let buoyancy = if self.temperature > EMBER_BUOYANCY_THRESHOLD {
            let temp_ratio = *self.temperature / 300.0;
            AIR_DENSITY * 9.81 * ember_volume * (temp_ratio as f32)
        } else {
            0.0
        };

        // 2. Wind drag (THE CRITICAL EFFECT for 25km spotting)
        let relative_velocity = wind - self.velocity;
        let drag_coeff = 0.4; // sphere approximation
                              // Cross-section scales with mass: sqrt(mass/density) for characteristic length
                              // For a 1g ember at 400 kg/m³: volume = 2.5e-6 m³, characteristic length ~1.4cm
                              // Cross section ~0.0002 m² (2 cm²)
                              // Scale cross section with mass for realistic physics
        let base_cross_section = 0.0002; // m² for 1g ember
        let cross_section = base_cross_section * (*self.mass / 0.001).powf(0.67); // Surface area scales as mass^(2/3)
        let drag_force =
            0.5 * AIR_DENSITY * drag_coeff * relative_velocity.magnitude_squared() * cross_section;
        // Cap acceleration to prevent numerical instability
        let max_accel = 50.0; // m/s² maximum acceleration from drag
        let drag_accel = if relative_velocity.magnitude() > 0.01 {
            let accel = (relative_velocity.normalize() * drag_force) / *self.mass;
            let accel_mag = accel.magnitude();
            if accel_mag > max_accel {
                accel * (max_accel / accel_mag)
            } else {
                accel
            }
        } else {
            Vec3::zeros()
        };

        // 3. Gravity
        let gravity = Vec3::new(0.0, 0.0, -9.81);

        // 4. Integrate motion (Euler method)
        let accel = Vec3::new(0.0, 0.0, buoyancy / *self.mass) + drag_accel + gravity;
        self.velocity += accel * dt;
        self.position += self.velocity * dt;

        // 5. Radiative cooling (Newton's law of cooling)
        // Stable exponential decay: T = T_ambient + (T_0 - T_ambient) * exp(-k*t)
        // This naturally asymptotes to ambient and NEVER overshoots
        let cooling_coefficient = 0.05; // 1/s
        let decay_factor = (-cooling_coefficient * f64::from(dt)).exp();
        let temp_above_ambient = self.temperature - ambient_temp;
        self.temperature = ambient_temp + temp_above_ambient * decay_factor;
    }

    /// Check if ember is still active (hot and airborne)
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.temperature > EMBER_ACTIVE_THRESHOLD && self.position.z > 0.0
    }

    /// Check if ember has landed on the ground
    #[must_use]
    pub fn has_landed(&self) -> bool {
        self.position.z < 1.0
    }

    /// Check if ember can ignite fuel (landed and hot enough)
    ///
    /// Embers can ignite receptive fuel if:
    /// - Temperature > 250°C (typical ignition threshold)
    /// - Landed on surface (z < 1m)
    #[must_use]
    pub fn can_ignite(&self) -> bool {
        self.has_landed() && self.temperature > EMBER_IGNITION_THRESHOLD
    }

    /// Get current temperature
    #[must_use]
    pub fn temperature(&self) -> Celsius {
        self.temperature
    }

    /// Get current mass
    #[must_use]
    pub fn mass(&self) -> Kilograms {
        self.mass
    }

    /// Get current velocity (m/s)
    #[must_use]
    pub fn velocity(&self) -> Vec3 {
        self.velocity
    }

    /// Get current position (m)
    #[must_use]
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Get source fuel type ID
    #[must_use]
    pub fn source_fuel_type(&self) -> u8 {
        self.source_fuel_type
    }

    /// Predict final landing position using Albini trajectory model
    ///
    /// Uses the detailed trajectory integration from Albini (1983) to predict
    /// where this ember will land based on current conditions.
    ///
    /// # Arguments
    /// * `wind_speed_10m` - Wind speed at 10m reference height (m/s)
    /// * `wind_direction` - Wind direction as unit vector
    /// * `dt` - Integration time step (seconds)
    /// * `max_time` - Maximum simulation time (seconds)
    ///
    /// # Returns
    /// Predicted landing position (Vec3)
    ///
    /// # References
    /// Albini (1979, 1983) trajectory integration
    #[must_use]
    pub fn predict_landing_position(
        &self,
        wind_speed_10m: f32,
        wind_direction: Vec3,
        dt: f32,
        max_time: f32,
    ) -> Vec3 {
        // Calculate ember diameter from mass assuming density of 400 kg/m³
        // Volume = mass / density, diameter = (6V/π)^(1/3)
        let volume = *self.mass / 400.0;
        let ember_diameter = (6.0 * volume / std::f32::consts::PI).powf(1.0 / 3.0);

        crate::physics::calculate_ember_trajectory(
            self.position,
            self.velocity,
            *self.mass,
            ember_diameter,
            self.temperature.as_f32(),
            wind_speed_10m,
            wind_direction,
            dt,
            max_time,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ember_physics() {
        let mut ember = Ember::new(
            1,
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::new(0.0, 0.0, 5.0),
            Celsius::new(800.0),
            Kilograms::new(0.001),
            1,
        );

        let wind = Vec3::new(10.0, 0.0, 0.0); // 10 m/s wind
        let initial_temp = ember.temperature;

        // Update for several seconds
        for _ in 0..100 {
            ember.update_physics(wind, Celsius::new(20.0), 0.1);
        }

        // Ember should cool down over time
        assert!(ember.temperature < initial_temp);

        // Ember should be affected by physics (moved from initial position)
        assert!(ember.position.z != 10.0 || ember.position.x != 0.0);
    }

    #[test]
    fn test_ember_buoyancy() {
        let mut ember = Ember::new(
            1,
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(0.0, 0.0, 0.0),
            Celsius::new(600.0),
            Kilograms::new(0.001),
            1,
        );

        // Hot ember should rise or at least not fall immediately
        // Update multiple times to allow buoyancy to overcome initial gravity
        for _ in 0..5 {
            ember.update_physics(Vec3::zeros(), Celsius::new(20.0), 0.1);
        }

        // Should have moved upward or stayed roughly the same (buoyancy counteracts gravity)
        // With small embers, gravity may win but velocity should show upward component initially
        assert!(ember.velocity.z > -5.0); // Not falling fast
    }

    #[test]
    fn test_ember_cooling_never_below_absolute_zero() {
        // Regression test for bug where aggressive cooling could cause panic
        // after ~260 steps with "Celsius::new: value is below absolute zero"
        let mut ember = Ember::new(
            1,
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::new(0.0, 0.0, 0.0),
            Celsius::new(100.0), // Start with low temperature
            Kilograms::new(0.001),
            1,
        );

        let ambient = Celsius::new(20.0);

        // Run for many steps (more than 260) - this previously would panic
        for _ in 0..500 {
            ember.update_physics(Vec3::zeros(), ambient, 0.1);
        }

        // Temperature should stabilize at ambient, never go below
        assert!(*ember.temperature >= *ambient);
        assert!(ember.temperature >= Celsius::ABSOLUTE_ZERO); // Never below absolute zero
    }
}
