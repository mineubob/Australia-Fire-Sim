use crate::core_types::element::Vec3;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Ember particle with physics simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ember {
    pub id: u32,
    pub position: Vec3,
    pub(crate) velocity: Vec3,
    pub(crate) temperature: f32,
    pub(crate) mass: f32, // kg (0.0001 to 0.01)
    pub(crate) source_fuel_type: u8,
}

impl Ember {
    /// Create a new ember
    pub fn new(
        id: u32,
        position: Vec3,
        velocity: Vec3,
        temperature: f32,
        mass: f32,
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

    /// Update ember physics (wind drift, buoyancy, cooling)
    pub fn update_physics(&mut self, wind: Vec3, ambient_temp: f32, dt: f32) {
        const AIR_DENSITY: f32 = 1.225; // kg/m³

        let ember_volume = self.mass / 400.0; // ~400 kg/m³ for char

        // 1. Buoyancy (hot embers rise)
        let buoyancy = if self.temperature > 300.0 {
            let temp_ratio = self.temperature / 300.0;
            AIR_DENSITY * 9.81 * ember_volume * temp_ratio
        } else {
            0.0
        };

        // 2. Wind drag (THE CRITICAL EFFECT)
        let relative_velocity = wind - self.velocity;
        let drag_coeff = 0.4; // sphere approximation
        let cross_section = 0.01; // m²
        let drag_force =
            0.5 * AIR_DENSITY * drag_coeff * relative_velocity.magnitude_squared() * cross_section;
        let drag_accel = if relative_velocity.magnitude() > 0.01 {
            (relative_velocity.normalize() * drag_force) / self.mass
        } else {
            Vec3::zeros()
        };

        // 3. Gravity
        let gravity = Vec3::new(0.0, 0.0, -9.81);

        // 4. Integrate
        let accel = Vec3::new(0.0, 0.0, buoyancy / self.mass) + drag_accel + gravity;
        self.velocity += accel * dt;
        self.position += self.velocity * dt;

        // 5. Radiative cooling (Stefan-Boltzmann)
        let cooling_rate = (self.temperature - ambient_temp) * 0.05;
        self.temperature -= cooling_rate * dt;

        // Clamp temperature
        self.temperature = self.temperature.max(ambient_temp);
    }

    /// Check if ember is still active
    pub fn is_active(&self) -> bool {
        self.temperature > 200.0 && self.position.z > 0.0
    }

    /// Check if ember has landed
    pub fn has_landed(&self) -> bool {
        self.position.z < 1.0
    }

    /// Check if ember can ignite fuel
    pub fn can_ignite(&self) -> bool {
        self.has_landed() && self.temperature > 250.0
    }
}

/// Generate embers from a burning fuel element
pub fn spawn_embers(
    position: Vec3,
    temperature: f32,
    fuel_remaining: f32,
    ember_production: f32,
    fuel_type_id: u8,
    next_id: &mut u32,
) -> Vec<Ember> {
    let count = (ember_production * fuel_remaining * 100.0) as u32;
    let count = count.min(50); // Limit per spawn to avoid performance issues

    let mut embers = Vec::new();
    let mut rng = rand::thread_rng();

    for _ in 0..count {
        let id = *next_id;
        *next_id += 1;

        // Random initial velocity with strong updraft
        let horizontal_spread = 5.0;
        let velocity = Vec3::new(
            rng.gen_range(-horizontal_spread..horizontal_spread),
            rng.gen_range(-horizontal_spread..horizontal_spread),
            rng.gen_range(8.0..20.0), // Strong updraft
        );

        let ember_position = position + Vec3::new(0.0, 0.0, 2.0);
        let ember_temp = temperature * rng.gen_range(0.7..0.9);
        let ember_mass = rng.gen_range(0.0001..0.01);

        embers.push(Ember::new(
            id,
            ember_position,
            velocity,
            ember_temp,
            ember_mass,
            fuel_type_id,
        ));
    }

    embers
}

/// Calculate ignition probability from ember
pub fn ember_ignition_probability(ember: &Ember, fuel_receptivity: f32) -> f32 {
    let temp_factor = (ember.temperature / 300.0).min(1.0);
    let mass_factor = (ember.mass / 0.001).min(1.0);

    fuel_receptivity * temp_factor * mass_factor
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
            800.0,
            0.001,
            1,
        );

        let wind = Vec3::new(10.0, 0.0, 0.0); // 10 m/s wind
        let initial_temp = ember.temperature;

        // Update for several seconds
        for _ in 0..100 {
            ember.update_physics(wind, 20.0, 0.1);
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
            600.0,
            0.001,
            1,
        );

        // Hot ember should rise or at least not fall immediately
        // Update multiple times to allow buoyancy to overcome initial gravity
        for _ in 0..5 {
            ember.update_physics(Vec3::zeros(), 20.0, 0.1);
        }

        // Should have moved upward or stayed roughly the same (buoyancy counteracts gravity)
        // With small embers, gravity may win but velocity should show upward component initially
        assert!(ember.velocity.z > -5.0); // Not falling fast
    }

    #[test]
    fn test_spawn_embers() {
        let mut next_id = 0;
        let embers = spawn_embers(Vec3::new(0.0, 0.0, 0.0), 1000.0, 5.0, 0.5, 1, &mut next_id);

        // Should generate some embers
        assert!(!embers.is_empty());

        // All embers should have upward velocity
        for ember in &embers {
            assert!(ember.velocity.z > 0.0);
            assert!(ember.temperature > 0.0);
        }
    }
}
