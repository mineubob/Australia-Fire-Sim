/// Pyrocumulonimbus (PyroCb) Storm System
/// 
/// Models fire-generated thunderstorms that can create their own weather patterns,
/// including lightning strikes that can start new fires.
/// 
/// Based on research:
/// - Nature Communications: "Understanding the critical elements of the pyrocumulonimbus storm"
/// - Australian Bureau of Meteorology: "When bushfires make their own weather"
/// - AGU Journal of Geophysical Research: "Pyrocumulonimbus lightning and fire ignition"

use crate::core_types::element::Vec3;
use rand::Rng;

/// Pyrocumulonimbus cloud state
#[derive(Debug, Clone)]
pub struct PyroCb {
    pub position: Vec3,           // Cloud base position
    pub height: f32,              // Cloud top height (meters above base)
    pub energy: f32,              // Total energy driving the convection (kJ)
    pub updraft_velocity: f32,    // Vertical wind speed (m/s)
    pub charge_separation: f32,   // Electrical charge buildup (0-1)
    pub lightning_potential: f32, // Probability of lightning strike per second
    pub age: f32,                 // Time since formation (seconds)
    pub diameter: f32,            // Cloud horizontal extent (meters)
    pub active: bool,
}

/// Lightning strike event
#[derive(Debug, Clone)]
pub struct LightningStrike {
    pub position: Vec3,
    pub energy: f32,              // kJ
    pub ignition_radius: f32,     // meters
    pub temperature: f32,         // °C at strike point
}

/// Downdraft/outflow event
#[derive(Debug, Clone)]
pub struct Downdraft {
    pub position: Vec3,
    pub wind_speed: f32,          // m/s
    pub radius: f32,              // meters affected
    pub duration: f32,            // seconds
}

impl PyroCb {
    /// Check if conditions support pyroCb formation
    /// Requires:
    /// - Fire intensity > 50,000 kW/m (extreme fire behavior)
    /// - Updraft energy sufficient to lift air to condensation level
    /// - Atmospheric instability (low humidity, high temperature gradient)
    pub fn can_form(
        fire_intensity: f32,
        ambient_temp: f32,
        humidity: f32,
        wind_speed: f32,
    ) -> bool {
        // Threshold from research: ~50,000 kW/m for pyroCb formation
        let intensity_threshold = 50000.0;
        
        // Atmospheric conditions must be unstable
        let atmospheric_instability = (100.0 - humidity) / 100.0 * (ambient_temp / 40.0);
        
        // Strong winds can disrupt column formation
        let wind_disruption = (wind_speed / 60.0).min(1.0);
        
        fire_intensity > intensity_threshold 
            && atmospheric_instability > 0.4
            && wind_disruption < 0.7  // Too much wind prevents vertical development
    }
    
    /// Create a new pyroCb cloud
    pub fn new(position: Vec3, fire_intensity: f32, _ambient_temp: f32) -> Self {
        // Energy from fire drives the convection
        let energy = fire_intensity * 100.0; // Simplified conversion
        
        // Initial updraft velocity based on fire intensity
        // Research shows updrafts can exceed 20 m/s in extreme pyroCbs
        let updraft_velocity = ((fire_intensity / 10000.0).sqrt() * 5.0).min(25.0);
        
        // Cloud height correlates with energy (can reach 10-15 km)
        let height = (energy / 10000.0).sqrt() * 1000.0;
        let height = height.clamp(3000.0, 15000.0);
        
        // Initial diameter based on fire area
        let diameter = (fire_intensity / 100.0).sqrt().clamp(500.0, 5000.0);
        
        Self {
            position,
            height,
            energy,
            updraft_velocity,
            charge_separation: 0.0,
            lightning_potential: 0.0,
            age: 0.0,
            diameter,
            active: true,
        }
    }
    
    /// Update pyroCb cloud state
    pub fn update(&mut self, dt: f32, fire_intensity: f32, _ambient_temp: f32, _humidity: f32) {
        if !self.active {
            return;
        }
        
        self.age += dt;
        
        // Energy input from ongoing fire
        let energy_input = fire_intensity * dt * 10.0;
        self.energy += energy_input;
        
        // Energy dissipation over time (radiation, precipitation)
        let dissipation_rate = self.energy * 0.01 * dt;
        self.energy -= dissipation_rate;
        
        // Updraft velocity responds to energy changes
        self.updraft_velocity = ((self.energy / 10000.0).sqrt() * 5.0).min(25.0);
        
        // Cloud grows with sustained energy
        if self.updraft_velocity > 10.0 {
            self.height += self.updraft_velocity * dt * 0.5;
            self.height = self.height.min(15000.0);
            
            self.diameter += dt * 20.0; // Expands outward
            self.diameter = self.diameter.min(10000.0);
        }
        
        // Charge separation increases with updraft intensity
        // Ice particle collisions in the cloud create electrical charge
        if self.height > 5000.0 && self.updraft_velocity > 8.0 {
            let charge_rate = (self.updraft_velocity - 5.0) * 0.02 * dt;
            self.charge_separation += charge_rate;
            self.charge_separation = self.charge_separation.min(1.0);
        }
        
        // Lightning potential increases with charge separation
        self.lightning_potential = self.charge_separation.powf(2.0) * 0.1;
        
        // Cloud dissipates if fire weakens or after extended time
        if fire_intensity < 10000.0 || self.age > 3600.0 {
            self.active = false;
        }
        
        // Energy threshold for maintaining activity
        if self.energy < 1000.0 {
            self.active = false;
        }
    }
    
    /// Generate lightning strike
    pub fn generate_lightning(&mut self, rng: &mut impl Rng) -> Option<LightningStrike> {
        if !self.active || self.charge_separation < 0.3 {
            return None;
        }
        
        // Probability check
        if rng.gen::<f32>() > self.lightning_potential {
            return None;
        }
        
        // Discharge reduces charge separation
        self.charge_separation *= 0.6;
        
        // Strike location: within cloud diameter, random offset from center
        let angle = rng.gen::<f32>() * 2.0 * std::f32::consts::PI;
        let distance = rng.gen::<f32>() * self.diameter * 0.5;
        
        let strike_pos = Vec3::new(
            self.position.x + distance * angle.cos(),
            self.position.y + distance * angle.sin(),
            0.0, // Ground level
        );
        
        // Lightning energy (typical: 1-5 billion joules = 250,000 - 1,250,000 kJ)
        // But only small fraction goes to heating ground fuel
        let energy = rng.gen_range(50.0..200.0); // kJ effective for ignition
        
        // Ignition radius (lightning can ignite within ~1-5 meters)
        let ignition_radius = rng.gen_range(1.0..5.0);
        
        // Temperature at strike point (lightning channel: 30,000 K, but brief)
        // Effective temperature for fuel ignition
        let temperature = rng.gen_range(1000.0..2000.0);
        
        Some(LightningStrike {
            position: strike_pos,
            energy,
            ignition_radius,
            temperature,
        })
    }
    
    /// Generate downdraft/outflow wind
    /// PyroCbs can produce severe downdrafts when precipitation forms or cloud collapses
    pub fn generate_downdraft(&self, rng: &mut impl Rng) -> Option<Downdraft> {
        if !self.active {
            return None;
        }
        
        // Downdrafts more likely with tall, mature clouds
        if self.height < 8000.0 || self.age < 300.0 {
            return None;
        }
        
        // Probability increases with age and height
        let probability = ((self.height - 8000.0) / 7000.0) * (self.age / 1800.0).min(1.0);
        
        if rng.gen::<f32>() > probability * 0.01 {
            return None;
        }
        
        // Downdraft wind speed (can exceed 50 m/s in extreme cases)
        let wind_speed = rng.gen_range(15.0..40.0);
        
        // Affected area
        let radius = self.diameter * 0.7;
        
        // Duration (typically 5-15 minutes)
        let duration = rng.gen_range(300.0..900.0);
        
        Some(Downdraft {
            position: self.position,
            wind_speed,
            radius,
            duration,
        })
    }
}

/// Manager for all pyroCb clouds in simulation
pub struct PyroCbSystem {
    pub clouds: Vec<PyroCb>,
    pub lightning_strikes: Vec<LightningStrike>,
    pub downdrafts: Vec<Downdraft>,
    pub total_lightning_events: usize,
}

impl PyroCbSystem {
    pub fn new() -> Self {
        Self {
            clouds: Vec::new(),
            lightning_strikes: Vec::new(),
            downdrafts: Vec::new(),
            total_lightning_events: 0,
        }
    }
    
    /// Check for pyroCb formation at given position
    pub fn check_formation(
        &mut self,
        position: Vec3,
        fire_intensity: f32,
        ambient_temp: f32,
        humidity: f32,
        wind_speed: f32,
    ) {
        if PyroCb::can_form(fire_intensity, ambient_temp, humidity, wind_speed) {
            // Don't form if cloud already exists nearby
            let too_close = self.clouds.iter().any(|cloud| {
                cloud.active && (cloud.position - position).magnitude() < 5000.0
            });
            
            if !too_close {
                let cloud = PyroCb::new(position, fire_intensity, ambient_temp);
                println!("🌩️  PYROCUMULONIMBUS FORMATION at ({:.0}, {:.0})", 
                         position.x, position.y);
                println!("   Cloud height: {:.0}m, Updraft: {:.1}m/s, Diameter: {:.0}m",
                         cloud.height, cloud.updraft_velocity, cloud.diameter);
                self.clouds.push(cloud);
            }
        }
    }
    
    /// Update all clouds and generate weather events
    pub fn update(
        &mut self,
        dt: f32,
        fire_intensity: f32,
        ambient_temp: f32,
        humidity: f32,
        rng: &mut impl Rng,
    ) {
        // Clear old events
        self.lightning_strikes.clear();
        self.downdrafts.clear();
        
        // Update each cloud
        for cloud in &mut self.clouds {
            if !cloud.active {
                continue;
            }
            
            cloud.update(dt, fire_intensity, ambient_temp, humidity);
            
            // Generate lightning
            if let Some(strike) = cloud.generate_lightning(rng) {
                println!("⚡ LIGHTNING STRIKE at ({:.0}, {:.0}), energy: {:.0} kJ",
                         strike.position.x, strike.position.y, strike.energy);
                self.total_lightning_events += 1;
                self.lightning_strikes.push(strike);
            }
            
            // Generate downdrafts
            if let Some(downdraft) = cloud.generate_downdraft(rng) {
                println!("💨 DOWNDRAFT from pyroCb, wind speed: {:.0} m/s, radius: {:.0}m",
                         downdraft.wind_speed, downdraft.radius);
                self.downdrafts.push(downdraft);
            }
        }
        
        // Remove inactive clouds
        self.clouds.retain(|cloud| cloud.active);
    }
    
    /// Get number of active pyroCb clouds
    pub fn active_cloud_count(&self) -> usize {
        self.clouds.iter().filter(|c| c.active).count()
    }
}

impl Default for PyroCbSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pyrocb_formation_criteria() {
        // Should NOT form with low intensity
        assert!(!PyroCb::can_form(10000.0, 35.0, 30.0, 20.0));
        
        // Should form with extreme intensity and good conditions
        assert!(PyroCb::can_form(60000.0, 40.0, 15.0, 25.0));
        
        // Should NOT form with very high humidity
        assert!(!PyroCb::can_form(60000.0, 35.0, 80.0, 20.0));
        
        // Should NOT form with extremely high winds
        assert!(!PyroCb::can_form(60000.0, 40.0, 15.0, 70.0));
    }
    
    #[test]
    fn test_pyrocb_cloud_properties() {
        let cloud = PyroCb::new(Vec3::new(0.0, 0.0, 0.0), 60000.0, 40.0);
        
        // Cloud should have reasonable properties
        assert!(cloud.height >= 3000.0 && cloud.height <= 15000.0);
        assert!(cloud.updraft_velocity > 0.0 && cloud.updraft_velocity <= 25.0);
        assert!(cloud.diameter >= 500.0);
        assert!(cloud.active);
    }
    
    #[test]
    fn test_charge_buildup() {
        let mut cloud = PyroCb::new(Vec3::new(0.0, 0.0, 0.0), 60000.0, 40.0);
        cloud.height = 10000.0; // Tall enough for charge separation
        cloud.updraft_velocity = 15.0;
        
        // Simulate time
        for _ in 0..60 {
            cloud.update(1.0, 60000.0, 40.0, 20.0);
        }
        
        // Should have built up some charge
        assert!(cloud.charge_separation > 0.0);
        assert!(cloud.lightning_potential > 0.0);
    }
}
