# Fire Simulation Core - Multi-Phase Implementation Plan

**Project**: Australia Fire Simulation - Core Physics Engine
**Target**: Copilot Coding Agent
**Scope**: Fire physics simulation enhancements (game engine responsibilities excluded)

═══════════════════════════════════════════════════════════════════════
## OVERVIEW
═══════════════════════════════════════════════════════════════════════

This plan details the implementation of advanced fire simulation features for the core physics engine. The following are **explicitly excluded** as they will be handled by the game engine:
- ❌ Firefighter character operations (movement, hose handling, injury visualization)
- ❌ Vehicle systems (truck control, equipment deployment UI)
- ❌ Suppression particle effects (water/foam visual rendering)
- ❌ Communications UI (radio interface, command structures)
- ❌ Terrain rendering and visualization

### What We ARE Implementing (Core Physics Only):
- ✅ Fire retardant **physics** (chemical effects on combustion)
- ✅ Suppression application **at specified positions** (for game engine to trigger)
- ✅ Suppression **state tracking** (coverage, evaporation, effectiveness per fuel element)
- ✅ Advanced weather physics (pyrocumulus, atmospheric instability)
- ✅ Terrain elevation/slope **data structures** (for fire spread calculations)
- ✅ Fire behavior state data (for game engine to query and render)
- ✅ **Query interfaces** for fire state (intensity, temperature, spread rate at any position)
- ✅ **Query interfaces** for suppression state (coverage %, agent type, remaining mass)

═══════════════════════════════════════════════════════════════════════
## PHASE 1: FIRE SUPPRESSION PHYSICS (CORE ONLY)
═══════════════════════════════════════════════════════════════════════

**Goal**: Implement the physics of fire suppression agents (water, foam, retardant) acting on fuel elements at specified world positions. Game engine will handle visual effects and user interaction.

### 1.1 - Suppression Agent Data Structures

**Location**: `crates/core/src/suppression/agent.rs` (new file)

```rust
/// Types of suppression agents with different physical properties
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SuppressionAgentType {
    Water,              // Pure water
    FoamClassA,         // Class A foam (wildland)
    FoamClassB,         // Class B foam (fuel fires)
    LongTermRetardant,  // Phos-Chek, etc.
    ShortTermRetardant, // Water-based gel
    WettingAgent,       // Surfactant-enhanced water
}

/// Physical properties of suppression agents
pub struct SuppressionAgent {
    pub agent_type: SuppressionAgentType,
    
    // Thermal properties
    pub specific_heat: f32,              // kJ/(kg·K)
    pub latent_heat_vaporization: f32,   // kJ/kg
    pub boiling_point: f32,              // °C
    
    // Coverage properties
    pub application_rate: f32,           // kg/m²
    pub coverage_efficiency: f32,        // 0-1 (foam > water)
    pub penetration_depth: f32,          // meters into fuel bed
    
    // Chemical properties
    pub combustion_inhibition: f32,      // 0-1 (retardant effect)
    pub oxygen_displacement: f32,        // 0-1 (foam blanketing)
    pub fuel_coating_duration: f32,      // seconds (long-term retardant)
    
    // Evaporation/degradation
    pub evaporation_rate_modifier: f32,  // relative to water
    pub uv_degradation_rate: f32,        // per hour (foam/retardant)
    pub rain_washoff_rate: f32,          // per mm rainfall
}
```

**Implementation Requirements**:
- Research-based values for each agent type (NFPA, USFS publications)
- Document sources for all constants
- Unit tests validating physical property ranges

### 1.2 - Suppression Coverage System

**Location**: `crates/core/src/suppression/coverage.rs` (new file)

```rust
/// Represents suppression agent coverage on a fuel element
pub struct SuppressionCoverage {
    pub agent: SuppressionAgent,
    pub mass_per_area: f32,              // kg/m²
    pub application_time: f32,           // simulation time
    pub coverage_fraction: f32,          // 0-1 (% of fuel surface covered)
    pub penetration_achieved: f32,       // meters into fuel bed
    pub active: bool,                    // Still effective?
}

impl SuppressionCoverage {
    /// Apply suppression physics to fuel element heating
    pub fn modify_heat_transfer(
        &self,
        incoming_heat_kj: f32,
        fuel_element: &FuelElement,
        dt: f32
    ) -> f32 {
        // 1. Heat absorbed by evaporating suppression agent
        let agent_mass = self.mass_per_area * fuel_element.surface_area();
        let evaporation_energy = agent_mass * self.agent.latent_heat_vaporization;
        
        // 2. Reduce incoming heat by agent evaporation
        let heat_after_evaporation = (incoming_heat_kj - evaporation_energy).max(0.0);
        
        // 3. Chemical combustion inhibition (retardants)
        let inhibition_factor = 1.0 - (self.agent.combustion_inhibition * self.coverage_fraction);
        
        // 4. Oxygen displacement (foam blanketing)
        let oxygen_factor = 1.0 - (self.agent.oxygen_displacement * self.coverage_fraction);
        
        // 5. Combined suppression effectiveness
        heat_after_evaporation * inhibition_factor * oxygen_factor
    }
    
    /// Update coverage state (evaporation, degradation)
    pub fn update(&mut self, weather: &WeatherState, dt: f32) {
        // Evaporate based on temperature and humidity
        let evaporation_rate = self.calculate_evaporation_rate(weather);
        self.mass_per_area -= evaporation_rate * dt;
        
        // UV degradation (foam/retardant)
        if weather.solar_radiation > 500.0 {
            let degradation = self.agent.uv_degradation_rate * dt / 3600.0;
            self.coverage_fraction -= degradation;
        }
        
        // Deactivate if depleted
        if self.mass_per_area < 0.1 || self.coverage_fraction < 0.05 {
            self.active = false;
        }
    }
    
    fn calculate_evaporation_rate(&self, weather: &WeatherState) -> f32 {
        // Penman-Monteith equation for evaporation
        let vapor_pressure_deficit = calculate_vpd(weather.temperature, weather.humidity);
        let base_rate = 0.0005 * vapor_pressure_deficit; // kg/(m²·s)
        base_rate * self.agent.evaporation_rate_modifier
    }
}
```

**Implementation Requirements**:
- Penman-Monteith evaporation model (peer-reviewed)
- UV degradation based on solar radiation intensity
- Validation tests against suppression effectiveness research

### 1.3 - FuelElement Suppression Integration

**Location**: `crates/core/src/core_types/element.rs` (modify existing)

```rust
pub struct FuelElement {
    // ... existing fields ...
    
    /// Active suppression coverage (if any)
    pub suppression: Option<SuppressionCoverage>,
}

impl FuelElement {
    /// Apply heat with suppression effects considered
    pub fn apply_heat_with_suppression(
        &mut self, 
        heat_kj: f32, 
        weather: &WeatherState,
        dt: f32
    ) {
        let effective_heat = if let Some(ref mut suppression) = self.suppression {
            // Update suppression state
            suppression.update(weather, dt);
            
            // Modify incoming heat
            if suppression.active {
                suppression.modify_heat_transfer(heat_kj, self, dt)
            } else {
                // Suppression depleted
                self.suppression = None;
                heat_kj
            }
        } else {
            heat_kj
        };
        
        // Apply modified heat (existing evaporation logic)
        self.apply_heat(effective_heat, dt);
    }
}
```

### 1.4 - FFI Suppression Application Interface

**Location**: `crates/ffi/src/lib.rs` (add functions)

```rust
/// Apply suppression agent at specified world position
/// Game engine calls this when player triggers suppression
/// 
/// # Returns
/// Number of fuel elements affected by this suppression application
#[no_mangle]
pub extern "C" fn fire_sim_apply_suppression(
    sim_id: usize,
    x: f32, y: f32, z: f32,           // World position
    radius: f32,                       // Coverage radius (meters)
    agent_type: u8,                    // SuppressionAgentType as u8
    amount_kg: f32,                    // Total agent mass applied
    coverage_quality: f32              // 0-1 (accuracy/effectiveness)
) -> u32 {
    // Returns number of fuel elements affected
}

/// Query suppression coverage at position (for game engine effects)
/// 
/// # Returns
/// true if active suppression exists at position, false otherwise
#[no_mangle]
pub extern "C" fn fire_sim_query_suppression(
    sim_id: usize,
    x: f32, y: f32, z: f32,
    out_agent_type: *mut u8,
    out_coverage_fraction: *mut f32,
    out_mass_remaining: *mut f32
) -> bool {
    // Returns true if active suppression exists at position
}

/// Query fire intensity at position (for heat exposure calculations)
/// 
/// Game engine uses this to:
/// - Determine firefighter heat exposure
/// - Calculate equipment damage
/// - Trigger audio/visual effects
/// 
/// # Returns
/// Fire intensity in kW/m at the specified position (0 if no fire)
#[no_mangle]
pub extern "C" fn fire_sim_query_fire_intensity(
    sim_id: usize,
    x: f32, y: f32, z: f32,
    radius: f32                        // Query radius (meters)
) -> f32 {
    // Returns average Byram intensity within radius
}

/// Query radiant heat flux at position (for injury calculations)
/// 
/// Game engine uses this to calculate:
/// - Firefighter heat stress
/// - Safe approach distances
/// - Equipment exposure damage
/// 
/// # Returns
/// Radiant heat flux in kW/m² at the specified position
#[no_mangle]
pub extern "C" fn fire_sim_query_radiant_heat(
    sim_id: usize,
    x: f32, y: f32, z: f32
) -> f32 {
    // Returns heat flux from all nearby burning elements
}

/// Query flame height at position (for visibility/obstruction)
/// 
/// # Returns
/// Maximum flame height in meters within query radius
#[no_mangle]
pub extern "C" fn fire_sim_query_flame_height(
    sim_id: usize,
    x: f32, y: f32, z: f32,
    radius: f32
) -> f32 {
    // Returns max flame height for rendering/collision
}

/// Check if position is within active fire perimeter
/// 
/// Game engine uses this for:
/// - Player warning systems
/// - AI pathfinding avoidance
/// - Mission objective checks
/// 
/// # Returns
/// true if position is within burning area
#[no_mangle]
pub extern "C" fn fire_sim_is_position_in_fire(
    sim_id: usize,
    x: f32, y: f32, z: f32,
    safety_margin: f32                 // Additional margin in meters
) -> bool {
    // Returns true if fire within safety_margin distance
}
```

**Key Design Points**:
- Game engine handles visual effects (particle systems, decals)
- Core simulation handles physics (heat reduction, evaporation)
- FFI provides position-based application interface
- Core tracks suppression state per fuel element
- **Query functions allow game engine to make gameplay decisions without duplicating physics**

### 1.5 - Validation & Testing

**Location**: `crates/core/tests/suppression_physics.rs` (new file)

Required tests:
- Water evaporation rates match empirical data
- Foam oxygen displacement effectiveness (60-80% reduction)
- Long-term retardant duration (4-8 hours typical)
- Class A foam vs water comparison (3-5x more effective)
- Heat absorption by agent evaporation
- Suppression depletion over time

**Scientific References**:
- NFPA 1150: Standard on Foam Chemicals for Fires in Class A Fuels
- USFS MTDC: Long-Term Fire Retardant Effectiveness Studies
- Penman-Monteith equation (FAO Irrigation and Drainage Paper 56)

### 1.6 - Ember Physics & Automatic Spot Fire Ignition

**Location**: `crates/core/src/core_types/ember.rs` (modify existing)

```rust
impl Ember {
    /// Check if ember can ignite fuel at landing position
    /// Called automatically during ember physics update
    pub fn attempt_ignition(&self, simulation: &mut FireSimulation) -> bool {
        // Only attempt if landed
        if self.position.z > 1.0 || self.temperature < 250.0 {
            return false;
        }
        
        // Find fuel element at landing position
        let nearby_fuel = simulation.spatial_index.query_radius(self.position, 2.0);
        let target_fuel = nearby_fuel
            .iter()
            .filter_map(|&id| simulation.get_fuel_element(id))
            .min_by_key(|f| (f.position - self.position).length() as u32);
        
        if let Some(fuel) = target_fuel {
            // Check suppression coverage (blocks ignition)
            if let Some(ref suppression) = fuel.suppression {
                if suppression.coverage_fraction > 0.5 {
                    return false; // Too much suppression
                }
            }
            
            // Calculate ignition probability (PHYSICS)
            let temp_factor = (self.temperature - 250.0) / 150.0; // 250-400°C range
            let moisture_factor = 1.0 - fuel.moisture_fraction;
            let receptivity = fuel.fuel.ember_receptivity;
            
            let suppression_penalty = if let Some(ref s) = fuel.suppression {
                1.0 - (s.coverage_fraction * 0.7)
            } else {
                1.0
            };
            
            let ignition_prob = temp_factor * moisture_factor * receptivity * suppression_penalty;
            
            // Probabilistic ignition
            if rand::random::<f32>() < ignition_prob {
                simulation.ignite_element(fuel.id);
                return true;
            }
        }
        
        false
    }
}
```

**Location**: `crates/core/src/simulation/mod.rs` (modify ember update)

```rust
impl FireSimulation {
    pub fn update_embers(&mut self, dt: f32) {
        let wind = self.weather.wind_vector();
        let ambient_temp = self.weather.temperature;
        
        // Update all ember physics in parallel
        self.embers.par_iter_mut().for_each(|ember| {
            ember.update_physics(wind, ambient_temp, dt);
        });
        
        // Check for ignitions (must be sequential for safety)
        let mut ignited_ember_ids = Vec::new();
        for ember in &self.embers {
            if ember.attempt_ignition(self) {
                ignited_ember_ids.push(ember.id);
            }
        }
        
        // Remove embers that ignited or cooled
        self.embers.retain(|e| {
            !ignited_ember_ids.contains(&e.id) && 
            e.temperature > 200.0 && 
            e.position.z > 0.0
        });
    }
}
```

**Location**: `crates/ffi/src/lib.rs` (add functions)

```rust
#[repr(C)]
pub struct EmberData {
    pub id: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub velocity_z: f32,
    pub temperature: f32,
    pub mass: f32,
    pub fuel_type: u8,
}

/// Get all active embers for rendering/Niagara
/// 
/// Game engine uses this ONLY for visualization:
/// - Render ember particles with correct position/velocity
/// - Display ember trails and glow effects
/// - Play ember sound effects
/// 
/// Ignition is handled automatically by simulation based on physics.
/// 
/// # Safety
/// out_count must be valid non-null pointer. Return pointer is valid only
/// until the next fire_sim_update call.
#[no_mangle]
pub extern "C" fn fire_sim_get_embers(
    sim_id: usize,
    out_count: *mut u32,
) -> *const EmberData {
    // Returns snapshot of all active embers
}

#[repr(C)]
pub struct SpotFireEvent {
    pub ember_id: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub fuel_element_id: u32,
    pub timestamp: f32,
}

/// Get spot fires that were created this frame (for visual/audio effects)
/// 
/// Game engine uses this to:
/// - Spawn spot fire explosion particle effects
/// - Play ignition sound effects
/// - Show UI notifications ("Spot fire detected!")
/// 
/// # Returns
/// Pointer to array of spot fire events from last update
#[no_mangle]
pub extern "C" fn fire_sim_get_spot_fire_events(
    sim_id: usize,
    out_count: *mut u32,
) -> *const SpotFireEvent {
    // Returns events for game engine to visualize
}
```

**Key Design Points**:
- **Simulation handles ALL ember behavior** (physics + ignition)
- Ignition is a **physics consequence** (temperature, moisture, receptivity)
- Suppression coverage **blocks ignition** (physics interaction)
- Game engine **only renders** embers (Niagara particles, trails, glow)
- Game engine **responds to events** (spot fire VFX, audio, UI notifications)
- **No game logic in ignition decision** - purely physical conditions

═══════════════════════════════════════════════════════════════════════
## PHASE 2: ADVANCED WEATHER PHENOMENA
═══════════════════════════════════════════════════════════════════════

**Goal**: Implement pyrocumulus cloud formation, atmospheric instability, and fire-weather feedback loops.

### 2.1 - Atmospheric Instability Modeling

**Location**: `crates/core/src/weather/atmosphere.rs` (new file)

```rust
/// Atmospheric stability state
pub struct AtmosphericProfile {
    // Vertical temperature profile (up to 5000m)
    pub temperature_layers: Vec<(f32, f32)>,  // (altitude_m, temp_C)
    
    // Stability indices
    pub lifted_index: f32,              // °C (negative = unstable)
    pub k_index: f32,                   // Thunderstorm potential
    pub haines_index: u8,               // 2-6 (fire weather severity)
    
    // Boundary layer
    pub mixing_height: f32,             // meters
    pub inversion_strength: f32,        // °C
    pub inversion_altitude: f32,        // meters
    
    // Wind profile (vertical)
    pub wind_shear: Vec<(f32, Vec3)>,  // (altitude, wind_vector)
}

impl AtmosphericProfile {
    /// Calculate Haines Index (fire weather severity)
    /// Source: Haines, D.A. (1988) - USFS Research Paper
    pub fn calculate_haines_index(&self) -> u8 {
        // Low elevation variant (< 1000m MSL)
        let t_950 = self.temp_at_pressure(950.0); // hPa
        let t_850 = self.temp_at_pressure(850.0);
        let stability_term = ((t_950 - t_850) - 4.0).clamp(0.0, 3.0) as u8;
        
        let td_850 = self.dewpoint_at_pressure(850.0);
        let moisture_term = ((t_850 - td_850) / 3.0).clamp(0.0, 3.0) as u8;
        
        2 + stability_term + moisture_term // Range: 2-6
    }
    
    /// Check if conditions support pyrocumulus development
    pub fn pyrocumulus_potential(&self, fire_intensity_kwm: f32) -> bool {
        // Requires: unstable atmosphere + sufficient fire intensity
        let min_intensity = 10000.0; // kW/m (high-intensity fire)
        let unstable = self.lifted_index < -2.0;
        let strong_fire = fire_intensity_kwm > min_intensity;
        
        unstable && strong_fire && self.mixing_height > 1500.0
    }
}
```

**Implementation Requirements**:
- Standard atmosphere model (ICAO, 1993)
- Haines Index calculation (peer-reviewed formula)
- Lifted Index and K-Index meteorological standards
- Unit tests against published atmospheric profiles

### 2.2 - Pyrocumulus Cloud Formation

**Location**: `crates/core/src/weather/pyrocumulus.rs` (new file)

```rust
/// Fire-generated cloud system
pub struct PyrocumulusCloud {
    pub position: Vec3,                 // Cloud base position
    pub base_altitude: f32,             // meters AGL
    pub top_altitude: f32,              // meters AGL
    pub horizontal_extent: f32,         // meters radius
    
    // Dynamics
    pub updraft_velocity: f32,          // m/s
    pub condensation_rate: f32,         // kg/s
    pub precipitation: bool,            // Producing rain?
    
    // Fire feedback effects
    pub wind_modification: Vec3,        // Inflow/outflow winds
    pub humidity_increase: f32,         // % increase downwind
    
    // Severe weather potential
    pub rotation_detected: bool,        // Fire tornado risk
    pub lightning_strikes: u32,         // New ignitions possible
}

impl PyrocumulusCloud {
    /// Create from high-intensity fire
    pub fn try_form(
        fire_position: Vec3,
        fire_intensity: f32,              // kW/m
        atmosphere: &AtmosphericProfile,
        ambient_humidity: f32
    ) -> Option<Self> {
        if !atmosphere.pyrocumulus_potential(fire_intensity) {
            return None;
        }
        
        // Calculate convective available potential energy (CAPE)
        let parcel_temp = Self::calculate_plume_temperature(fire_intensity);
        let cape = atmosphere.calculate_cape(parcel_temp, fire_position.z);
        
        if cape < 500.0 {
            return None; // Insufficient instability
        }
        
        // Cloud base at lifting condensation level (LCL)
        let lcl = Self::calculate_lcl(parcel_temp, ambient_humidity);
        
        Some(PyrocumulusCloud {
            position: fire_position,
            base_altitude: lcl,
            top_altitude: lcl + (cape / 1000.0) * 500.0, // CAPE-dependent
            horizontal_extent: (fire_intensity / 1000.0).sqrt() * 100.0,
            updraft_velocity: (2.0 * cape).sqrt(), // Buoyancy-driven
            condensation_rate: 0.0,
            precipitation: false,
            wind_modification: Vec3::ZERO,
            rotation_detected: false,
            lightning_strikes: 0,
        })
    }
    
    /// Update cloud dynamics and fire feedback
    pub fn update(&mut self, dt: f32, atmosphere: &AtmosphericProfile) {
        // 1. Updraft evolution
        self.updraft_velocity -= 0.5 * dt; // Entrainment weakening
        
        // 2. Horizontal extent growth
        self.horizontal_extent += self.updraft_velocity * 0.1 * dt;
        
        // 3. Precipitation development (if cloud depth > 3 km)
        if self.top_altitude - self.base_altitude > 3000.0 {
            self.condensation_rate = self.updraft_velocity * 0.01;
            self.precipitation = self.condensation_rate > 0.5;
        }
        
        // 4. Inflow/outflow wind generation
        self.calculate_wind_modification(atmosphere);
        
        // 5. Fire tornado potential (high shear + rotation)
        self.check_fire_tornado_risk(atmosphere);
        
        // 6. Lightning strike potential
        if self.precipitation && rand::random::<f32>() < 0.001 * dt {
            self.lightning_strikes += 1;
        }
    }
    
    fn calculate_wind_modification(&mut self, atmosphere: &AtmosphericProfile) {
        // Inflow at base (converging toward fire)
        let inflow_strength = self.updraft_velocity * 0.3;
        
        // Outflow aloft (diverging from cloud top)
        let outflow_altitude = self.top_altitude;
        let outflow_wind = atmosphere.wind_at_altitude(outflow_altitude);
        
        self.wind_modification = outflow_wind * 0.5;
    }
    
    fn check_fire_tornado_risk(&mut self, atmosphere: &AtmosphericProfile) {
        // Requires: strong updraft + wind shear + vorticity
        let shear = atmosphere.calculate_wind_shear(0.0, self.base_altitude);
        let strong_updraft = self.updraft_velocity > 20.0;
        let high_shear = shear > 0.01; // 1/s
        
        self.rotation_detected = strong_updraft && high_shear;
    }
    
    /// Calculate plume temperature rise from fire intensity
    /// Source: Byram's plume equations
    fn calculate_plume_temperature(intensity_kwm: f32) -> f32 {
        // ΔT = 0.1 × I^0.33 (empirical)
        let ambient = 30.0; // °C (assume hot day)
        ambient + 0.1 * intensity_kwm.powf(0.33)
    }
    
    /// Lifting Condensation Level (LCL) calculation
    fn calculate_lcl(temperature: f32, relative_humidity: f32) -> f32 {
        // Espy's equation: LCL = 125 × (T - Td)
        let dewpoint = temperature - ((100.0 - relative_humidity) / 5.0);
        125.0 * (temperature - dewpoint)
    }
}
```

**Implementation Requirements**:
- CAPE calculation (Convective Available Potential Energy)
- LCL/CCL thermodynamic equations
- Plume rise models (Briggs equations)
- Validation against documented pyrocumulus events

### 2.3 - Fire Tornado Physics

**Location**: `crates/core/src/weather/fire_tornado.rs` (new file)

```rust
/// Fire-induced vortex (fire tornado/fire whirl)
pub struct FireTornado {
    pub position: Vec3,
    pub core_radius: f32,               // meters (1-10m typical)
    pub outer_radius: f32,              // meters (10-50m)
    pub height: f32,                    // meters (up to 500m)
    
    // Dynamics
    pub tangential_velocity: f32,       // m/s (up to 50 m/s)
    pub updraft_velocity: f32,          // m/s (core)
    pub vorticity: f32,                 // 1/s
    
    // Fire effects
    pub intensity_multiplier: f32,      // 2-5x normal spread
    pub ember_lofting_height: f32,      // meters (extreme spotting)
}

impl FireTornado {
    /// Attempt formation from pyrocumulus + wind shear
    pub fn try_form(
        cloud: &PyrocumulusCloud,
        atmosphere: &AtmosphericProfile,
        fire_intensity: f32
    ) -> Option<Self> {
        if !cloud.rotation_detected {
            return None;
        }
        
        // Rankine vortex model initialization
        let core_radius = 5.0; // meters
        let circulation = atmosphere.calculate_circulation(cloud.position);
        let tangential_vel = circulation / (2.0 * PI * core_radius);
        
        if tangential_vel < 10.0 {
            return None; // Insufficient rotation
        }
        
        Some(FireTornado {
            position: cloud.position,
            core_radius,
            outer_radius: core_radius * 5.0,
            height: cloud.base_altitude * 0.7,
            tangential_velocity: tangential_vel,
            updraft_velocity: cloud.updraft_velocity * 1.5,
            vorticity: tangential_vel / core_radius,
            intensity_multiplier: 3.0,
            ember_lofting_height: cloud.base_altitude * 2.0,
        })
    }
    
    /// Calculate wind field at position (for fire spread modification)
    pub fn wind_at_position(&self, pos: Vec3) -> Vec3 {
        let r = ((pos.x - self.position.x).powi(2) 
               + (pos.y - self.position.y).powi(2)).sqrt();
        
        if r > self.outer_radius {
            return Vec3::ZERO;
        }
        
        // Rankine vortex model
        let tangential_speed = if r < self.core_radius {
            // Solid body rotation
            self.tangential_velocity * (r / self.core_radius)
        } else {
            // Free vortex
            self.tangential_velocity * (self.core_radius / r)
        };
        
        // Tangent direction (perpendicular to radius)
        let dx = pos.x - self.position.x;
        let dy = pos.y - self.position.y;
        let tangent = Vec3::new(-dy, dx, 0.0).normalize();
        
        tangent * tangential_speed + Vec3::Z * self.updraft_velocity
    }
}
```

**Implementation Requirements**:
- Rankine vortex model (classical fluid dynamics)
- Vorticity calculation from wind shear
- Validation against fire whirl research (NIST, BRI)

**Scientific References**:
- Forthofer & Goodrick (2011): "Review of Vortices in Wildland Fire"
- Chuah et al. (2011): "Modeling a Fire Whirl Generated over a 5-cm-Diameter Methanol Pool Fire"

### 2.4 - Weather System Integration

**Location**: `crates/core/src/weather/mod.rs` (modify existing)

```rust
pub struct WeatherState {
    // ... existing fields ...
    
    /// Atmospheric profile for instability calculations
    pub atmosphere: AtmosphericProfile,
    
    /// Active pyrocumulus clouds
    pub pyrocumulus_clouds: Vec<PyrocumulusCloud>,
    
    /// Active fire tornadoes
    pub fire_tornadoes: Vec<FireTornado>,
}

impl WeatherState {
    pub fn update_advanced_phenomena(&mut self, fire_simulation: &FireSimulation, dt: f32) {
        // 1. Update atmospheric profile (diurnal heating)
        self.atmosphere.update_profile(self.temperature, self.humidity, self.time_of_day);
        
        // 2. Check for pyrocumulus formation
        for fire_element in fire_simulation.burning_elements() {
            let intensity = fire_element.byram_fireline_intensity();
            if intensity > 10000.0 {
                if let Some(cloud) = PyrocumulusCloud::try_form(
                    fire_element.position,
                    intensity,
                    &self.atmosphere,
                    self.humidity
                ) {
                    self.pyrocumulus_clouds.push(cloud);
                }
            }
        }
        
        // 3. Update existing clouds
        for cloud in &mut self.pyrocumulus_clouds {
            cloud.update(dt, &self.atmosphere);
            
            // 4. Check for fire tornado formation
            if cloud.rotation_detected {
                if let Some(tornado) = FireTornado::try_form(
                    cloud,
                    &self.atmosphere,
                    10000.0 // min intensity
                ) {
                    self.fire_tornadoes.push(tornado);
                }
            }
        }
        
        // 5. Remove dissipated phenomena
        self.pyrocumulus_clouds.retain(|c| c.updraft_velocity > 1.0);
        self.fire_tornadoes.retain(|t| t.tangential_velocity > 5.0);
    }
    
    /// Get effective wind at position (including tornado effects)
    pub fn wind_at_position(&self, pos: Vec3) -> Vec3 {
        let mut wind = self.wind_vector();
        
        // Add fire tornado winds
        for tornado in &self.fire_tornadoes {
            wind += tornado.wind_at_position(pos);
        }
        
        // Add pyrocumulus inflow/outflow
        for cloud in &self.pyrocumulus_clouds {
            let dist = (pos - cloud.position).length();
            if dist < cloud.horizontal_extent * 2.0 {
                wind += cloud.wind_modification;
            }
        }
        
        wind
    }
}
```

### 2.5 - FFI Advanced Weather Interface

**Location**: `crates/ffi/src/lib.rs` (add functions)

```rust
#[repr(C)]
pub struct PyrocumulusCloudData {
    pub position: [f32; 3],
    pub base_altitude: f32,
    pub top_altitude: f32,
    pub radius: f32,
    pub updraft_velocity: f32,
    pub has_precipitation: u8,
    pub has_lightning: u8,
}

#[repr(C)]
pub struct FireTornadoData {
    pub position: [f32; 3],
    pub core_radius: f32,
    pub outer_radius: f32,
    pub height: f32,
    pub tangential_velocity: f32,
}

/// Query active pyrocumulus clouds (for game engine rendering)
#[no_mangle]
pub extern "C" fn fire_sim_get_pyrocumulus_clouds(
    sim_id: usize,
    out_count: *mut u32
) -> *const PyrocumulusCloudData {
    // Returns array of cloud data for rendering
}

/// Query active fire tornadoes
#[no_mangle]
pub extern "C" fn fire_sim_get_fire_tornadoes(
    sim_id: usize,
    out_count: *mut u32
) -> *const FireTornadoData {
    // Returns array of tornado data for rendering
}

/// Get Haines Index (fire weather severity indicator)
#[no_mangle]
pub extern "C" fn fire_sim_get_haines_index(sim_id: usize) -> u8 {
    // Returns 2-6 (low to extreme fire weather)
}
```

### 2.6 - Validation & Testing

**Location**: `crates/core/tests/advanced_weather.rs` (new file)

Required tests:
- Haines Index calculation against published examples
- Pyrocumulus formation thresholds (>10,000 kW/m intensity)
- LCL calculation accuracy (±100m)
- Fire tornado wind velocities (10-50 m/s range)
- CAPE calculation validation
- Atmospheric stability indices (LI, K-index)

**Scientific References**:
- Haines, D.A. (1988): "A Lower Atmosphere Severity Index for Wildlife Fires"
- Fromm et al. (2010): "The Untold Story of Pyrocumulonimbus"
- NIST TN 1713: "Fire-Induced Flows in Wildland Fires"

═══════════════════════════════════════════════════════════════════════
## PHASE 3: TERRAIN DATA INTEGRATION
═══════════════════════════════════════════════════════════════════════

**Goal**: Implement terrain elevation, slope, and aspect data structures for realistic fire spread on complex topography. Visualization handled by game engine.

### 3.1 - Terrain Data Structures

**Location**: `crates/core/src/terrain/mod.rs` (new file)

```rust
/// Digital Elevation Model (DEM) representation
pub struct TerrainModel {
    // Elevation data
    pub width: usize,                   // Grid cells X
    pub height: usize,                  // Grid cells Y
    pub cell_size: f32,                 // meters
    pub elevations: Vec<f32>,           // meters (row-major)
    
    // Origin (southwest corner)
    pub origin_x: f32,
    pub origin_y: f32,
    
    // Derived slope/aspect (cached)
    pub slopes: Vec<f32>,               // degrees
    pub aspects: Vec<f32>,              // degrees (0-360, N=0)
    
    // Vegetation/fuel mapping (optional)
    pub fuel_type_map: Option<Vec<u8>>, // FuelType ID per cell
}

impl TerrainModel {
    /// Create from elevation grid
    pub fn new(
        width: usize, 
        height: usize, 
        cell_size: f32,
        elevations: Vec<f32>,
        origin: (f32, f32)
    ) -> Self {
        let mut terrain = TerrainModel {
            width,
            height,
            cell_size,
            elevations,
            origin_x: origin.0,
            origin_y: origin.1,
            slopes: Vec::new(),
            aspects: Vec::new(),
            fuel_type_map: None,
        };
        
        // Calculate slope/aspect from elevation
        terrain.calculate_slope_aspect();
        terrain
    }
    
    /// Calculate slope and aspect using Horn's method (3x3 kernel)
    fn calculate_slope_aspect(&mut self) {
        self.slopes = vec![0.0; self.width * self.height];
        self.aspects = vec![0.0; self.width * self.height];
        
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let idx = y * self.width + x;
                
                // Horn's 3x3 kernel for gradient estimation
                let z = [
                    self.get_elevation(x-1, y-1), self.get_elevation(x, y-1), self.get_elevation(x+1, y-1),
                    self.get_elevation(x-1, y),   self.get_elevation(x, y),   self.get_elevation(x+1, y),
                    self.get_elevation(x-1, y+1), self.get_elevation(x, y+1), self.get_elevation(x+1, y+1),
                ];
                
                // Gradient calculation
                let dz_dx = ((z[2] + 2.0*z[5] + z[8]) - (z[0] + 2.0*z[3] + z[6])) / (8.0 * self.cell_size);
                let dz_dy = ((z[6] + 2.0*z[7] + z[8]) - (z[0] + 2.0*z[1] + z[2])) / (8.0 * self.cell_size);
                
                // Slope (degrees)
                let slope_rad = (dz_dx*dz_dx + dz_dy*dz_dy).sqrt().atan();
                self.slopes[idx] = slope_rad.to_degrees();
                
                // Aspect (degrees, N=0, E=90)
                let aspect_rad = dz_dy.atan2(dz_dx);
                self.aspects[idx] = (90.0 - aspect_rad.to_degrees() + 360.0) % 360.0;
            }
        }
    }
    
    /// Query elevation at world position (bilinear interpolation)
    pub fn elevation_at(&self, world_x: f32, world_y: f32) -> f32 {
        let grid_x = (world_x - self.origin_x) / self.cell_size;
        let grid_y = (world_y - self.origin_y) / self.cell_size;
        
        // Bilinear interpolation
        let x0 = grid_x.floor() as usize;
        let y0 = grid_y.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        
        let fx = grid_x.fract();
        let fy = grid_y.fract();
        
        let z00 = self.get_elevation(x0, y0);
        let z10 = self.get_elevation(x1, y0);
        let z01 = self.get_elevation(x0, y1);
        let z11 = self.get_elevation(x1, y1);
        
        let z0 = z00 * (1.0 - fx) + z10 * fx;
        let z1 = z01 * (1.0 - fx) + z11 * fx;
        z0 * (1.0 - fy) + z1 * fy
    }
    
    /// Query slope at world position
    pub fn slope_at(&self, world_x: f32, world_y: f32) -> f32 {
        let grid_x = ((world_x - self.origin_x) / self.cell_size) as usize;
        let grid_y = ((world_y - self.origin_y) / self.cell_size) as usize;
        
        if grid_x < self.width && grid_y < self.height {
            self.slopes[grid_y * self.width + grid_x]
        } else {
            0.0
        }
    }
    
    /// Query aspect at world position
    pub fn aspect_at(&self, world_x: f32, world_y: f32) -> f32 {
        let grid_x = ((world_x - self.origin_x) / self.cell_size) as usize;
        let grid_y = ((world_y - self.origin_y) / self.cell_size) as usize;
        
        if grid_x < self.width && grid_y < self.height {
            self.aspects[grid_y * self.width + grid_x]
        } else {
            0.0
        }
    }
    
    fn get_elevation(&self, x: usize, y: usize) -> f32 {
        self.elevations[y * self.width + x]
    }
}
```

**Implementation Requirements**:
- Horn's method for slope/aspect (standard GIS algorithm)
- Bilinear interpolation for continuous elevation queries
- Support for large DEMs (10km+ extent, 10m resolution)
- Memory-efficient storage (consider compression for large terrains)

### 3.2 - Enhanced Slope Fire Spread

**Location**: `crates/core/src/physics/slope.rs` (modify existing)

```rust
/// Calculate slope effect on fire spread using terrain model
pub fn slope_spread_multiplier_terrain(
    from: &FuelElement,
    to: &FuelElement,
    terrain: &TerrainModel
) -> f32 {
    // Get accurate slope angle from terrain
    let mid_x = (from.position.x + to.position.x) / 2.0;
    let mid_y = (from.position.y + to.position.y) / 2.0;
    let slope_angle = terrain.slope_at(mid_x, mid_y);
    
    // Get aspect (slope direction)
    let aspect = terrain.aspect_at(mid_x, mid_y);
    
    // Fire spread direction
    let spread_direction = (to.position - from.position).normalize();
    let spread_angle = spread_direction.y.atan2(spread_direction.x).to_degrees();
    
    // Alignment with slope (uphill = positive, downhill = negative)
    let aspect_alignment = ((spread_angle - aspect).abs() - 180.0).abs() / 180.0;
    let effective_slope = slope_angle * aspect_alignment;
    
    // Existing slope formula with accurate terrain data
    if effective_slope > 0.0 {
        // Uphill: exponential effect
        1.0 + (effective_slope / 10.0).powf(1.5) * 2.0
    } else {
        // Downhill: reduced spread
        (1.0 + effective_slope / 30.0).max(0.3)
    }
}
```

### 3.3 - Aspect-Wind Interaction

**Location**: `crates/core/src/physics/aspect_wind.rs` (new file)

```rust
/// Calculate combined aspect-wind effect on fire spread
pub fn aspect_wind_multiplier(
    terrain: &TerrainModel,
    position: Vec3,
    wind_vector: Vec3
) -> f32 {
    let aspect = terrain.aspect_at(position.x, position.y);
    let slope = terrain.slope_at(position.x, position.y);
    
    // Wind direction (degrees)
    let wind_direction = wind_vector.y.atan2(wind_vector.x).to_degrees();
    
    // Alignment: aspect facing wind = sheltered, opposite = exposed
    let alignment = ((wind_direction - aspect).abs() - 180.0).abs();
    
    if alignment < 90.0 {
        // Windward slope (exposed) - enhanced spread
        1.0 + (slope / 45.0) * 0.5
    } else {
        // Leeward slope (sheltered) - reduced spread
        1.0 - (slope / 45.0) * 0.3
    }
}
```

### 3.4 - Simulation Integration

**Location**: `crates/core/src/simulation/mod.rs` (modify existing)

```rust
pub struct FireSimulation {
    // ... existing fields ...
    
    /// Terrain model (optional - flat if None)
    pub terrain: Option<TerrainModel>,
}

impl FireSimulation {
    /// Update fuel element positions with terrain elevation
    pub fn update_element_elevations(&mut self) {
        if let Some(ref terrain) = self.terrain {
            for element in &mut self.fuel_elements {
                let elevation = terrain.elevation_at(element.position.x, element.position.y);
                element.position.z = elevation + element.height_above_ground;
            }
        }
    }
    
    /// Enhanced heat transfer with terrain-aware slope
    fn calculate_heat_transfer(&self, source: &FuelElement, target: &FuelElement) -> f32 {
        // ... existing radiation calculation ...
        
        // Apply terrain-aware slope multiplier
        let slope_factor = if let Some(ref terrain) = self.terrain {
            slope_spread_multiplier_terrain(source, target, terrain)
        } else {
            // Fallback to element-based slope
            slope_spread_multiplier(source, target)
        };
        
        // ... rest of calculation ...
    }
}
```

### 3.5 - FFI Terrain Interface

**Location**: `crates/ffi/src/lib.rs` (add functions)

```rust
/// Load terrain from elevation grid (game engine provides DEM data)
#[no_mangle]
pub extern "C" fn fire_sim_load_terrain(
    sim_id: usize,
    width: u32,
    height: u32,
    cell_size: f32,
    origin_x: f32,
    origin_y: f32,
    elevations: *const f32,
    elevations_count: u32
) -> bool {
    // Load terrain into simulation
}

/// Query terrain elevation at world position
#[no_mangle]
pub extern "C" fn fire_sim_terrain_elevation(
    sim_id: usize,
    x: f32,
    y: f32
) -> f32 {
    // Returns elevation in meters (or 0 if no terrain)
}

/// Query terrain slope at world position
#[no_mangle]
pub extern "C" fn fire_sim_terrain_slope(
    sim_id: usize,
    x: f32,
    y: f32
) -> f32 {
    // Returns slope in degrees
}

/// Query terrain aspect at world position
#[no_mangle]
pub extern "C" fn fire_sim_terrain_aspect(
    sim_id: usize,
    x: f32,
    y: f32
) -> f32 {
    // Returns aspect in degrees (N=0, E=90)
}
```

### 3.6 - Validation & Testing

**Location**: `crates/core/tests/terrain_integration.rs` (new file)

Required tests:
- Horn's method slope calculation accuracy (±1°)
- Bilinear interpolation smoothness
- Aspect calculation validation (cardinal directions)
- Slope-fire spread multiplier with terrain data
- Large DEM performance (10,000+ cells)
- Edge case handling (terrain boundaries)

**Scientific References**:
- Horn, B.K.P. (1981): "Hill Shading and the Reflectance Map"
- Burrough & McDonnell (1998): "Principles of Geographical Information Systems"
- Rothermel (1972): Slope factor formulation (original paper)

═══════════════════════════════════════════════════════════════════════
## PHASE 4: REAL-TIME WEATHER DATA INTEGRATION (OPTIONAL)
═══════════════════════════════════════════════════════════════════════

**Goal**: Support real-time weather data ingestion from external APIs (BOM, NOAA, etc.) for realistic scenario initialization.

### 4.1 - Weather Data Provider Interface

**Location**: `crates/core/src/weather/providers.rs` (new file)

```rust
/// Weather data source interface
pub trait WeatherDataProvider {
    /// Fetch current weather at location
    fn fetch_current(&self, lat: f32, lon: f32) -> Result<WeatherSnapshot, Error>;
    
    /// Fetch forecast weather
    fn fetch_forecast(&self, lat: f32, lon: f32, hours_ahead: u32) -> Result<Vec<WeatherSnapshot>, Error>;
}

/// Point-in-time weather observation
#[derive(Debug, Clone)]
pub struct WeatherSnapshot {
    pub timestamp: u64,                 // Unix time
    pub location: (f32, f32),           // (lat, lon)
    
    // Surface observations
    pub temperature: f32,               // °C
    pub relative_humidity: f32,         // %
    pub wind_speed: f32,                // km/h
    pub wind_direction: f32,            // degrees
    pub pressure: f32,                  // hPa
    
    // Fire weather indices
    pub ffdi: Option<f32>,              // If available from provider
    pub drought_factor: Option<f32>,
    pub fuel_curing: Option<f32>,       // %
    
    // Upper air (if available)
    pub atmospheric_profile: Option<AtmosphericProfile>,
}

/// Bureau of Meteorology (Australia) provider
pub struct BOMWeatherProvider {
    api_key: String,
    base_url: String,
}

impl WeatherDataProvider for BOMWeatherProvider {
    fn fetch_current(&self, lat: f32, lon: f32) -> Result<WeatherSnapshot, Error> {
        // HTTP request to BOM API
        // Parse JSON response into WeatherSnapshot
        unimplemented!("Requires HTTP client dependency")
    }
    
    fn fetch_forecast(&self, lat: f32, lon: f32, hours_ahead: u32) -> Result<Vec<WeatherSnapshot>, Error> {
        unimplemented!("Requires HTTP client dependency")
    }
}
```

**Implementation Requirements**:
- HTTP client with async support (e.g., `reqwest`)
- JSON parsing (e.g., `serde_json`)
- Error handling for network failures
- Rate limiting to respect API terms of service
- Caching to reduce API calls

### 4.2 - Weather State Initialization from Live Data

**Location**: `crates/core/src/weather/mod.rs` (extend existing)

```rust
impl WeatherState {
    /// Initialize from real-time weather observation
    pub fn from_live_data(snapshot: WeatherSnapshot, day_of_year: u32) -> Self {
        let mut weather = WeatherState::new();
        
        // Surface conditions
        weather.temperature = snapshot.temperature;
        weather.humidity = snapshot.relative_humidity;
        weather.wind_speed = snapshot.wind_speed;
        weather.wind_direction = snapshot.wind_direction;
        weather.pressure = snapshot.pressure;
        
        // Fire weather indices (use observed if available, else calculate)
        if let Some(ffdi) = snapshot.ffdi {
            // BOM provides FFDI directly
            weather.drought_factor = Self::reverse_calculate_drought_factor(
                ffdi, weather.temperature, weather.humidity, weather.wind_speed
            );
        } else if let Some(df) = snapshot.drought_factor {
            weather.drought_factor = df;
        }
        
        // Fuel curing
        if let Some(curing) = snapshot.fuel_curing {
            weather.fuel_curing_percent = curing;
        } else {
            // Estimate from season
            weather.fuel_curing_percent = Self::estimate_curing(day_of_year);
        }
        
        // Atmospheric profile (if available)
        if let Some(profile) = snapshot.atmospheric_profile {
            weather.atmosphere = profile;
        } else {
            // Use standard atmosphere
            weather.atmosphere = AtmosphericProfile::standard(weather.temperature);
        }
        
        weather
    }
}
```

### 4.3 - FFI Live Weather Interface

**Location**: `crates/ffi/src/lib.rs` (add functions)

```rust
/// Initialize weather from live data (game engine fetches externally)
#[no_mangle]
pub extern "C" fn fire_sim_set_weather_from_live(
    sim_id: usize,
    temperature: f32,
    humidity: f32,
    wind_speed: f32,
    wind_direction: f32,
    pressure: f32,
    ffdi: f32,
    drought_factor: f32,
    fuel_curing: f32
) {
    // Update simulation weather state from external data
}
```

**Note**: Game engine responsible for HTTP requests and API key management. Core simulation only processes provided data.

### 4.4 - Validation & Testing

**Location**: `crates/core/tests/live_weather.rs` (new file)

Required tests:
- Parse sample BOM JSON responses
- Validate FFDI reverse calculation
- Handle missing data fields gracefully
- Weather state initialization correctness

═══════════════════════════════════════════════════════════════════════
## PHASE 5: MULTIPLAYER/CO-OP SUPPORT (PHYSICS LAYER ONLY)
═══════════════════════════════════════════════════════════════════════

**Goal**: Provide action-based replication support for multiplayer scenarios. Game engine handles networking, simulation provides deterministic physics.

### 5.1 - Player Action Queue System

**Location**: `crates/core/src/simulation/action_queue.rs` (new file)

```rust
/// Player action types for replication
#[derive(Debug, Clone, Copy)]
pub enum PlayerActionType {
    ApplySuppression,
    IgniteSpot,
    ModifyWeather,  // For scenario control
}

/// Replicatable player action
#[derive(Debug, Clone)]
pub struct PlayerAction {
    pub action_type: PlayerActionType,
    pub player_id: u32,
    pub timestamp: f32,              // Simulation time
    pub position: Vec3,
    pub param1: f32,                 // Mass, intensity, etc.
    pub param2: u32,                 // Type ID, element ID, etc.
}

/// Action queue for deterministic replay
pub struct ActionQueue {
    pending: Vec<PlayerAction>,
    executed: Vec<PlayerAction>,     // History for late joiners
    max_history: usize,              // Limit history size
}

impl ActionQueue {
    pub fn submit_action(&mut self, action: PlayerAction) {
        self.pending.push(action);
    }
    
    pub fn process_pending(&mut self, sim: &mut FireSimulation) {
        for action in self.pending.drain(..) {
            match action.action_type {
                PlayerActionType::ApplySuppression => {
                    let agent = SuppressionAgent::from_type_id(action.param2 as u8);
                    sim.apply_suppression_direct(action.position, action.param1, agent);
                }
                PlayerActionType::IgniteSpot => {
                    sim.ignite_at_position(action.position);
                }
                PlayerActionType::ModifyWeather => {
                    // Reserved for scenario control
                }
            }
            
            // Store in history
            self.executed.push(action);
            if self.executed.len() > self.max_history {
                self.executed.remove(0);
            }
        }
    }
    
    pub fn get_history(&self) -> &[PlayerAction] {
        &self.executed
    }
}
```

### 5.2 - FFI Action Queue Interface

**Location**: `crates/ffi/src/lib.rs` (add functions)

```rust
#[repr(C)]
pub struct CPlayerAction {
    pub action_type: u8,
    pub player_id: u32,
    pub timestamp: f32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub param1: f32,
    pub param2: u32,
}

/// Submit player action to simulation (from game engine)
/// 
/// Game engine:
/// - Receives action from client via network
/// - Validates action (anti-cheat)
/// - Submits to simulation
/// - Replicates to all clients
/// 
/// # Returns
/// true if action was accepted and queued
#[no_mangle]
pub extern "C" fn fire_sim_submit_player_action(
    sim_id: usize,
    action: *const CPlayerAction,
) -> bool {
    // Queue action for processing in next update
}

/// Get pending actions for this frame (for replication)
/// 
/// Game engine:
/// - Calls this after fire_sim_update
/// - Sends actions to all clients
/// - Clients execute same actions locally
/// 
/// # Returns
/// Pointer to array of actions executed this frame
#[no_mangle]
pub extern "C" fn fire_sim_get_pending_actions(
    sim_id: usize,
    out_count: *mut u32,
) -> *const CPlayerAction {
    // Returns actions executed in last update (for broadcast)
}

/// Get action history (for late joiners)
/// 
/// Game engine:
/// - Calls this when new player joins
/// - Sends full history to new player
/// - New player replays all actions to sync state
/// 
/// # Returns
/// Pointer to array of all historical actions
#[no_mangle]
pub extern "C" fn fire_sim_get_action_history(
    sim_id: usize,
    out_count: *mut u32,
) -> *const CPlayerAction {
    // Returns full action history for synchronization
}
```

### 5.3 - Simulation State Snapshot (Late Joiner Sync)

**Location**: `crates/ffi/src/lib.rs` (add functions)

```rust
#[repr(C)]
pub struct SimulationSnapshot {
    pub frame_number: u32,
    pub simulation_time: f32,
    pub burning_element_count: u32,
    pub total_fuel_consumed: f32,
    pub active_ember_count: u32,
}

/// Get current simulation state summary
/// 
/// Game engine uses this for:
/// - Displaying statistics to players
/// - Synchronization checks (frame number matching)
/// - Late joiner validation (is sync successful?)
#[no_mangle]
pub extern "C" fn fire_sim_get_snapshot(
    sim_id: usize,
    out_snapshot: *mut SimulationSnapshot,
) -> bool {
    // Populate snapshot with current state
}
```

**Key Design Points**:
- **Simulation is deterministic** - same actions produce same results on all clients
- **Action-based replication** - only replicate player commands, not fire state
- **Each client runs local simulation** - no network lag for fire physics
- **Server validates actions** - anti-cheat handled by game engine
- **Late joiners replay history** - deterministic replay catches them up
- **Game engine handles networking** - simulation only provides action queue

**Game Engine Responsibilities**:
- Network transport (send/receive actions)
- Action validation (anti-cheat)
- Late joiner synchronization flow
- Player identification and authentication
- Network error handling and reconnection

**Simulation Responsibilities**:
- Deterministic physics (same inputs = same outputs)
- Action queue management
- Action history storage
- Frame-perfect action execution

═══════════════════════════════════════════════════════════════════════
## PHASE 6: OPTIMIZATION & PERFORMANCE ENHANCEMENTS
═══════════════════════════════════════════════════════════════════════

**Goal**: Ensure all new features maintain 60 FPS with 600,000+ fuel elements and 1,000+ burning elements.

**Note**: This phase was previously Phase 5, renumbered due to addition of multiplayer support phase.

### 5.1 - Profiling Targets

Before optimization, profile with realistic scenarios:
- 100,000 fuel elements, 200 burning, 1 pyrocumulus cloud
- 500,000 fuel elements, 500 burning, 3 fire tornadoes
- 600,000 fuel elements, 1,000 burning, suppression applied, terrain enabled

**Tools**:
- `cargo flamegraph` - CPU profiling
- `cargo instruments -t time` - macOS performance analysis
- `valgrind --tool=massif` - Memory profiling

### 5.2 - Spatial Index Enhancements

**Location**: `crates/core/src/spatial/octree.rs` (modify existing)

Optimizations:
- Pre-allocate octree cells for known terrain extent
- Batch insert/remove operations
- Cache frequent queries (e.g., 15m radius neighbor lists)
- Parallel spatial queries with `rayon`

### 5.3 - Weather Phenomena Culling

**Location**: `crates/core/src/weather/mod.rs`

Optimizations:
- Only update pyrocumulus clouds near active fire
- Cull distant fire tornadoes (>1km from any burning element)
- LOD for atmospheric calculations (coarse grid far from fire)

### 5.4 - Suppression Coverage Optimization

**Location**: `crates/core/src/suppression/coverage.rs`

Optimizations:
- Batch suppression updates (update every N frames)
- Skip evaporation calculations for thick coverage (>10 kg/m²)
- Early exit for depleted coverage (mark inactive)

### 5.5 - Terrain Query Caching

**Location**: `crates/core/src/terrain/mod.rs`

Optimizations:
- Cache slope/aspect per fuel element (update only if element moves)
- Use coarser terrain grid for distant fire (LOD)
- SIMD bilinear interpolation for batch queries

### 5.6 - Parallel Processing Expansion

Use `rayon` for:
- Suppression coverage updates (independent per element)
- Pyrocumulus cloud updates (independent per cloud)
- Terrain elevation queries (batch processing)
- Fire tornado wind field calculations

### 5.7 - Performance Benchmarks

**Location**: `crates/core/benches/` (new benchmarks)

Required benchmarks:
- Suppression application: <0.1ms per 100 elements
- Pyrocumulus formation check: <0.05ms per high-intensity fire
- Terrain elevation query: <0.001ms per query (bilinear)
- Fire tornado wind field: <0.01ms per position
- Full simulation step: <16ms (60 FPS) with all features enabled

═══════════════════════════════════════════════════════════════════════
## IMPLEMENTATION WORKFLOW
═══════════════════════════════════════════════════════════════════════

### Step-by-Step Process for Each Phase

1. **Research Phase**
   - Find peer-reviewed papers for phenomenon
   - Document formulas and empirical constants
   - Add references to `docs/CITATIONS.bib`

2. **Implementation Phase**
   - Create new modules/files as specified
   - Implement data structures with full documentation
   - Include units in comments (°C, kJ, m/s, etc.)
   - Use exact formulas from literature (NO SIMPLIFICATIONS)

3. **Testing Phase**
   - Write unit tests for each formula
   - Validate against published data/examples
   - Create integration tests for feature interactions
   - Document expected vs actual results

4. **FFI Integration Phase**
   - Add C-compatible functions to `crates/ffi/src/lib.rs`
   - Ensure thread safety (Arc<Mutex<>>)
   - Document game engine responsibilities clearly
   - Add usage examples in comments

5. **Validation Phase**
   - Run full test suite: `cargo test --all-features`
   - Run clippy: `cargo clippy --all-targets --all-features -- -D warnings`
   - Format code: `cargo fmt --all -v`
   - Profile performance: `cargo bench`
   - Create validation document: `docs/validation/PHASE_X_VALIDATION.md`

6. **Documentation Phase**
   - Update `README.md` with new features
   - Add examples to demo applications
   - Update FFI header comments
   - Add troubleshooting notes if needed

### Quality Checklist (Before Marking Phase Complete)

- [ ] All formulas have scientific references cited
- [ ] Units are documented in comments
- [ ] No simplifications made to physics equations
- [ ] Thread-safe FFI functions (Arc<Mutex<>>)
- [ ] Unit tests passing (95%+ coverage)
- [ ] Integration tests passing
- [ ] Clippy warnings = 0
- [ ] Code formatted (rustfmt)
- [ ] Performance benchmarks meet targets
- [ ] Validation document created
- [ ] FFI interface documented
- [ ] Game engine responsibilities clarified

═══════════════════════════════════════════════════════════════════════
## EXCLUDED FEATURES (GAME ENGINE RESPONSIBILITIES)
═══════════════════════════════════════════════════════════════════════

The following are **NOT implemented** in the core simulation. The game engine will handle:

### ❌ Visual Rendering
- Particle systems (suppression spray, smoke, embers)
- Fire materials and shaders
- Weather visualization (clouds, tornadoes)
- Terrain mesh rendering
- UI elements

### ❌ Character Operations
- Firefighter movement and animations
- Hose handling mechanics (grab, drag, connect)
- Equipment interaction (pull hose from truck)
- Heat stress visual effects
- Injury/fatality mechanics

### ❌ Vehicle Systems
- Fire truck driving physics
- Water tank visual level
- Equipment deployment UI (roll out hose)
- Pump controls and gauges
- Vehicle damage visualization

### ❌ Communications
- Radio UI and voice simulation
- Incident command hierarchy UI
- Resource dispatch interface
- Map annotations and waypoints

### ❌ Scenario Management
- Mission objectives and scoring
- Save/load functionality
- Replay system
- Multiplayer networking (server/client infrastructure)

### ❌ Firefighter Operations
- Character movement, pathfinding, animations
- Hose deployment state machines
- Equipment interaction (grab, drag, connect)
- Heat stress progression and injury tracking
- Energy/fatigue management
- Team coordination and AI behaviors

### ❌ Ember Rendering
- Ember particle effects (Niagara/particle systems)
- Ember trails, glow, and smoke visuals
- Spot fire ignition VFX and sound effects (response to simulation events)

**Core simulation provides**:
- Physics state data (for rendering)
- FFI query functions (for game logic)
- Position-based effect application (suppression, ignition)
- Fire behavior data (intensity, temperature, etc.)
- **Deterministic action processing** (for multiplayer)
- **Ember physics** (position, velocity, temperature)
- **Query interfaces** for fire/suppression state at any position

═══════════════════════════════════════════════════════════════════════
## SCIENTIFIC REFERENCES (TO BE EXPANDED)
═══════════════════════════════════════════════════════════════════════

### Fire Suppression
- NFPA 1150: Standard on Foam Chemicals for Fires in Class A Fuels (2022)
- USFS MTDC: Long-Term Fire Retardant Effectiveness Studies (2019)
- George & Johnson (2009): "Effectiveness of Aerial Fire Retardant"

### Atmospheric Phenomena
- Haines, D.A. (1988): "A Lower Atmosphere Severity Index for Wildlife Fires"
- Fromm et al. (2010): "The Untold Story of Pyrocumulonimbus"
- Peterson et al. (2015): "Meteorology Influencing Springtime Fire Behavior"

### Fire Tornadoes
- Forthofer & Goodrick (2011): "Review of Vortices in Wildland Fire"
- Chuah et al. (2011): "Modeling a Fire Whirl Generated over a Pool Fire"
- NIST TN 1713: "Fire-Induced Flows in Wildland Fires"

### Terrain Analysis
- Horn, B.K.P. (1981): "Hill Shading and the Reflectance Map"
- Burrough & McDonnell (1998): "Principles of Geographical Information Systems"
- USGS Digital Elevation Model Standards (2009)

### Evaporation & Heat Transfer
- Allen et al. (1998): "FAO Irrigation and Drainage Paper 56 - Penman-Monteith"
- Incropera et al. (2011): "Fundamentals of Heat and Mass Transfer"

═══════════════════════════════════════════════════════════════════════
## ARCHITECTURE SUMMARY: SIMULATION vs GAME ENGINE
═══════════════════════════════════════════════════════════════════════

### Core Simulation Responsibilities (THIS IMPLEMENTATION PLAN)

**Fire Physics**:
- ✅ Fire spread calculation (Rothermel model)
- ✅ Heat transfer (radiation, convection, conduction)
- ✅ Moisture evaporation (latent heat)
- ✅ Ignition probability and conditions
- ✅ Ember physics (generation, flight, cooling)
- ✅ **Ember spot fire ignition** (automatic, physics-based)
- ✅ Fuel consumption and burn rates

**Suppression Physics**:
- ✅ Agent properties (water, foam, retardant)
- ✅ Heat absorption calculations
- ✅ Evaporation and degradation over time
- ✅ Chemical combustion inhibition
- ✅ Coverage tracking per fuel element

**Weather Physics**:
- ✅ Atmospheric instability (Haines Index, CAPE)
- ✅ Pyrocumulus cloud formation and evolution
- ✅ Fire tornado dynamics (Rankine vortex)
- ✅ Wind-fire feedback loops
- ✅ Diurnal cycles and seasonal variations

**Terrain Physics**:
- ✅ Slope/aspect calculations (Horn's method)
- ✅ Elevation data structures (DEM)
- ✅ Terrain-aware fire spread multipliers

**Query Interfaces** (for game engine):
- ✅ `fire_sim_query_fire_intensity(x, y, z, radius)` → kW/m
- ✅ `fire_sim_query_radiant_heat(x, y, z)` → kW/m²
- ✅ `fire_sim_query_flame_height(x, y, z, radius)` → meters
- ✅ `fire_sim_query_suppression(x, y, z)` → coverage data
- ✅ `fire_sim_is_position_in_fire(x, y, z, margin)` → bool
- ✅ `fire_sim_get_embers()` → ember state data (for rendering only)
- ✅ `fire_sim_get_spot_fire_events()` → ignitions this frame (for VFX/audio)

**Action Interfaces** (game engine triggers):
- ✅ `fire_sim_apply_suppression(pos, mass, agent_type)`
- ✅ `fire_sim_submit_player_action(action)` (multiplayer)

**Multiplayer Support**:
- ✅ Deterministic physics (same inputs = same outputs)
- ✅ Action queue (record all player commands)
- ✅ Action history (for late joiner replay)
- ✅ State snapshots (for sync validation)

---

### Game Engine Responsibilities (UNREAL/UNITY)

**Rendering**:
- Niagara/particle systems (embers, smoke, suppression spray)
- Fire materials and shaders
- Flame mesh/sprite rendering
- Weather visualization (clouds, tornadoes)
- Terrain mesh and textures

**Firefighter Operations**:
- Character movement and animations
- Hose mechanics (grab, drag, connect, spray)
- Equipment interaction state machines
- Heat stress UI and injury progression
- Energy/fatigue management
- Team coordination and AI

**Gameplay Logic**:
- Mission objectives and scoring
- Player input handling
- UI/HUD rendering
- **Response to spot fire events** (spawn VFX, play audio, show UI notifications)

**Multiplayer**:
- Network transport (send/receive actions)
- Action validation (anti-cheat)
- Client/server synchronization
- Late joiner connection flow
- Player authentication

**Audio**:
- Fire crackling, roaring sounds
- Equipment sounds (pump, spray, hose)
- Radio communications
- Environmental ambience

**Data Flow Example**:
```
Player Sprays Water:
1. Game: Player clicks, raycasts to find spray target
2. Game: Calls fire_sim_apply_suppression(pos, 10kg, WATER)
3. Sim:  Applies suppression physics to fuel elements in radius
4. Game: Spawns Niagara water spray particle effect
5. Game: Calls fire_sim_query_fire_intensity(spray_pos) each frame
6. Game: Updates UI "Fire intensity: 5000 kW/m → 2000 kW/m"
7. Game: If multiplayer, sends action to server for replication

Ember Creates Spot Fire:
1. Sim:  Ember lands (position.z < 1.0), temp = 350°C
2. Sim:  Finds fuel element at landing position
3. Sim:  Checks suppression coverage (20% coverage = OK to ignite)
4. Sim:  Calculates ignition probability: temp × moisture × receptivity
5. Sim:  Random roll succeeds → ignites fuel element
6. Sim:  Adds to spot_fire_events[] for this frame
7. Game: Calls fire_sim_get_spot_fire_events()
8. Game: Spawns explosion VFX, plays ignition sound, shows UI alert
```

═══════════════════════════════════════════════════════════════════════
## COMPLETION CRITERIA
═══════════════════════════════════════════════════════════════════════

### Phase 1 Complete When:
- [ ] All suppression agent types implemented with research-based properties
- [ ] Suppression coverage system working (evaporation, degradation)
- [ ] FFI suppression application functions available
- [ ] FFI fire state query functions available (intensity, heat, flame height)
- [ ] Ember automatic ignition system implemented (physics-based)
- [ ] Spot fire event tracking and FFI retrieval available
- [ ] Suppression blocks ember ignition correctly
- [ ] At least 15 unit tests passing for suppression physics and ember ignition
- [ ] Validation document showing effectiveness matches research

### Phase 2 Complete When:
- [ ] Atmospheric instability calculations implemented (LI, K-index, Haines)
- [ ] Pyrocumulus cloud formation and evolution working
- [ ] Fire tornado physics functional with Rankine vortex model
- [ ] Weather phenomena integrated into fire spread calculations
- [ ] At least 15 unit tests passing for advanced weather
- [ ] Validation document with atmospheric model accuracy

### Phase 3 Complete When:
- [ ] Terrain DEM data structure implemented
- [ ] Horn's method slope/aspect calculation accurate (±1°)
- [ ] Terrain-aware fire spread multipliers working
- [ ] FFI terrain loading/query functions available
- [ ] At least 10 unit tests passing for terrain integration
- [ ] Performance maintained with large DEMs (10km+ extent)

### Phase 4 Complete When:
- [ ] Weather data provider interface defined
- [ ] Live data initialization working
- [ ] FFI live weather update functions available
- [ ] Sample integration with BOM data successful
- [ ] Error handling for missing/invalid data

### Phase 5 Complete When:
- [ ] Player action queue system implemented
- [ ] Action submission/retrieval FFI functions available
- [ ] Action history storage working (for late joiners)
- [ ] Deterministic replay validated (same actions = same fire state)
- [ ] State snapshot functions available
- [ ] At least 5 unit tests passing for action processing

### Phase 6 Complete When:
- [ ] All profiling benchmarks meet targets (60 FPS)
- [ ] Parallel processing optimizations applied
- [ ] Memory usage <2GB for full-scale simulation
- [ ] No clippy warnings
- [ ] All tests passing at release configuration

═══════════════════════════════════════════════════════════════════════
## NOTES FOR COPILOT CODING AGENT
═══════════════════════════════════════════════════════════════════════

**CRITICAL RULES**:
1. This is NOT a game - it's a scientific simulation for emergency training
2. NEVER simplify equations or formulas - use exact published formulas
3. Document ALL formulas with sources and units
4. Thread safety is MANDATORY for FFI (use Arc<Mutex<>>)
5. Game engine handles visualization and user interaction
6. Core simulation handles physics ONLY
7. Every formula needs a validation test
8. Performance target: 60 FPS with 600K+ fuel elements

**SIMULATION DOES NOT TRACK**:
- ❌ Player positions, health, or inventory
- ❌ Firefighter character states (energy, heat stress, injury)
- ❌ Equipment states (hose connections, pump operation)
- ❌ Vehicle positions or fuel levels
- ❌ Mission objectives or score
- ❌ UI state or camera positions
- ❌ Audio playback state
- ❌ Network connections or player IDs (only action player_id for logging)

**SIMULATION ONLY TRACKS**:
- ✅ Fuel element states (temperature, moisture, position, ignition)
- ✅ Fire spread and intensity
- ✅ Suppression coverage per fuel element
- ✅ Weather state (temperature, wind, humidity, pressure)
- ✅ Ember positions, velocities, and temperatures
- ✅ Atmospheric phenomena (pyrocumulus, fire tornadoes)
- ✅ Terrain data (elevation, slope, aspect)
- ✅ Action queue for multiplayer determinism

**WHEN STUCK**:
- Search scientific literature (Google Scholar)
- Check NFPA standards
- Reference USFS research papers
- Look at CSIRO bushfire research
- Ask for clarification if physics unclear

**COMPLETION RULE**:
Do NOT stop, pause, or ask for permission until the entire phase is FULLY implemented, tested, validated, and documented. Work continuously until all checkboxes are ticked and validation passes.

═══════════════════════════════════════════════════════════════════════

**END OF IMPLEMENTATION PLAN**
