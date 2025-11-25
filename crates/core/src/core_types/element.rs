use crate::core_types::fuel::Fuel;
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};

pub type Vec3 = Vector3<f32>;

/// Types of fuel parts in the simulation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FuelPart {
    // Vertical structures
    Root,
    TrunkLower,
    TrunkMiddle,
    TrunkUpper,
    BarkLayer(f32), // Height along trunk (meters)
    Branch { height: f32, angle: f32 },
    Crown, // Foliage canopy

    // Ground layer
    GroundLitter,     // Dead leaves, twigs
    GroundVegetation, // Grass, shrubs

    // Anthropogenic
    BuildingWall { floor: u8 },
    BuildingRoof,
    BuildingInterior,
    Vehicle,

    // Surface features
    Surface, // Roads, water, rock (non-burnable)
}

/// Individual fuel element in 3D space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelElement {
    pub(crate) id: u32,
    pub(crate) position: Vec3, // World position in meters
    pub(crate) fuel: Fuel,     // Comprehensive fuel type with all properties

    // Thermal state (accessible within crate only)
    pub(crate) temperature: f32,       // Current temperature (°C)
    pub(crate) moisture_fraction: f32, // 0-1, calculated from weather
    pub(crate) fuel_remaining: f32,    // kg
    pub(crate) ignited: bool,
    pub(crate) flame_height: f32, // meters (Byram's formula)

    // Structural relationships
    pub(crate) parent_id: Option<u32>, // Parent structure/tree ID
    pub(crate) part_type: FuelPart,    // What kind of fuel part

    // Spatial context
    pub(crate) elevation: f32,      // Height above ground
    pub(crate) slope_angle: f32,    // Local terrain slope (degrees)
    pub(crate) neighbors: Vec<u32>, // Cached nearby fuel IDs (within 15m)

    // Advanced physics state (Phase 1-3 enhancements)
    /// Fuel moisture state by timelag class (Nelson 2000)
    pub(crate) moisture_state: Option<crate::physics::FuelMoistureState>,
    /// Smoldering combustion phase (Rein 2009)
    pub(crate) smoldering_state: Option<crate::physics::SmolderingState>,
    /// Crown fire initiated flag
    pub(crate) crown_fire_active: bool,
}

impl FuelElement {
    /// Create a new fuel element
    pub(crate) fn new(
        id: u32,
        position: Vec3,
        fuel: Fuel,
        mass: f32,
        part_type: FuelPart,
        parent_id: Option<u32>,
    ) -> Self {
        let moisture_fraction = fuel.base_moisture;
        let elevation = position.z;

        // Initialize fuel moisture timelag state
        let moisture_state = Some(crate::physics::FuelMoistureState::new(
            moisture_fraction,
            moisture_fraction,
            moisture_fraction,
            moisture_fraction,
        ));

        // Initialize smoldering state
        let smoldering_state = Some(crate::physics::SmolderingState::new());

        FuelElement {
            id,
            position,
            temperature: 20.0, // Ambient temperature
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
            moisture_state,
            smoldering_state,
            crown_fire_active: false,
        }
    }

    /// This fuel element's id.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get world position
    pub fn position(&self) -> &Vec3 {
        &self.position
    }

    /// Get fuel type
    pub fn fuel(&self) -> &Fuel {
        &self.fuel
    }

    /// Get fuel part type
    pub fn part_type(&self) -> FuelPart {
        self.part_type
    }

    /// Get parent element ID (if any)
    pub fn parent_id(&self) -> Option<u32> {
        self.parent_id
    }

    /// Get elevation (height above ground)
    pub fn elevation(&self) -> f32 {
        self.elevation
    }

    /// Get local terrain slope angle in degrees
    pub fn slope_angle(&self) -> f32 {
        self.slope_angle
    }

    /// Get neighboring element IDs
    pub fn neighbors(&self) -> &[u32] {
        &self.neighbors
    }

    /// Get fuel moisture state (if present)
    pub fn moisture_state(&self) -> Option<&crate::physics::FuelMoistureState> {
        self.moisture_state.as_ref()
    }

    /// Set temperature (for testing)
    #[cfg(test)]
    pub(crate) fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Apply heat to this fuel element (CRITICAL: moisture evaporation first)
    pub(crate) fn apply_heat(&mut self, heat_kj: f32, dt: f32, ambient_temperature: f32) {
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
        let max_temp = self
            .fuel
            .calculate_max_flame_temperature(self.moisture_fraction);
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
        // OPTIMIZATION: Early exit for saturated fuel (can't ignite)
        if self.moisture_fraction >= self.fuel.moisture_of_extinction {
            return;
        }

        // OPTIMIZATION: Early exit for cold fuel (far from ignition temp)
        if self.temperature < self.fuel.ignition_temperature - 50.0 {
            return;
        }

        // Moisture reduces ignition probability
        let moisture_factor =
            (1.0 - self.moisture_fraction / self.fuel.moisture_of_extinction).max(0.0);

        // Temperature above ignition increases probability
        let temp_factor = ((self.temperature - self.fuel.ignition_temperature) / 50.0).min(1.0);

        let ignition_prob = moisture_factor * temp_factor * dt * 2.0;

        if rand::random::<f32>() < ignition_prob {
            self.ignited = true;
        }
    }

    /// Calculate burn rate in kg/s
    pub(crate) fn calculate_burn_rate(&self) -> f32 {
        // OPTIMIZATION: Early exits for non-burning conditions
        if !self.ignited {
            return 0.0;
        }

        if self.fuel_remaining <= 0.0 {
            return 0.0;
        }

        // OPTIMIZATION: Early exit for cold fuel (not hot enough to burn)
        if self.temperature < self.fuel.ignition_temperature {
            return 0.0;
        }

        // Realistic burn rate - slower burning for sustained fires
        let moisture_factor =
            (1.0 - self.moisture_fraction / self.fuel.moisture_of_extinction).max(0.0);
        let temp_factor =
            ((self.temperature - self.fuel.ignition_temperature) / 200.0).clamp(0.0, 1.0);

        // Reduced burn rate coefficient for longer-lasting fires (multiply by 0.1)
        self.fuel.burn_rate_coefficient
            * moisture_factor
            * temp_factor
            * self.fuel_remaining.sqrt()
            * 0.1
    }

    /// Calculate Byram's fireline intensity in kW/m
    ///
    /// Uses Rothermel spread rate model for accurate intensity calculation.
    ///
    /// # Formula
    /// ```text
    /// I = (H × w × r) / 60 [kW/m]
    /// ```
    ///
    /// Where:
    /// - **I** = Fireline intensity (kW/m)
    /// - **H** = Heat content of fuel (kJ/kg)
    /// - **w** = Fuel consumed per unit area (kg/m²)
    /// - **r** = Rate of spread (m/min)
    ///
    /// # References
    /// - Byram, G.M. (1959). "Combustion of forest fuels." In Forest Fire: Control and Use.
    /// - Rothermel, R.C. (1972). "A mathematical model for predicting fire spread in wildland fuels."
    pub(crate) fn byram_fireline_intensity(&self, wind_speed_ms: f32) -> f32 {
        // OPTIMIZATION: Early exits for non-burning conditions
        if !self.ignited {
            return 0.0;
        }

        if self.fuel_remaining <= 0.0 {
            return 0.0;
        }

        // OPTIMIZATION: Early exit for cold fuel
        if self.temperature < self.fuel.ignition_temperature {
            return 0.0;
        }

        // Calculate spread rate using Rothermel model
        let spread_rate_m_per_min = crate::physics::rothermel::rothermel_spread_rate(
            &self.fuel,
            self.moisture_fraction,
            wind_speed_ms,
            self.slope_angle,
        );

        // Fuel loading (kg/m²) - mass per unit area
        let fuel_loading = self.fuel.bulk_density * self.fuel.fuel_bed_depth;

        // Heat release with combustion efficiency (90%)
        let heat_per_area = self.fuel.heat_content * fuel_loading * 0.9;

        // Byram's formula: I = (H × w × r) / 60
        // Units: (kJ/kg × kg/m² × m/min) / 60 = kW/m
        (heat_per_area * spread_rate_m_per_min) / 60.0
    }

    // Public accessor methods for visualization/external use

    /// Get current temperature in Celsius
    pub fn temperature(&self) -> f32 {
        self.temperature
    }

    /// Get current moisture fraction (0-1)
    pub fn moisture_fraction(&self) -> f32 {
        self.moisture_fraction
    }

    /// Get remaining fuel mass in kg
    pub fn fuel_remaining(&self) -> f32 {
        self.fuel_remaining
    }

    /// Check if element is currently ignited
    pub fn is_ignited(&self) -> bool {
        self.ignited
    }

    /// Get current flame height in meters
    pub fn flame_height(&self) -> f32 {
        self.flame_height
    }

    /// Get smoldering state (if present)
    pub fn smoldering_state(&self) -> Option<&crate::physics::SmolderingState> {
        self.smoldering_state.as_ref()
    }

    /// Check if crown fire is currently active
    pub fn is_crown_fire_active(&self) -> bool {
        self.crown_fire_active
    }

    /// Get comprehensive statistics about this fuel element
    pub fn get_stats(&self) -> FuelElementStats {
        FuelElementStats {
            id: self.id,
            position: self.position,
            temperature: self.temperature,
            moisture_fraction: self.moisture_fraction,
            fuel_remaining: self.fuel_remaining,
            ignited: self.ignited,
            flame_height: self.flame_height,
            part_type: self.part_type,
            elevation: self.elevation,
            slope_angle: self.slope_angle,
            crown_fire_active: self.crown_fire_active,
            fuel_type_name: self.fuel.name.clone(),
            ignition_temperature: self.fuel.ignition_temperature,
            heat_content: self.fuel.heat_content,
        }
    }
}

/// Statistics snapshot of a fuel element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelElementStats {
    pub id: u32,
    pub position: Vector3<f32>,
    pub temperature: f32,
    pub moisture_fraction: f32,
    pub fuel_remaining: f32,
    pub ignited: bool,
    pub flame_height: f32,
    pub part_type: FuelPart,
    pub elevation: f32,
    pub slope_angle: f32,
    pub crown_fire_active: bool,
    pub fuel_type_name: String,
    pub ignition_temperature: f32,
    pub heat_content: f32,
}
