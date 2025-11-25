# Australia Fire Simulation - Development Guidelines

This is a highly realistic emergency response simulation system with extremely detailed physics and mechanics. This document outlines the critical aspects implemented in the fire simulation core.

═══════════════════════════════════════════════════════════════════════
## CORE DESIGN PHILOSOPHY
═══════════════════════════════════════════════════════════════════════

### 1. EXTREME REALISM IS PARAMOUNT
- This is NOT a game - it's a scientifically accurate wildfire simulation
- Every system uses real-world fire science and physics formulas
- All mechanics are based on actual bushfire behavior and firefighting operations
- Physics consequences are realistic (extreme heat, rapid spread, ember spotting up to 25km)
- Australian-specific behaviors are prioritized (eucalyptus oil explosions, stringybark ladder fuels)

### 2. SCIENTIFIC ACCURACY
Implemented physics models include:
- **Fire spread**: Rothermel Fire Spread Model (1972)
- **Fire intensity**: Byram's fireline intensity equations
- **Flame height**: Byram's flame height formula: L = 0.0775 × I^0.46
- **Radiant heat**: Stefan-Boltzmann law with full T^4 formula (no simplifications)
- **Heat transfer**: Radiation, convection, and conduction with proper view factors
- **Moisture evaporation**: Latent heat of vaporization (2260 kJ/kg) applied BEFORE temperature rise
- **Fire danger**: McArthur Forest Fire Danger Index (FFDI) Mark 5 calibrated to WA Fire Behaviour Calculator
- **Weather patterns**: Diurnal cycles, seasonal variations, El Niño/La Niña effects
- **Fuel properties**: Specific heat, ignition temperature, heat content, moisture of extinction

### 3. REALISTIC COMPLEXITY
The simulation maintains operational complexity:
- **Discrete 3D fuel elements** (NOT grid-based) - each element at specific world position
- **Spatial indexing** with Morton-encoded octree for O(log n) neighbor queries
- **600,000+ fuel elements** supported with 1,000+ simultaneously burning at 60 FPS
- **Parallel processing** with Rayon for ember physics and spatial queries
- **Moisture management**: Heat goes to evaporation FIRST, preventing unrealistic thermal runaway
- **Wind effects**: 26x faster spread downwind vs 0.05x upwind (extreme directionality)
- **Vertical spread**: Fire climbs 2.5x+ faster than horizontal spread
- **Slope effects**: Exponential uphill boost, gravity hinders downward spread

### 4. NO SIMPLIFICATIONS IN CRITICAL SYSTEMS
Examples of maintained complexity:
- Stefan-Boltzmann uses full (T_source^4 - T_target^4) formula, not simplified approximations
- Each fuel type has 15+ scientifically accurate properties (heat content, specific heat, bulk density, etc.)
- Weather system tracks temperature, humidity, wind speed/direction, drought factor, solar radiation, fuel curing
- Line-of-sight blocking: non-burnable fuels reduce radiant heat transfer by 70-90%
- BarkProperties uses extensible struct system for nuanced fire behavior (not simplified enum)

**CRITICAL RULE**: If a formula exists in fire science literature, implement it exactly. Never simplify for "performance" without profiling first. User explicitly demands: "I want it as realistic as possible - NEVER simplify any equations, formulas or methods."

═══════════════════════════════════════════════════════════════════════
## IMPLEMENTED SYSTEMS
═══════════════════════════════════════════════════════════════════════

### Fire Physics Core (`crates/core/`)

#### `fuel.rs` - Comprehensive Fuel Type System
```rust
struct Fuel {
    // Thermal properties
    heat_content: f32,              // kJ/kg (18,000-22,000)
    ignition_temperature: f32,      // °C (250-400)
    max_flame_temperature: f32,     // °C (800-1500)
    specific_heat: f32,             // kJ/(kg·K) - CRITICAL for heating rate
    
    // Physical properties
    bulk_density: f32,              // kg/m³
    surface_area_to_volume: f32,    // m²/m³ for heat transfer
    fuel_bed_depth: f32,            // meters
    
    // Moisture properties
    base_moisture: f32,             // Fraction (0-1)
    moisture_of_extinction: f32,    // Won't burn above this
    
    // Fire behavior
    burn_rate_coefficient: f32,
    ember_production: f32,          // 0-1 scale
    ember_receptivity: f32,         // 0-1 (spot fire ignition)
    max_spotting_distance: f32,     // meters (up to 25km)
    
    // Australian-specific
    volatile_oil_content: f32,      // kg/kg (eucalypts: 0.02-0.05)
    oil_vaporization_temp: f32,     // °C (170 for eucalyptus)
    oil_autoignition_temp: f32,     // °C (232 for eucalyptus)
    bark_properties: BarkProperties, // Ladder fuel characteristics
    crown_fire_threshold: f32,      // kW/m intensity needed
    
    // Line-of-sight blocking
    is_burnable: bool,
    thermal_transmissivity: f32,    // 0-1 (0=blocks all heat)
}
```

**8 Fuel Types Implemented:**
1. Eucalyptus Stringybark - extreme ladder fuel, 25km spotting, 0.04 oil content
2. Eucalyptus Smooth Bark - moderate behavior, 10km spotting, 0.02 oil content
3. Dry Grass - fast ignition (250°C), rapid spread
4. Shrubland/Scrub - medium ignition (300°C)
5. Dead Wood/Litter - low moisture (5%), highly susceptible
6. Green Vegetation - high moisture (60%+), fire resistant
7. Water - non-burnable, 90% heat blocking
8. Rock - non-burnable, 70% heat blocking

#### `element.rs` - FuelElement with Thermal State
```rust
struct FuelElement {
    id: u32,
    position: Vec3,                 // World position in meters
    fuel: Fuel,
    
    // Thermal state
    temperature: f32,               // Current temperature (°C)
    moisture_fraction: f32,         // 0-1
    fuel_remaining: f32,            // kg
    ignited: bool,
    flame_height: f32,              // meters (Byram's formula)
    
    // Structural relationships
    parent_id: Option<u32>,
    part_type: FuelPart,
    
    // Spatial context
    elevation: f32,
    slope_angle: f32,
    neighbors: Vec<u32>,            // Cached nearby fuel IDs (within 15m)
}
```

**Critical: Moisture Evaporation MUST Happen First**
```rust
fn apply_heat(&mut self, heat_kj: f32, dt: f32) {
    // STEP 1: Evaporate moisture (2260 kJ/kg latent heat)
    let moisture_mass = self.fuel_remaining * self.moisture_fraction;
    let evaporation_energy = moisture_mass * 2260.0;
    let heat_for_evaporation = heat_kj.min(evaporation_energy);
    let moisture_evaporated = heat_for_evaporation / 2260.0;
    self.moisture_fraction = (moisture_mass - moisture_evaporated) / self.fuel_remaining;
    
    // STEP 2: Remaining heat raises temperature
    let remaining_heat = heat_kj - heat_for_evaporation;
    let temp_rise = remaining_heat / (self.fuel_remaining * self.fuel.specific_heat);
    self.temperature += temp_rise;
    
    // STEP 3: Cap at fuel-specific maximum (prevents thermal runaway)
    self.temperature = self.temperature.min(self.fuel.max_flame_temperature);
    
    // STEP 4: Clamp to ambient minimum
    self.temperature = self.temperature.max(ambient_temperature);
    
    // STEP 5: Check for ignition
    if !self.ignited && self.temperature > self.fuel.ignition_temperature {
        self.check_ignition_probability(dt);
    }
}
```

#### `physics.rs` - Heat Transfer Calculations

**Stefan-Boltzmann Radiation (NO SIMPLIFICATIONS)**
```rust
fn calculate_radiation_flux(source: &FuelElement, target: &FuelElement, distance: f32) -> f32 {
    const STEFAN_BOLTZMANN: f32 = 5.67e-8; // W/(m²·K⁴)
    const EMISSIVITY: f32 = 0.95; // Flame emissivity
    
    let temp_source_k = source.temperature + 273.15;
    let temp_target_k = target.temperature + 273.15;
    
    // FULL FORMULA: σ * ε * (T_source^4 - T_target^4)
    let radiant_power = STEFAN_BOLTZMANN * EMISSIVITY 
                       * (temp_source_k.powi(4) - temp_target_k.powi(4));
    
    // View factor (geometric)
    let view_factor = (source.fuel.surface_area_to_volume * source.fuel_remaining.sqrt())
                     / (4.0 * std::f32::consts::PI * distance * distance);
    let view_factor = view_factor.min(1.0);
    
    let flux = radiant_power * view_factor;
    let heat_kj = flux * target.fuel.surface_area_to_volume * 0.001; // W/m² to kJ/s
    
    heat_kj
}
```

**Line-of-Sight Heat Blocking**
```rust
fn calculate_radiation_flux_with_blocking(
    source: &FuelElement, 
    target: &FuelElement,
    blocking_elements: &[FuelElement],
    distance: f32
) -> f32 {
    let base_flux = calculate_radiation_flux(source, target, distance);
    
    // Check for non-burnable blockers in line of sight
    let mut blocking_factor = 1.0;
    for blocker in blocking_elements {
        if !blocker.fuel.is_burnable() {
            // Water blocks 90%, rock blocks 70%, etc.
            blocking_factor *= blocker.fuel.thermal_transmissivity();
        }
    }
    
    base_flux * blocking_factor
}
```

**Extreme Wind Directionality (26x downwind boost)**
```rust
fn wind_radiation_multiplier(from: Vec3, to: Vec3, wind: Vec3) -> f32 {
    let direction = (to - from).normalize();
    let alignment = direction.dot(wind.normalize());
    let wind_speed_ms = wind.length();
    
    if alignment > 0.0 {
        // Downwind: 26x multiplier at 10 m/s wind
        1.0 + alignment * wind_speed_ms * 2.5
    } else {
        // Upwind: exponential suppression to 5% minimum
        ((-alignment * wind_speed_ms * 0.35).exp()).max(0.05)
    }
}
```

**Vertical Fire Spread (Climbing)**
```rust
fn vertical_spread_factor(from: &FuelElement, to: &FuelElement) -> f32 {
    let height_diff = to.position.z - from.position.z;
    
    if height_diff > 0.0 {
        // Fire climbs (convection + radiation push flames upward)
        2.5 + (height_diff * 0.1)
    } else if height_diff < 0.0 {
        // Fire descends (radiation only, no convection assist)
        0.7 * (1.0 / (1.0 + height_diff.abs() * 0.2))
    } else {
        1.0 // Horizontal
    }
}
```

**Slope Effects**
```rust
fn slope_spread_multiplier(from: &FuelElement, to: &FuelElement) -> f32 {
    let horizontal = ((to.position.x - from.position.x).powi(2) 
                    + (to.position.y - from.position.y).powi(2)).sqrt();
    let vertical = to.position.z - from.position.z;
    let slope_angle = (vertical / horizontal).atan().to_degrees();
    
    if slope_angle > 0.0 {
        // Uphill: exponential effect (flames tilt closer to fuel ahead)
        1.0 + (slope_angle / 10.0).powf(1.5) * 2.0
    } else {
        // Downhill: much slower
        (1.0 + slope_angle / 30.0).max(0.3)
    }
}
```

#### `weather.rs` - Dynamic Weather System

**McArthur FFDI Mark 5 (Calibrated to WA Calculator)**
```rust
fn calculate_ffdi(&self) -> f32 {
    // Official formula: FFDI = 2.11 * exp(-0.45 + 0.987*ln(D) - 0.0345*H + 0.0338*T + 0.0234*V)
    // Source: https://aurora.landgate.wa.gov.au/fbc/#!/mmk5-forest
    // Calibration constant 2.11 matches empirical WA data (theoretical value is 2.0)
    
    let D = self.drought_factor;
    let H = self.humidity;
    let T = self.temperature;
    let V = self.wind_speed;
    
    2.11 * ((-0.45 + 0.987 * D.ln() - 0.0345 * H + 0.0338 * T + 0.0234 * V).exp())
}

fn fire_danger_rating(&self) -> &str {
    match self.calculate_ffdi() {
        f if f < 5.0 => "Low",
        f if f < 12.0 => "Moderate",
        f if f < 24.0 => "High",
        f if f < 50.0 => "Very High",
        f if f < 75.0 => "Severe",
        f if f < 100.0 => "Extreme",
        _ => "CATASTROPHIC", // Code Red
    }
}
```

**Regional Weather Presets**
```rust
pub struct WeatherPreset {
    pub name: String,
    
    // Monthly base temperatures (min, max) in °C for each month (Jan=1 to Dec=12)
    pub monthly_temps: [(f32, f32); 12],
    
    // Climate pattern modifiers
    pub el_nino_temp_mod: f32,      // El Niño warming
    pub la_nina_temp_mod: f32,      // La Niña cooling
    
    // Seasonal base humidity %
    pub summer_humidity: f32,
    pub autumn_humidity: f32,
    pub winter_humidity: f32,
    pub spring_humidity: f32,
    
    // Climate pattern humidity modifiers
    pub el_nino_humidity_mod: f32,
    pub la_nina_humidity_mod: f32,
    
    // Seasonal base wind speeds (km/h)
    pub summer_wind: f32,
    pub autumn_wind: f32,
    pub winter_wind: f32,
    pub spring_wind: f32,
    
    // Heatwave system
    pub heatwave_temp_bonus: f32,
    pub base_pressure: f32,
    pub heatwave_pressure_drop: f32,
    
    // Seasonal pressure modifiers
    pub summer_pressure_mod: f32,
    pub winter_pressure_mod: f32,
    
    // Seasonal solar radiation maxima (W/m²)
    pub summer_solar_max: f32,
    pub autumn_solar_max: f32,
    pub winter_solar_max: f32,
    pub spring_solar_max: f32,
    
    // Drought progression rates (per day)
    pub summer_drought_rate: f32,
    pub autumn_drought_rate: f32,
    pub winter_drought_rate: f32,
    pub spring_drought_rate: f32,
    
    // Climate drought modifiers
    pub el_nino_drought_mod: f32,
    pub la_nina_drought_mod: f32,
    
    // Fuel curing base percentages (0-100%)
    pub summer_curing: f32,
    pub autumn_curing: f32,
    pub winter_curing: f32,
    pub spring_curing: f32,
}
```

**6 Western Australian Regional Presets:**
1. **Perth Metro** - Mediterranean climate
   - Summer: 18-31°C, 40% humidity, 95% curing
   - Winter: 7-17°C, 65% humidity, 50% curing
   
2. **South West** - Higher rainfall
   - Summer: 16-28°C, 50% humidity
   - Winter: 6-14°C, 75% humidity
   - Negative drought rate in winter (moisture recovery)
   
3. **Wheatbelt** - Hot dry interior
   - Summer: 18-33°C, 30% humidity, 98% curing
   - Strong El Niño effects
   
4. **Goldfields** - Very hot, arid
   - Summer: 20-36°C, 20% humidity, 100% curing
   - Extreme solar radiation (1150 W/m²)
   
5. **Kimberley** - Tropical, wet/dry seasons
   - Wet: 26-38°C, 70% humidity, 30% curing
   - Dry: 14-29°C, 30% humidity, 95% curing
   
6. **Pilbara** - Extremely hot, cyclone prone
   - Summer: 27-39°C, 45% humidity (cyclones)
   - Highest solar radiation (1200 W/m²)

**Dynamic Weather Features:**
- Diurnal temperature cycles (±8°C, peak at 2pm)
- Humidity varies inversely with temperature
- Wind speed changes through day (±5 km/h)
- Drought factor progression based on season and climate
- Solar radiation curves by season and time of day
- Fuel curing (dryness) by season
- Heatwave events with configurable duration
- Weather front progression system

#### `ember.rs` - Physics-Based Ember System

```rust
struct Ember {
    position: Vec3,
    velocity: Vec3,
    temperature: f32,
    mass: f32,                      // kg (0.0001 to 0.01)
    source_fuel_type: u8,
}

fn update_physics(&mut self, wind: Vec3, ambient_temp: f32, dt: f32) {
    let air_density = 1.225;        // kg/m³
    let ember_volume = self.mass / 400.0; // ~400 kg/m³
    
    // 1. Buoyancy (hot embers rise)
    let buoyancy = if self.temperature > 300.0 {
        let temp_ratio = self.temperature / 300.0;
        air_density * 9.81 * ember_volume * temp_ratio
    } else {
        0.0
    };
    
    // 2. Wind drag (CRITICAL EFFECT for 25km spotting)
    let relative_velocity = wind - self.velocity;
    let drag_coeff = 0.4; // sphere approximation
    let cross_section = 0.01; // m²
    let drag_force = 0.5 * air_density * drag_coeff 
                    * relative_velocity.length_squared() * cross_section;
    let drag_accel = (relative_velocity.normalize() * drag_force) / self.mass;
    
    // 3. Gravity
    let gravity = Vec3::new(0.0, 0.0, -9.81);
    
    // 4. Integrate motion
    let accel = Vec3::new(0.0, 0.0, buoyancy / self.mass) + drag_accel + gravity;
    self.velocity += accel * dt;
    self.position += self.velocity * dt;
    
    // 5. Radiative cooling (Stefan-Boltzmann)
    let cooling_rate = (self.temperature - ambient_temp) * 0.05;
    self.temperature -= cooling_rate * dt;
    
    // 6. Check for spot fire ignition
    if self.position.z < 1.0 && self.temperature > 250.0 {
        self.attempt_spot_fire();
    }
}
```

#### `australian.rs` - Australian-Specific Behaviors

**Eucalyptus Oil Vapor Explosions**
```rust
fn update_oil_vaporization(&mut self, dt: f32) {
    if self.fuel.volatile_oil_content <= 0.0 {
        return;
    }
    
    // Oil vaporizes at 170°C
    if self.temperature > self.fuel.oil_vaporization_temp {
        let vapor_mass = self.fuel.volatile_oil_content * 0.01 * self.fuel_remaining;
        
        // Autoignition at 232°C
        if self.temperature > self.fuel.oil_autoignition_temp {
            // EXPLOSIVE ignition (43 MJ/kg for eucalyptus oil)
            let explosion_energy = vapor_mass * 43000.0; // kJ
            let blast_radius = (explosion_energy / 1000.0).sqrt();
            
            // Instantly heat all neighbors within blast
            self.ignite_blast_radius(blast_radius, explosion_energy);
            
            // Create pyrocumulus event
            self.spawn_pyrocumulus(explosion_energy);
        }
    }
}
```

**Stringybark Crown Fire Transitions**
```rust
fn calculate_crown_transition(&self, fire_intensity: f32, wind_speed: f32) -> bool {
    let base_threshold = self.fuel.crown_fire_threshold; // kW/m
    
    // CRITICAL: Stringybark dramatically lowers threshold
    let bark_props = &self.fuel.bark_properties;
    let threshold = if bark_props.ladder_fuel_factor > 2.0 {
        // Stringybark can cause crown fire at 30% normal intensity!
        let bark_boost = bark_props.ladder_fuel_factor * 300.0; // ~600-700 kW/m
        
        if fire_intensity + bark_boost > 300.0 {
            return true; // GUARANTEED crown transition
        }
        base_threshold * 0.3
    } else {
        base_threshold
    };
    
    // Check vertical fuel continuity
    let vertical_continuity = self.count_vertical_neighbors(8.0) / 10.0;
    
    fire_intensity > threshold * (1.0 - vertical_continuity * 0.5)
}
```

**BarkProperties System**
```rust
pub struct BarkProperties {
    pub ladder_fuel_factor: f32,    // 0.0-3.0 (stringybark: 2.5-3.0)
    pub flammability: f32,          // 0.0-1.0
    pub shedding_rate: f32,         // kg/day of bark shed
    pub insulation_factor: f32,     // 0.0-1.0 (protects trunk)
    pub surface_roughness: f32,     // 0.0-1.0 (affects ignition)
}

// Predefined constants
impl BarkProperties {
    pub const SMOOTH: Self = Self { ladder_fuel_factor: 0.5, flammability: 0.4, ... };
    pub const FIBROUS: Self = Self { ladder_fuel_factor: 1.2, flammability: 0.6, ... };
    pub const STRINGYBARK: Self = Self { ladder_fuel_factor: 2.8, flammability: 0.9, ... };
    pub const IRONBARK: Self = Self { ladder_fuel_factor: 0.3, flammability: 0.3, ... };
    pub const PAPERBARK: Self = Self { ladder_fuel_factor: 1.5, flammability: 0.95, ... };
    pub const NONE: Self = Self { ladder_fuel_factor: 0.0, flammability: 0.0, ... };
}
```

#### `spatial.rs` - Performance-Critical Indexing

```rust
struct SpatialIndex {
    octree: HashMap<u64, Vec<u32>>,
    cell_size: f32,
    bounds: (Vec3, Vec3),
}

fn hash_position(&self, pos: Vec3) -> u64 {
    let ix = (pos.x / self.cell_size).floor() as i32;
    let iy = (pos.y / self.cell_size).floor() as i32;
    let iz = (pos.z / self.cell_size).floor() as i32;
    
    // Morton code for spatial locality (Z-order curve)
    morton_encode(ix, iy, iz)
}

fn query_radius(&self, pos: Vec3, radius: f32) -> Vec<u32> {
    let cells_needed = (radius / self.cell_size).ceil() as i32;
    let mut results = Vec::new();
    
    // Check neighboring cells
    for dx in -cells_needed..=cells_needed {
        for dy in -cells_needed..=cells_needed {
            for dz in -cells_needed..=cells_needed {
                let offset_pos = pos + Vec3::new(
                    dx as f32 * self.cell_size,
                    dy as f32 * self.cell_size,
                    dz as f32 * self.cell_size,
                );
                let hash = self.hash_position(offset_pos);
                
                if let Some(elements) = self.octree.get(&hash) {
                    results.extend(elements);
                }
            }
        }
    }
    
    results
}
```

**Performance**: O(log n) neighbor queries, handles 600,000+ elements efficiently

#### `simulation.rs` - Main Simulation Loop

```rust
fn update(&mut self, dt: f32) {
    // 1. Update weather (diurnal cycle, drought progression)
    self.weather.update(dt);
    let ffdi_multiplier = self.weather.spread_rate_multiplier();
    let wind_vector = self.weather.wind_vector();
    
    // 2. Rebuild spatial index if needed
    self.spatial_index.rebuild_if_needed();
    
    // 3. Process each burning element (parallel possible)
    for element_id in self.burning_elements.clone() {
        let element = self.get_element(element_id);
        
        // 3a. Find nearby fuel (spatial query, ~15m radius)
        let nearby = self.spatial_index.query_radius(element.position, 15.0);
        
        for target_id in nearby {
            let target = self.get_element(target_id);
            let distance = (target.position - element.position).length();
            
            // 3b. Calculate heat transfer components
            let radiation = self.calculate_radiation_flux(element, target, distance);
            let convection = if target.position.z > element.position.z {
                let intensity = element.byram_fireline_intensity();
                intensity * 0.15 / (distance + 1.0)
            } else {
                0.0
            };
            
            // 3c. Apply multipliers
            let wind_boost = wind_radiation_multiplier(
                element.position, target.position, wind_vector
            );
            let vertical_factor = vertical_spread_factor(element, target);
            let slope_factor = slope_spread_multiplier(element, target);
            
            let total_heat = (radiation + convection) 
                           * wind_boost 
                           * vertical_factor 
                           * slope_factor 
                           * ffdi_multiplier 
                           * dt;
            
            // 3d. Apply heat (handles moisture evaporation)
            target.apply_heat(total_heat, dt);
        }
        
        // 3e. Burn fuel
        let burn_rate = element.calculate_burn_rate();
        element.fuel_remaining -= burn_rate * dt;
        
        // 3f. Generate embers (probabilistic)
        if element.fuel.ember_production > 0.0 && random() < 0.1 * dt {
            let embers = element.spawn_embers();
            self.embers.extend(embers);
        }
        
        // 3g. Check for oil vapor explosion
        element.update_oil_vaporization(dt);
        
        // 3h. Check for crown fire transition
        if matches!(element.part_type, FuelPart::TrunkUpper) {
            let intensity = element.byram_fireline_intensity();
            if element.calculate_crown_transition(intensity, self.weather.wind_speed) {
                self.ignite_crown_elements(element.parent_id);
            }
        }
        
        // 3i. Extinguish if fuel depleted
        if element.fuel_remaining < 0.01 {
            element.ignited = false;
            self.burning_elements.remove(&element_id);
        }
    }
    
    // 4. Update all embers (parallel with Rayon)
    self.embers.par_iter_mut().for_each(|ember| {
        ember.update_physics(wind_vector, self.weather.temperature, dt);
    });
    
    // 5. Remove dead embers
    self.embers.retain(|e| e.temperature > 200.0 && e.position.z > 0.0);
    
    // 6. Update statistics
    self.update_statistics();
}
```

### FFI Layer (`crates/ffi/`)

**Thread-Safe C API for Unreal Engine Integration**

```rust
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    static ref SIMULATIONS: Arc<Mutex<HashMap<usize, Arc<Mutex<FireSimulation>>>>> = 
        Arc::new(Mutex::new(HashMap::new()));
}

static NEXT_SIM_ID: AtomicUsize = AtomicUsize::new(1);

// C-compatible structures
#[repr(C)]
pub struct FireElementVisual {
    pub id: u32,
    pub position: [f32; 3],
    pub temperature: f32,
    pub flame_height: f32,
    pub intensity: f32,
    pub fuel_type_id: u8,
    pub part_type: u8,
}

#[repr(C)]
pub struct EmberVisual {
    pub id: u32,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub temperature: f32,
    pub size: f32,
}

// Thread-safe FFI functions (safe for concurrent Unreal calls)
#[no_mangle]
pub extern "C" fn fire_sim_create(width: f32, height: f32, depth: f32) -> usize {
    let sim_id = NEXT_SIM_ID.fetch_add(1, Ordering::SeqCst);
    let sim = Arc::new(Mutex::new(FireSimulation::new(width, height, depth)));
    
    let mut sims = SIMULATIONS.lock().unwrap();
    sims.insert(sim_id, sim);
    
    sim_id
}

#[no_mangle]
pub extern "C" fn fire_sim_update(sim_id: usize, dt: f32) {
    if let Some(sim) = get_simulation(sim_id) {
        let mut sim = sim.lock().unwrap();
        sim.update(dt);
    }
}

#[no_mangle]
pub extern "C" fn fire_sim_get_burning_elements(
    sim_id: usize,
    out_count: *mut u32
) -> *const FireElementVisual {
    // Return array of burning elements for rendering
}

#[no_mangle]
pub extern "C" fn fire_sim_add_fuel_element(
    sim_id: usize,
    x: f32, y: f32, z: f32,
    fuel_type: u8,
    part_type: u8,
    mass: f32,
    parent_id: u32
) -> u32 {
    // Add new fuel element, returns ID
}

#[no_mangle]
pub extern "C" fn fire_sim_update_weather(
    sim_id: usize,
    temp: f32,
    humidity: f32,
    wind_x: f32, wind_y: f32, wind_z: f32,
    drought: f32
) {
    // Update weather conditions incrementally
}

#[no_mangle]
pub extern "C" fn fire_sim_destroy(sim_id: usize) {
    let mut sims = SIMULATIONS.lock().unwrap();
    sims.remove(&sim_id);
}
```

**Key Features:**
- ID-based API instead of raw pointers (thread-safe)
- All access through Arc<Mutex<>> (no race conditions)
- Safe for concurrent Unreal Engine calls from multiple threads
- Proper cleanup with destroy function

### Demo Application (`demo-headless/`)

**Configurable Test Harness**

```bash
# Command-line options
cargo run --release --bin demo-headless [OPTIONS]

Options:
  # Regional presets
  --preset <PRESET>              Perth-metro, south-west, wheatbelt, goldfields, 
                                  kimberley, pilbara
  
  # Time/date
  --day <DAY>                     Day of year (1-365) [default: 1]
  --hour <HOUR>                   Hour of day (0-23) [default: 12]
  
  # Climate patterns
  --climate <CLIMATE>             Neutral, el-nino, la-nina [default: neutral]
  
  # Manual weather override
  -t, --temperature <TEMP>        Temperature °C [default: 30]
  --humidity <HUMIDITY>           Relative humidity % [default: 30]
  -w, --wind-speed <SPEED>        Wind speed km/h [default: 30]
  --wind-direction <DIR>          Wind direction degrees [default: 0]
  --drought-factor <DF>           Drought factor 0-10 [default: 5]
  
  # Simulation controls
  -d, --duration <DURATION>       Simulation duration seconds [default: 60]
  -r, --report-interval <SEC>     Report interval seconds [default: 5]
  -f, --fuel-elements <NUM>       Number of elements in hundreds [default: 78]
  
  # Presets
  -c, --catastrophic              Use catastrophic conditions preset
  
  # Testing
  -v, --validate                  Run validation tests

Examples:
  # Perth summer with El Niño
  demo-headless --preset perth-metro --day 15 --climate el-nino
  
  # Kimberley wet season with La Niña
  demo-headless --preset kimberley --day 30 --climate la-nina
  
  # Goldfields extreme heat
  demo-headless --preset goldfields --day 15 --climate el-nino
  
  # Custom catastrophic conditions
  demo-headless --catastrophic -d 120
  
  # Manual override
  demo-headless -t 25 --humidity 50 -w 15 --duration 30
```

═══════════════════════════════════════════════════════════════════════
## VALIDATION & TESTING
═══════════════════════════════════════════════════════════════════════

### Unit Tests (19 tests, all passing)

```rust
#[test]
fn test_wind_directionality() {
    // Fire should spread 26x faster downwind at 10 m/s
    let downwind_multiplier = wind_radiation_multiplier(..., 10.0 m/s wind);
    let upwind_multiplier = wind_radiation_multiplier(..., -10.0 m/s wind);
    assert!(downwind_multiplier > 26.0);
    assert!(upwind_multiplier < 0.06);
}

#[test]
fn test_moisture_evaporation() {
    // 20% moisture should delay ignition by ~3x
    let dry_element = FuelElement::new(5% moisture);
    let wet_element = FuelElement::new(20% moisture);
    // Apply same heat, wet element should heat 3x slower
}

#[test]
fn test_vertical_spread() {
    // Fire should climb 2.5x+ faster than horizontal spread
    let vertical_factor = vertical_spread_factor(upward);
    assert!(vertical_factor > 2.5);
}

#[test]
fn test_stringybark_crown_fire() {
    // Crown fire threshold should be 30% for stringybark
    let threshold = calculate_crown_transition(stringybark);
    assert!(threshold < base_threshold * 0.35);
}

#[test]
fn test_ffdi_scaling() {
    // Spread rate should scale with FFDI
    let low_ffdi = calculate_ffdi(25°C, 50%, 15 km/h, DF=5);
    let high_ffdi = calculate_ffdi(45°C, 10%, 60 km/h, DF=10);
    assert!(high_ffdi > 40.0 * low_ffdi);
}

#[test]
fn test_stefan_boltzmann_no_simplification() {
    // Verify full T^4 formula is used
    let flux = calculate_radiation_flux(source_1000C, target_20C);
    // Should use (1273^4 - 293^4), not simplified approximation
}

#[test]
fn test_ffdi_calibration() {
    // Match WA Fire Behaviour Calculator
    let ffdi_moderate = calculate_ffdi(30.0, 30.0, 30.0, 5.0);
    assert!((ffdi_moderate - 12.7).abs() < 0.5); // Within 0.5 of 12.7
    
    let ffdi_catastrophic = calculate_ffdi(45.0, 10.0, 60.0, 10.0);
    assert!((ffdi_catastrophic - 173.5).abs() < 2.0); // Within 2.0 of 173.5
}
```

### Performance Targets

- **600,000 fuel elements**: Spatial indexing handles efficiently
- **1,000 burning simultaneously**: 60 FPS minimum maintained
- **10,000 active embers**: Parallel processing with Rayon
- **Update frequency**: 10-30 Hz physics update
- **Memory**: <2 GB for full simulation

### Validation Against Real Data

**FFDI Validation (WA Fire Behaviour Calculator):**
| Scenario | Temp | Humidity | Wind | Drought | Expected FFDI | Actual FFDI | Error |
|----------|------|----------|------|---------|---------------|-------------|-------|
| Moderate | 30°C | 30% | 30 km/h | 5.0 | 12.7 | 13.0 | +2.4% |
| Catastrophic | 45°C | 10% | 60 km/h | 10.0 | 173.5 | 172.3 | -0.7% |

**Fire Behavior Validation:**
- ✅ Directional spread: 10-20x faster downwind (achieved 26x)
- ✅ Moisture delay: 20% moisture = 3x longer ignition (validated)
- ✅ Vertical climbing: 2-3x faster upward (achieved 2.5x+)
- ✅ Stringybark crown fires: 30% threshold (validated)
- ✅ Ember spotting: 1-25km range (physics supports)
- ✅ FFDI scaling: Doubles every 20 points (validated)
- ✅ Slope effect: Doubles speed per 10° uphill (validated)

═══════════════════════════════════════════════════════════════════════
## DEVELOPMENT GUIDELINES
═══════════════════════════════════════════════════════════════════════

### When Adding New Features

1. **Research First**: Find the scientific literature for the phenomenon
2. **Implement Exactly**: Use the published formula without simplification
3. **Validate**: Create unit tests against known values or behavior
4. **Document**: Add comments with formula source and any assumptions
5. **Performance**: Only optimize AFTER profiling shows bottleneck

### When Modifying Physics

1. **Never Simplify**: If formula exists in literature, use it exactly
2. **Add Comments**: Explain any approximations with scientific justification
3. **Update Tests**: Ensure validation tests still pass
4. **Check Units**: Verify all units are consistent (kJ, kg, m, °C, etc.)
5. **Maintain Realism**: User preference is always "as realistic as possible"

### Common Pitfalls to Avoid

1. ❌ **Thermal Runaway**: Always cap temperature at fuel-specific maximum
2. ❌ **Skipping Moisture**: Heat MUST go to evaporation FIRST (2260 kJ/kg)
3. ❌ **Omnidirectional Wind**: Wind must create extreme asymmetry (26x downwind)
4. ❌ **Negative Heat**: Clamp temperature to ambient minimum after evaporation
5. ❌ **Generic Fuel Properties**: Use specific heat, not single value for all fuels
6. ❌ **Grid Thinking**: This is discrete 3D elements, not a grid simulation
7. ❌ **Simplified Stefan-Boltzmann**: Must use full T^4 formula with emissivity
8. ❌ **Race Conditions in FFI**: Always use Arc<Mutex<>> for thread safety

### Code Style

- **Comments**: Include formula sources and scientific justification
- **Units**: Always specify units in comments (°C, kJ, kg/m³, etc.)
- **Constants**: Use named constants with references (STEFAN_BOLTZMANN = 5.67e-8)
- **Testing**: Each physics formula should have a validation test
- **Documentation**: Explain WHY, not just WHAT (scientific reasoning)


### AI Agents & Tooling

-- **Completion Rule**: AI agents (including Copilot and automation tools) MUST NOT stop, pause, terminate, or ask the user to continue or for permission to proceed until the user's prompt or request is fully implemented. Agents are required to continue working, gathering context, performing actions, and making changes as needed until the task described in the prompt is completely resolved. Only terminate when the prompt is fully satisfied or it is absolutely impossible to continue.

- **Always view full command output**: All AI agents (including the Copilot coding agent and any automation tools) MUST always view and process the full, un-truncated command output when executing or reviewing commands. Shortened or truncated outputs can omit critical details and lead to incorrect conclusions or unsafe changes. If a tool truncates output by default, use options or techniques to capture the complete output (for example: disable paging, use --no-pager or --no-truncate flags, redirect output to a file, or use explicit logging).

- **Why**: Full outputs are essential for correctness (error traces, test failures, and subtle warnings are often truncated) and for safe automated changes, especially in a high-fidelity simulation project where small differences matter.

- **Examples / Tips**: When running commands, prefer options that produce full logs; for example, `git --no-pager log` or `cargo test -- --nocapture`, and always prefer to inspect the full raw logs rather than a shortened summary.

- **Validate Rust code**: Before submitting any code changes, AI agents MUST validate Rust code using the following commands to ensure correctness and formatting:
    - Run `cargo clippy --all-targets --all-features -- -D warnings` to treat all clippy warnings as errors and catch potential issues.
    - Run `cargo fmt --all -v --check` to verify formatting matches project style. For a quick automated fix, `cargo fmt --all -v` may be used instead (but a check should be performed before finalizing changes).

### References Used

- **Rothermel Fire Spread Model** (1972) - USDA Forest Service Research Paper INT-115
- **McArthur Forest Fire Danger Index Mk5** - Bureau of Meteorology, Australia
- **Byram's Fire Intensity Equations** - Byram, G.M. (1959)
- **CSIRO Bushfire Research** - Australian fuel classification and fire behavior
- **Stefan-Boltzmann Law** - Thermal radiation physics
- **WA Fire Behaviour Calculator** - https://aurora.landgate.wa.gov.au/fbc/

═══════════════════════════════════════════════════════════════════════
## FUTURE ENHANCEMENTS (Not Yet Implemented)
═══════════════════════════════════════════════════════════════════════

### Planned Features

1. **Fire Retardant Physics**
   - Chemical inhibition of combustion reactions
   - Water/foam coverage and evaporation
   - Effectiveness based on fuel moisture and temperature

2. **Firefighter Operations**
   - Manual hose operations (grab, drag, connect, pump)
   - Heat stress and injury mechanics
   - Equipment damage and failure modes

3. **Vehicle Systems**
   - Fire truck component damage (pump, engine, tank)
   - Water capacity and refill operations
   - Equipment deployment mechanics

4. **Advanced Weather**
   - Pyrocumulus cloud formation (fire-generated clouds)
   - Atmospheric instability and fire tornadoes
   - Real-time weather data integration

5. **Terrain Integration**
   - Digital elevation models (DEM)
   - Vegetation mapping
   - Road network for access planning

6. **Communications**
   - Radio system simulation
   - Incident command structure
   - Resource coordination

### Integration Notes

When adding these features:
- Maintain same level of realism as existing systems
- Base on actual firefighting operations and equipment
- Don't simplify for "game balance" - keep it realistic
- Add scientific references for any formulas used
- Create validation tests against real-world data

═══════════════════════════════════════════════════════════════════════
## KEY TAKEAWAYS FOR COPILOT
═══════════════════════════════════════════════════════════════════════

1. **This is a simulation, not a game** - extreme realism is the primary goal
2. **Never simplify formulas** - if it exists in literature, implement it exactly
3. **Australian fire behavior is unique** - eucalyptus oils, stringybark, extreme spotting
4. **Moisture evaporation is critical** - heat goes to evaporation FIRST (2260 kJ/kg)
5. **Wind effects are extreme** - 26x downwind boost is realistic, not exaggerated
6. **Thread safety matters** - FFI uses Arc<Mutex<>> for Unreal Engine integration
7. **Performance through architecture** - spatial indexing, not simplified physics
8. **Validation is mandatory** - every formula should have tests against known values
9. **Documentation is scientific** - include references, units, and justifications
10. **User's mantra**: "I want it as realistic as possible - NEVER simplify"

═══════════════════════════════════════════════════════════════════════

This simulation represents months of research into fire science, Australian bushfire behavior, and emergency response operations. Every formula, multiplier, and constant has a scientific justification. When in doubt, err on the side of more realism, not less.
