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
    pub id: u32,
    pub position: Vec3, // World position in meters
    pub fuel: Fuel,     // Comprehensive fuel type with all properties

    // Thermal state (accessible within crate only)
    pub(crate) temperature: f32,       // Current temperature (°C)
    pub(crate) moisture_fraction: f32, // 0-1, calculated from weather
    pub(crate) fuel_remaining: f32,    // kg
    pub(crate) ignited: bool,
    pub(crate) flame_height: f32, // meters (Byram's formula)

    // Structural relationships
    pub parent_id: Option<u32>, // Parent structure/tree ID
    pub part_type: FuelPart,    // What kind of fuel part

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

    /// Set temperature (for testing)
    #[cfg(test)]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
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

    /// Manually ignite this element
    pub fn ignite(&mut self, initial_temp: f32) {
        self.ignited = true;
        self.temperature = initial_temp.max(self.fuel.ignition_temperature);
    }

    /// Calculate burn rate in kg/s
    pub fn calculate_burn_rate(&self) -> f32 {
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
    pub fn byram_fireline_intensity(&self, wind_speed_ms: f32) -> f32 {
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

    /// Calculate flame height using Byram's formula
    ///
    /// # Formula
    /// ```text
    /// L = 0.0775 × I^0.46 [meters]
    /// ```
    ///
    /// Where:
    /// - **L** = Flame height (meters)
    /// - **I** = Fireline intensity (kW/m)
    ///
    /// # References
    /// - Byram, G.M. (1959). "Combustion of forest fuels." In Forest Fire: Control and Use.
    /// - Equation empirically validated for Australian conditions
    pub fn calculate_flame_height(&self, wind_speed_ms: f32) -> f32 {
        let intensity = self.byram_fireline_intensity(wind_speed_ms);

        // L = 0.0775 × I^0.46 [meters]
        if intensity > 0.0 {
            0.0775 * intensity.powf(0.46)
        } else {
            0.0
        }
    }

    /// Update flame height
    pub fn update_flame_height(&mut self, wind_speed_ms: f32) {
        self.flame_height = self.calculate_flame_height(wind_speed_ms);
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
        !self.ignited
            && self.fuel_remaining > 0.01
            && self.moisture_fraction < self.fuel.moisture_of_extinction
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
}
