use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use crate::fuel::Fuel;

pub type Vec3 = Vector3<f32>;

/// Types of fuel parts in the simulation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FuelPart {
    // Vertical structures
    Root,
    TrunkLower,
    TrunkMiddle,
    TrunkUpper,
    BarkLayer(f32),              // Height along trunk (meters)
    Branch { height: f32, angle: f32 },
    Crown,                       // Foliage canopy
    
    // Ground layer
    GroundLitter,                // Dead leaves, twigs
    GroundVegetation,            // Grass, shrubs
    
    // Anthropogenic
    BuildingWall { floor: u8 },
    BuildingRoof,
    BuildingInterior,
    Vehicle,
    
    // Surface features
    Surface,                     // Roads, water, rock (non-burnable)
}

/// Individual fuel element in 3D space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelElement {
    pub id: u32,
    pub position: Vec3,              // World position in meters
    pub fuel: Fuel,                  // Comprehensive fuel type with all properties
    
    // Thermal state
    pub temperature: f32,            // Current temperature (°C)
    pub moisture_fraction: f32,      // 0-1, calculated from weather
    pub fuel_remaining: f32,         // kg
    pub ignited: bool,
    pub flame_height: f32,          // meters (Byram's formula)
    
    // Structural relationships
    pub parent_id: Option<u32>,      // Parent structure/tree ID
    pub part_type: FuelPart,         // What kind of fuel part
    
    // Spatial context
    pub elevation: f32,              // Height above ground
    pub slope_angle: f32,            // Local terrain slope (degrees)
    pub neighbors: Vec<u32>,         // Cached nearby fuel IDs (within 15m)
}

impl FuelElement {
    /// Create a new fuel element
    pub fn new(
        id: u32,
        position: Vec3,
        fuel: Fuel,
        mass: f32,
        part_type: FuelPart,
        parent_id: Option<u32>,
    ) -> Self {
        let moisture_fraction = fuel.base_moisture;
        let elevation = position.z;
        
        FuelElement {
            id,
            position,
            temperature: 20.0,  // Ambient temperature
            moisture_fraction,
            fuel_remaining: mass,
            ignited: false,
            flame_height: 0.0,
            parent_id,
            part_type,
            elevation,
            slope_angle: 0.0,
            neighbors: Vec::new(),
            fuel,
        }
    }
    
    /// Apply heat to this fuel element (CRITICAL: moisture evaporation first)
    pub fn apply_heat(&mut self, heat_kj: f32, dt: f32, ambient_temperature: f32) {
        if heat_kj <= 0.0 || self.fuel_remaining <= 0.0 {
            return;
        }
        
        // STEP 1: Evaporate moisture (2260 kJ/kg latent heat of vaporization)
        let moisture_mass = self.fuel_remaining * self.moisture_fraction;
        if moisture_mass > 0.0 {
            let evaporation_energy = moisture_mass * 2260.0;
            let heat_for_evaporation = heat_kj.min(evaporation_energy);
            let moisture_evaporated = heat_for_evaporation / 2260.0;
            
            let new_moisture_mass = (moisture_mass - moisture_evaporated).max(0.0);
            self.moisture_fraction = if self.fuel_remaining > 0.0 {
                new_moisture_mass / self.fuel_remaining
            } else {
                0.0
            };
            
            // STEP 2: Remaining heat raises temperature
            let remaining_heat = heat_kj - heat_for_evaporation;
            if remaining_heat > 0.0 && self.fuel_remaining > 0.0 {
                let temp_rise = remaining_heat / (self.fuel_remaining * self.fuel.specific_heat);
                self.temperature += temp_rise;
            }
        } else {
            // No moisture, all heat goes to temperature rise
            let temp_rise = heat_kj / (self.fuel_remaining * self.fuel.specific_heat);
            self.temperature += temp_rise;
        }
        
        // STEP 3: Cap at fuel-specific maximum (prevents thermal runaway)
        let max_temp = self.fuel.calculate_max_flame_temperature(self.moisture_fraction);
        self.temperature = self.temperature.min(max_temp);
        
        // STEP 4: Clamp to ambient minimum (prevents negative heat)
        self.temperature = self.temperature.max(ambient_temperature);
        
        // STEP 5: Check for ignition
        if !self.ignited && self.temperature >= self.fuel.ignition_temperature {
            self.check_ignition_probability(dt);
        }
    }
    
    /// Check if element should ignite (probabilistic)
    fn check_ignition_probability(&mut self, dt: f32) {
        // Moisture reduces ignition probability
        let moisture_factor = (1.0 - self.moisture_fraction / self.fuel.moisture_of_extinction).max(0.0);
        
        // Temperature above ignition increases probability
        let temp_factor = ((self.temperature - self.fuel.ignition_temperature) / 50.0).min(1.0);
        
        let ignition_prob = moisture_factor * temp_factor * dt * 2.0;
        
        if rand::random::<f32>() < ignition_prob {
            self.ignited = true;
        }
    }
    
    /// Manually ignite this element
    pub fn ignite(&mut self, initial_temp: f32) {
        self.ignited = true;
        self.temperature = initial_temp.max(self.fuel.ignition_temperature);
    }
    
    /// Calculate burn rate in kg/s
    pub fn calculate_burn_rate(&self) -> f32 {
        if !self.ignited || self.fuel_remaining <= 0.0 {
            return 0.0;
        }
        
        // Burn rate based on fuel coefficient and moisture
        let moisture_factor = (1.0 - self.moisture_fraction / self.fuel.moisture_of_extinction).max(0.0);
        let temp_factor = ((self.temperature - self.fuel.ignition_temperature) / 200.0).clamp(0.0, 1.0);
        
        self.fuel.burn_rate_coefficient * moisture_factor * temp_factor * self.fuel_remaining.sqrt()
    }
    
    /// Calculate Byram's fireline intensity in kW/m
    pub fn byram_fireline_intensity(&self) -> f32 {
        if !self.ignited || self.fuel_remaining <= 0.0 {
            return 0.0;
        }
        
        // I = (H × w × r) / 60 [kW/m]
        let burn_rate = self.calculate_burn_rate();
        let heat_release = self.fuel.heat_content * burn_rate * 0.9; // 90% efficiency
        
        // Simplified spread rate estimate
        let spread_rate_m_per_min = 0.5 * 60.0; // Placeholder
        
        (heat_release * spread_rate_m_per_min) / 60.0
    }
    
    /// Calculate flame height using Byram's formula
    pub fn calculate_flame_height(&self) -> f32 {
        let intensity = self.byram_fireline_intensity();
        
        // L = 0.0775 × I^0.46 [meters]
        if intensity > 0.0 {
            0.0775 * intensity.powf(0.46)
        } else {
            0.0
        }
    }
    
    /// Update flame height
    pub fn update_flame_height(&mut self) {
        self.flame_height = self.calculate_flame_height();
    }
    
    /// Burn fuel mass
    pub fn burn_fuel(&mut self, dt: f32) {
        if !self.ignited {
            return;
        }
        
        let burn_rate = self.calculate_burn_rate();
        self.fuel_remaining = (self.fuel_remaining - burn_rate * dt).max(0.0);
        
        // Extinguish if fuel depleted
        if self.fuel_remaining < 0.01 {
            self.ignited = false;
            self.temperature = 20.0; // Cool down to ambient
            self.flame_height = 0.0;
        }
    }
    
    /// Check if this element can ignite (not already burning, has fuel, etc.)
    pub fn can_ignite(&self) -> bool {
        !self.ignited && self.fuel_remaining > 0.01 && 
        self.moisture_fraction < self.fuel.moisture_of_extinction
    }
    
    /// Get heat radiation output in kW
    pub fn get_radiation_power(&self) -> f32 {
        if !self.ignited {
            return 0.0;
        }
        
        // Simplified radiation based on temperature and surface area
        let temp_k = self.temperature + 273.15;
        let surface_area = self.fuel.surface_area_to_volume * self.fuel_remaining.sqrt();
        
        // Stefan-Boltzmann simplified
        5.67e-8 * (temp_k / 1000.0).powi(4) * surface_area * 10000.0
    }
}
