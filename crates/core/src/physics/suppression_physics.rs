//! Fire suppression physics modeling
//!
//! Implements realistic suppression agent behavior including:
//! - Water droplet physics with wind drift
//! - Retardant coverage and effectiveness
//! - Foam expansion and heat absorption
//! - Aircraft/ground drop modeling

use crate::core_types::element::Vec3;
use crate::grid::SimulationGrid;
use serde::{Deserialize, Serialize};

/// Types of suppression agents
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SuppressionAgent {
    Water,
    ShortTermRetardant,
    LongTermRetardant,
    Foam,
}

/// Suppression droplet/particle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppressionDroplet {
    pub position: Vec3,
    pub velocity: Vec3,
    pub mass: f32, // kg
    pub agent_type: SuppressionAgent,
    pub temperature: f32, // °C
    pub active: bool,
}

impl SuppressionDroplet {
    /// Create a new suppression droplet
    pub fn new(position: Vec3, velocity: Vec3, mass: f32, agent_type: SuppressionAgent) -> Self {
        SuppressionDroplet {
            position,
            velocity,
            mass,
            agent_type,
            temperature: 20.0,
            active: true,
        }
    }

    /// Update droplet physics (gravity, wind drift, evaporation)
    pub fn update(&mut self, wind: Vec3, ambient_temp: f32, dt: f32) {
        if !self.active {
            return;
        }

        const AIR_DENSITY: f32 = 1.2; // kg/m³
        const GRAVITY: f32 = 9.81; // m/s²

        // Estimate droplet size from mass (assuming spherical water droplet)
        let droplet_volume = self.mass / self.droplet_density();
        let droplet_radius = (3.0 * droplet_volume / (4.0 * std::f32::consts::PI)).powf(1.0 / 3.0);
        let cross_section = std::f32::consts::PI * droplet_radius * droplet_radius;

        // Drag coefficient (sphere approximation)
        let drag_coeff = 0.47;

        // Wind drift (relative velocity)
        let relative_velocity = wind - self.velocity;
        let drag_force =
            0.5 * AIR_DENSITY * drag_coeff * cross_section * relative_velocity.magnitude_squared();
        let drag_accel = if relative_velocity.magnitude() > 0.0 {
            relative_velocity.normalize() * (drag_force / self.mass)
        } else {
            Vec3::zeros()
        };

        // Gravity
        let gravity_accel = Vec3::new(0.0, 0.0, -GRAVITY);

        // Update velocity and position
        self.velocity += (drag_accel + gravity_accel) * dt;
        self.position += self.velocity * dt;

        // Evaporation (for water)
        if matches!(self.agent_type, SuppressionAgent::Water) && ambient_temp > 50.0 {
            let evap_rate = (ambient_temp - 50.0) * 0.0001; // kg/s per °C above 50
            let evaporated = evap_rate * dt;
            self.mass -= evaporated;

            if self.mass < 0.001 {
                self.active = false;
            }
        }

        // Deactivate if below ground
        if self.position.z < 0.0 {
            self.active = false;
        }
    }

    /// Get droplet density based on agent type
    fn droplet_density(&self) -> f32 {
        match self.agent_type {
            SuppressionAgent::Water => 1000.0,              // kg/m³
            SuppressionAgent::ShortTermRetardant => 1100.0, // Slightly denser
            SuppressionAgent::LongTermRetardant => 1150.0,  // With additives
            SuppressionAgent::Foam => 100.0,                // Expanded foam
        }
    }

    /// Get cooling effectiveness (kJ per kg)
    pub fn cooling_capacity(&self) -> f32 {
        match self.agent_type {
            SuppressionAgent::Water => {
                // Latent heat of vaporization: 2260 kJ/kg
                // Sensible heat: ~4.18 kJ/(kg·K) × temp rise
                let sensible = 4.18 * (100.0 - self.temperature);
                2260.0 + sensible
            }
            SuppressionAgent::ShortTermRetardant => {
                // Similar to water but with fire-retarding chemicals
                2500.0
            }
            SuppressionAgent::LongTermRetardant => {
                // Long-term retardants form coating
                1800.0 + 500.0 // Cooling + coating benefit
            }
            SuppressionAgent::Foam => {
                // Foam insulates and cools
                3000.0 // High effectiveness due to coverage
            }
        }
    }

    /// Get coverage effectiveness (0-1 scale per kg/m²)
    pub fn coverage_effectiveness(&self) -> f32 {
        match self.agent_type {
            SuppressionAgent::Water => 0.3,              // Evaporates quickly
            SuppressionAgent::ShortTermRetardant => 0.6, // Better coverage
            SuppressionAgent::LongTermRetardant => 0.9,  // Excellent coverage
            SuppressionAgent::Foam => 0.95,              // Best coverage
        }
    }
}

/// Aircraft drop parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AircraftDrop {
    pub position: Vec3, // Drop release position
    pub velocity: Vec3, // Aircraft velocity
    pub altitude: f32,  // Meters above terrain
    pub volume: f32,    // Liters
    pub agent_type: SuppressionAgent,
    pub drop_rate: f32, // kg/s
}

impl AircraftDrop {
    /// Generate droplets from aircraft drop
    pub fn generate_droplets(&self, dt: f32) -> Vec<SuppressionDroplet> {
        let mut droplets = Vec::new();

        // Mass dropped this timestep
        let total_mass = self.drop_rate * dt;

        // Droplet mass distribution (variable sizes)
        let num_droplets = (total_mass / 0.5).ceil() as usize; // ~0.5 kg per droplet
        let droplet_mass = total_mass / num_droplets as f32;

        for i in 0..num_droplets {
            // Spread droplets along drop trajectory
            let spread_factor = (i as f32 / num_droplets as f32) - 0.5;
            let lateral_spread = spread_factor * 20.0; // 20m spread

            let pos = self.position + Vec3::new(lateral_spread, 0.0, 0.0);

            // Initial velocity from aircraft + downward component
            let vel = self.velocity
                + Vec3::new(
                    0.0, 0.0, -5.0, // Initial downward velocity
                );

            droplets.push(SuppressionDroplet::new(
                pos,
                vel,
                droplet_mass,
                self.agent_type,
            ));
        }

        droplets
    }
}

/// Ground engine/hose suppression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundSuppression {
    pub position: Vec3,
    pub direction: Vec3, // Normalized direction
    pub flow_rate: f32,  // kg/s
    pub agent_type: SuppressionAgent,
    pub range: f32, // Effective range (m)
    pub active: bool,
}

impl GroundSuppression {
    /// Generate droplets from ground suppression
    pub fn generate_droplets(&self, dt: f32) -> Vec<SuppressionDroplet> {
        if !self.active {
            return Vec::new();
        }

        let mut droplets = Vec::new();

        let total_mass = self.flow_rate * dt;
        let num_droplets = (total_mass / 0.1).ceil() as usize; // Smaller droplets
        let droplet_mass = total_mass / num_droplets as f32;

        // Spray pattern
        let base_velocity = self.direction * 15.0; // 15 m/s stream velocity

        for i in 0..num_droplets {
            // Cone spray pattern
            let angle = (i as f32 / num_droplets as f32) * 2.0 * std::f32::consts::PI;
            let cone_angle = 0.2; // Radians

            let lateral = Vec3::new(cone_angle * angle.cos(), cone_angle * angle.sin(), 0.0);

            let vel = base_velocity + lateral * 5.0;

            droplets.push(SuppressionDroplet::new(
                self.position,
                vel,
                droplet_mass,
                self.agent_type,
            ));
        }

        droplets
    }
}

/// Apply suppression to grid
pub(crate) fn apply_suppression_to_grid(droplet: &SuppressionDroplet, grid: &mut SimulationGrid) {
    if !droplet.active {
        return;
    }

    // Get values we need before borrowing mutably
    let cell_volume = grid.cell_size.powi(3);
    let ambient_temp = grid.ambient_temperature;

    if let Some(cell) = grid.cell_at_position_mut(droplet.position) {
        // Add suppression agent to cell
        let concentration_increase = droplet.mass / cell_volume;

        cell.suppression_agent += concentration_increase;

        // Cooling effect
        let cooling_kj = droplet.mass * droplet.cooling_capacity();
        let air_mass = cell.air_density() * cell_volume;
        let specific_heat_air = 1.005; // kJ/(kg·K)
        let temp_drop = cooling_kj / (air_mass * specific_heat_air);

        cell.temperature = (cell.temperature - temp_drop).max(ambient_temp);

        // Increase humidity (water vapor)
        if matches!(
            droplet.agent_type,
            SuppressionAgent::Water | SuppressionAgent::Foam
        ) {
            let vapor_added = droplet.mass * 0.1; // Some evaporates immediately
            cell.water_vapor += vapor_added / cell_volume;

            // Humidity increases
            let max_vapor = 0.04; // Max ~40 g/m³
            let vapor_fraction = (cell.water_vapor / max_vapor).min(1.0);
            cell.humidity = cell.humidity.max(vapor_fraction);
        }
    }
}

/// Calculate suppression effectiveness on fuel element
pub(crate) fn calculate_suppression_effectiveness(
    agent_concentration: f32,
    agent_type: SuppressionAgent,
) -> f32 {
    let base_effectiveness = match agent_type {
        SuppressionAgent::Water => 0.7,
        SuppressionAgent::ShortTermRetardant => 0.85,
        SuppressionAgent::LongTermRetardant => 0.95,
        SuppressionAgent::Foam => 0.9,
    };

    // Effectiveness increases with concentration (saturation curve)
    let concentration_factor = 1.0 - (-agent_concentration * 2.0).exp();

    base_effectiveness * concentration_factor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::TerrainData;
    use approx::assert_relative_eq;

    #[test]
    fn test_droplet_physics() {
        let mut droplet = SuppressionDroplet::new(
            Vec3::new(0.0, 0.0, 100.0),
            Vec3::new(10.0, 0.0, 0.0),
            0.5,
            SuppressionAgent::Water,
        );

        let wind = Vec3::new(5.0, 5.0, 0.0);

        // Update for 1 second
        for _ in 0..10 {
            droplet.update(wind, 20.0, 0.1);
        }

        // Should have moved and fallen
        assert!(droplet.position.z < 100.0);
        assert!(droplet.active);
    }

    #[test]
    fn test_evaporation() {
        let mut droplet = SuppressionDroplet::new(
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::zeros(),
            0.01, // Small droplet
            SuppressionAgent::Water,
        );

        let initial_mass = droplet.mass;

        // Hot environment
        droplet.update(Vec3::zeros(), 200.0, 1.0);

        // Should have evaporated some
        assert!(droplet.mass < initial_mass);
    }

    #[test]
    fn test_aircraft_drop() {
        let drop = AircraftDrop {
            position: Vec3::new(100.0, 100.0, 100.0),
            velocity: Vec3::new(50.0, 0.0, 0.0),
            altitude: 100.0,
            volume: 1000.0,
            agent_type: SuppressionAgent::Water,
            drop_rate: 500.0, // 500 kg/s
        };

        let droplets = drop.generate_droplets(0.1);

        // Should generate multiple droplets
        assert!(!droplets.is_empty());

        // Total mass should match
        let total_mass: f32 = droplets.iter().map(|d| d.mass).sum();
        assert_relative_eq!(total_mass, 50.0, epsilon = 0.1); // 500 kg/s × 0.1 s
    }

    #[test]
    fn test_ground_suppression() {
        let suppression = GroundSuppression {
            position: Vec3::new(50.0, 50.0, 1.0),
            direction: Vec3::new(1.0, 0.0, 0.2).normalize(),
            flow_rate: 50.0,
            agent_type: SuppressionAgent::Water,
            range: 30.0,
            active: true,
        };

        let droplets = suppression.generate_droplets(1.0);

        assert!(!droplets.is_empty());

        // Check spray pattern
        for droplet in &droplets {
            assert!(droplet.velocity.magnitude() > 10.0);
        }
    }

    #[test]
    fn test_grid_application() {
        let terrain = TerrainData::flat(100.0, 100.0, 5.0, 0.0);
        let mut grid = SimulationGrid::new(100.0, 100.0, 50.0, 10.0, terrain);

        let droplet = SuppressionDroplet::new(
            Vec3::new(50.0, 50.0, 10.0),
            Vec3::zeros(),
            10.0, // 10 kg
            SuppressionAgent::Water,
        );

        let initial_temp = grid.cell_at_position(droplet.position).unwrap().temperature;

        apply_suppression_to_grid(&droplet, &mut grid);

        // Should cool the cell
        let final_temp = grid.cell_at_position(droplet.position).unwrap().temperature;
        assert!(final_temp <= initial_temp);

        // Suppression agent should be present
        let agent = grid
            .cell_at_position(droplet.position)
            .unwrap()
            .suppression_agent;
        assert!(agent > 0.0);
    }

    #[test]
    fn test_cooling_capacity() {
        let water =
            SuppressionDroplet::new(Vec3::zeros(), Vec3::zeros(), 1.0, SuppressionAgent::Water);

        let foam =
            SuppressionDroplet::new(Vec3::zeros(), Vec3::zeros(), 1.0, SuppressionAgent::Foam);

        // Foam should have higher cooling capacity
        assert!(foam.cooling_capacity() > water.cooling_capacity());
    }

    #[test]
    fn test_suppression_effectiveness() {
        let eff_low = calculate_suppression_effectiveness(0.1, SuppressionAgent::Water);
        let eff_high = calculate_suppression_effectiveness(1.0, SuppressionAgent::Water);

        // More agent = more effective
        assert!(eff_high > eff_low);

        // Long-term retardant is most effective
        let eff_retardant =
            calculate_suppression_effectiveness(0.5, SuppressionAgent::LongTermRetardant);
        let eff_water = calculate_suppression_effectiveness(0.5, SuppressionAgent::Water);
        assert!(eff_retardant > eff_water);
    }
}
