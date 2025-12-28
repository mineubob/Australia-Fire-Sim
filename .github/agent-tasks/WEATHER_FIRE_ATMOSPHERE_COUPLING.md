# Weather System Enhancements: Fire-Atmosphere Coupling

**Status:** Not Started  
**Priority:** Medium  
**Depends On:** [REALISTIC_GPU_FIRE_FRONT_SYSTEM.md](./REALISTIC_GPU_FIRE_FRONT_SYSTEM.md) (Phase 5+)  
**Target:** Enhance weather/wind system for more realistic fire-atmosphere interactions

---

## Overview

This task enhances the existing weather and wind systems to provide better fire-atmosphere coupling. These are **optional improvements** that build on the core GPU fire system.

**Prerequisites:** The GPU Fire System (Phases 1-5) must be complete before starting this task.

**Current Weather System Status: ✅ Comprehensive**

The existing weather system already includes:
- FFDI calculation (McArthur Mk5)
- Diurnal temperature/humidity cycles
- Seasonal variation with 12-month tables
- El Niño/La Niña climate effects
- Regional presets (6 WA regions + catastrophic)
- Mass-consistent wind field (Sherman 1978)
- Terrain effects on wind (slope, aspect, blocking)
- Plume coupling (Byram convection column)
- Atmospheric stability (Pasquill-Gifford classes)
- Atmospheric turbulence scaling
- Solar radiation (time of day, season)

This task focuses on **enhancements** to make fire-weather interaction more dynamic.

---

## Phases Overview

| Phase | Name | Complexity | Est. Days |
|-------|------|------------|-----------|
| **W1** | Fire-Induced Wind Feedback | High | 3-4 |
| **W2** | Dynamic Wind Variability | Medium | 2-3 |
| **W3** | Pressure System Dynamics | Medium | 2-3 |
| **W4** | Fire Whirl Detection | High | 3-4 |

**Total Estimated Time: 10-14 days**

---

## PHASE W1: Fire-Induced Wind Feedback

**Objective:** Make the wind field respond dynamically to fire intensity, creating the "wind rushing toward fire" effect.

**Branch Name:** `feature/weather-phase1-fire-wind-feedback`

**Estimated Time:** 3-4 days

### Scientific Basis

Large fires create their own local wind patterns through entrainment:
- Air is drawn toward the fire base (entrainment)
- Hot air rises in the convection column
- This creates surface winds converging on the fire

**Entrainment velocity (Beer 1991):**
```
v_entrain = 0.1 × (I / (ρ × cp × ΔT))^(1/3)
```
Where:
- I = local fire intensity (kW/m)
- ρ = air density (1.225 kg/m³)
- cp = specific heat of air (1005 J/(kg·K))
- ΔT = temperature difference (fire - ambient)

### Deliverables

- [ ] Add fire intensity field input to `WindField::update()`
- [ ] Calculate entrainment velocity toward fire front
- [ ] Blend entrainment with background wind
- [ ] Update wind field at fire-atmosphere coupling frequency

### Files to Modify

```
crates/core/src/grid/wind_field.rs
├── Add `update_with_fire_intensity()` method
├── Calculate entrainment wind vectors
└── Blend with existing mass-consistent solution

crates/core/src/simulation/mod.rs
└── Pass fire intensity field to wind update
```

### Implementation

```rust
impl WindField {
    /// Update wind field with fire-induced entrainment
    pub fn update_with_fire_intensity(
        &mut self,
        base_wind: Vec3,
        terrain: &TerrainData,
        plumes: &[PlameSource],
        fire_intensity: &[f32],  // NEW: intensity at each grid cell
        intensity_width: usize,
        intensity_height: usize,
        dt: f32,
    ) {
        // First: standard mass-consistent update
        self.update(base_wind, terrain, plumes, dt);
        
        // Second: add entrainment toward high-intensity cells
        const AIR_DENSITY: f32 = 1.225;
        const CP_AIR: f32 = 1005.0;
        const DELTA_T: f32 = 500.0;  // Typical fire-ambient difference
        
        // For each cell, calculate entrainment from nearby fire
        for iy in 0..self.config.ny {
            for ix in 0..self.config.nx {
                let world_x = ix as f32 * self.config.cell_size;
                let world_y = iy as f32 * self.config.cell_size;
                
                // Find nearest high-intensity fire cell
                let (fire_dir, fire_intensity) = self.find_fire_influence(
                    world_x, world_y, 
                    fire_intensity, intensity_width, intensity_height
                );
                
                if fire_intensity > 1000.0 {  // >1 MW/m threshold
                    // Beer (1991) entrainment formula
                    let v_entrain = 0.1 * (fire_intensity / (AIR_DENSITY * CP_AIR * DELTA_T)).powf(1.0/3.0);
                    
                    // Add entrainment wind toward fire
                    let entrain_wind = fire_dir * v_entrain;
                    
                    // Blend with existing wind (entrainment dominates near fire)
                    let distance = fire_dir.magnitude() * self.config.cell_size;
                    let blend = (1.0 - distance / 500.0).max(0.0);  // Fade over 500m
                    
                    for iz in 0..self.config.nz.min(3) {  // Surface layers only
                        let idx = self.index(ix, iy, iz);
                        self.wind[idx] += entrain_wind * blend;
                    }
                }
            }
        }
    }
}
```

### Validation Criteria

- [ ] Wind vectors point toward active fire front
- [ ] Entrainment velocity scales with fire intensity^(1/3)
- [ ] Effect diminishes with distance from fire
- [ ] No effect when fire intensity is low
- [ ] Existing mass-consistent behavior preserved for non-fire areas
- [ ] `cargo clippy --all-targets --all-features` passes

---

## PHASE W2: Dynamic Wind Variability

**Objective:** Add temporal and spatial variation to wind direction and speed.

**Branch Name:** `feature/weather-phase2-wind-variability`

**Estimated Time:** 2-3 days

### Scientific Basis

Real wind is never steady - it varies in:
- **Speed:** Gusts and lulls
- **Direction:** Slow oscillation (meandering) plus turbulent variation
- **Time:** Diurnal patterns (sea breezes, drainage flows)

### Deliverables

- [ ] Wind direction noise (±15-30° variation over minutes)
- [ ] Wind speed gusts (factor 1.3-2.0× background)
- [ ] Gust probability increases with FFDI
- [ ] Smooth temporal transitions (no sudden jumps)

### Files to Modify

```
crates/core/src/core_types/weather.rs
├── Add `wind_variability` parameters to WeatherPreset
└── Add `apply_wind_variation()` method

crates/core/src/simulation/mod.rs
└── Apply variation before passing to wind field
```

### Implementation

```rust
/// Wind variability parameters
pub struct WindVariability {
    /// Direction variation amplitude (degrees)
    pub direction_amplitude: Degrees,
    /// Direction variation period (seconds)
    pub direction_period: f32,
    /// Gust factor (multiplier on base speed)
    pub gust_factor: f32,
    /// Probability of gust per second
    pub gust_probability: f32,
    /// Current gust state
    gust_remaining: f32,
}

impl Weather {
    /// Apply temporal variation to wind
    pub fn apply_wind_variation(&mut self, dt: f32, time: f32) {
        // Slow direction meandering (period ~5-10 minutes)
        let meander = (time / self.variability.direction_period * std::f32::consts::TAU).sin()
            * *self.variability.direction_amplitude;
        
        // Random gust check
        if rand::random::<f32>() < self.variability.gust_probability * dt {
            self.variability.gust_remaining = 5.0 + rand::random::<f32>() * 10.0;  // 5-15 second gusts
        }
        
        // Apply gust if active
        let speed_factor = if self.variability.gust_remaining > 0.0 {
            self.variability.gust_remaining -= dt;
            self.variability.gust_factor
        } else {
            1.0
        };
        
        // Update effective wind
        self.effective_wind_direction = self.wind_direction + meander;
        self.effective_wind_speed = self.wind_speed * speed_factor;
    }
}
```

### Validation Criteria

- [ ] Wind direction varies smoothly over time
- [ ] Gusts occur stochastically with correct probability
- [ ] Higher FFDI = more frequent/stronger gusts
- [ ] No discontinuities in wind field
- [ ] `cargo clippy --all-targets --all-features` passes

---

## PHASE W3: Pressure System Dynamics

**Objective:** Model approaching weather systems that dramatically change fire behavior.

**Branch Name:** `feature/weather-phase3-pressure-systems`

**Estimated Time:** 2-3 days

### Scientific Basis

Major fire events often occur with:
- **Approaching cold fronts:** Wind direction shifts 90-180°, gusts increase
- **Pre-frontal conditions:** Hot, dry, strong N/NW winds (in Australia)
- **Post-frontal conditions:** Cooler, windier, direction change

Black Saturday (2009) saw a wind change from N to SW that caused the fire to turn 90°.

### Deliverables

- [ ] `PressureSystem` struct (position, intensity, movement)
- [ ] Front passage timing and effects
- [ ] Pre-frontal temperature/humidity spike
- [ ] Wind direction change over 30-60 minutes
- [ ] Post-frontal temperature drop

### Files to Create

```
crates/core/src/weather/
├── pressure_system.rs   # PressureSystem struct and dynamics
└── mod.rs               # Update to include pressure systems
```

### Implementation

```rust
/// A pressure system (cold front, trough) affecting weather
#[derive(Clone, Debug)]
pub struct PressureSystem {
    /// Type of system
    pub system_type: PressureSystemType,
    /// Current position (km from simulation origin)
    pub position: Vec2,
    /// Movement velocity (km/h)
    pub velocity: Vec2,
    /// Intensity factor (0.5 = weak, 1.0 = moderate, 2.0 = strong)
    pub intensity: f32,
    /// Width of transition zone (km)
    pub front_width: f32,
}

pub enum PressureSystemType {
    ColdFront,
    SeaBreeze,
    Trough,
}

impl PressureSystem {
    /// Calculate effect on weather at given position
    pub fn effect_at(&self, world_pos: Vec2, time: f32) -> PressureEffect {
        let front_pos = self.position + self.velocity * (time / 3600.0);  // km after time seconds
        let distance_to_front = (world_pos - front_pos).dot(self.velocity.normalize());
        
        // Normalized position: -1 = far behind, 0 = at front, +1 = far ahead
        let normalized = (distance_to_front / self.front_width).clamp(-1.0, 1.0);
        
        match self.system_type {
            PressureSystemType::ColdFront => PressureEffect {
                // Pre-frontal: hot, dry, N/NW wind
                // Post-frontal: cool, gusty, S/SW wind
                temp_modifier: Celsius::new(-10.0 * (1.0 - normalized) * self.intensity),
                humidity_modifier: Percent::new(20.0 * (1.0 - normalized) * self.intensity),
                wind_direction_shift: Degrees::new(90.0 * (1.0 - normalized) * self.intensity),
                wind_speed_factor: 1.0 + 0.5 * (1.0 - normalized.abs()) * self.intensity,
            },
            // ... other types
        }
    }
}
```

### Validation Criteria

- [ ] Front passage creates gradual wind shift
- [ ] Pre-frontal conditions are hotter and drier
- [ ] Post-frontal conditions are cooler with direction change
- [ ] Front movement is continuous
- [ ] `cargo clippy --all-targets --all-features` passes

---

## PHASE W4: Fire Whirl Detection

**Objective:** Detect conditions favorable for fire whirls and enhance ember lofting.

**Branch Name:** `feature/weather-phase4-fire-whirls`

**Estimated Time:** 3-4 days

### Scientific Basis

Fire whirls form when:
- Strong convective updraft (high intensity fire)
- Background vorticity (wind shear, terrain features)
- Atmospheric instability

Fire whirls can:
- Loft embers to extreme heights (>1000m)
- Create spotting distances >10km
- Generate 100+ km/h winds at ground level

### Deliverables

- [ ] Vorticity calculation in wind field
- [ ] Fire whirl probability based on intensity + vorticity + instability
- [ ] Enhanced ember lofting height during whirl events
- [ ] Visual/audio flag for fire whirl conditions

### Files to Modify

```
crates/core/src/grid/wind_field.rs
├── Add `calculate_vorticity()` method
└── Add `fire_whirl_probability()` method

crates/core/src/physics/albini_spotting.rs
└── Multiply lofting height during whirl conditions
```

### Implementation

```rust
impl WindField {
    /// Calculate vertical vorticity at a position
    pub fn calculate_vorticity(&self, x: f32, y: f32) -> f32 {
        // ω_z = ∂v/∂x - ∂u/∂y
        let dx = self.config.cell_size;
        
        let u_north = self.wind_at_position(Vec3::new(x, y + dx, 0.0)).x;
        let u_south = self.wind_at_position(Vec3::new(x, y - dx, 0.0)).x;
        let v_east = self.wind_at_position(Vec3::new(x + dx, y, 0.0)).y;
        let v_west = self.wind_at_position(Vec3::new(x - dx, y, 0.0)).y;
        
        let dv_dx = (v_east - v_west) / (2.0 * dx);
        let du_dy = (u_north - u_south) / (2.0 * dx);
        
        dv_dx - du_dy
    }
    
    /// Estimate fire whirl probability
    pub fn fire_whirl_probability(
        &self,
        position: Vec3,
        fire_intensity: f32,  // kW/m
        atmospheric_stability: f32,  // Lifted Index
    ) -> f32 {
        let vorticity = self.calculate_vorticity(position.x, position.y).abs();
        
        // Conditions for fire whirl:
        // 1. High vorticity (>0.01 s⁻¹)
        // 2. High fire intensity (>10 MW/m)
        // 3. Unstable atmosphere (LI < 0)
        
        let vorticity_factor = (vorticity / 0.01).min(1.0);
        let intensity_factor = (fire_intensity / 10000.0).min(1.0);
        let stability_factor = if atmospheric_stability < 0.0 {
            (-atmospheric_stability / 3.0).min(1.0)
        } else {
            0.0
        };
        
        // Combined probability (all factors must be present)
        vorticity_factor * intensity_factor * stability_factor * 0.5
    }
}
```

### Validation Criteria

- [ ] Vorticity calculation produces physically reasonable values
- [ ] Fire whirl probability increases with intensity and vorticity
- [ ] Stable atmosphere suppresses fire whirl formation
- [ ] Ember lofting height increases during whirl conditions
- [ ] `cargo clippy --all-targets --all-features` passes

---

## References

1. **Beer, T. (1991)**. "The interaction of wind and fire." Boundary-Layer Meteorology, 54, 287-313.
2. **Clark, T.L. et al. (1996)**. "Coupled atmosphere-fire model." Int. J. Wildland Fire, 6(2), 55-68.
3. **Coen, J.L. (2005)**. "Simulation of the Big Elk Fire using coupled atmosphere-fire modeling."
4. **Forthofer, J.M., Goodrick, S.L. (2011)**. "Review of vortices in wildland fire." J. Combustion.
5. **Sharples, J.J. et al. (2012)**. "An overview of mountain meteorological effects relevant to fire behavior."

---

## Completion Checklist

- [ ] **Phase W1:** Fire-Induced Wind Feedback
  - Branch: `feature/weather-phase1-fire-wind-feedback`
  - PR: #___

- [ ] **Phase W2:** Dynamic Wind Variability  
  - Branch: `feature/weather-phase2-wind-variability`
  - PR: #___

- [ ] **Phase W3:** Pressure System Dynamics
  - Branch: `feature/weather-phase3-pressure-systems`
  - PR: #___

- [ ] **Phase W4:** Fire Whirl Detection
  - Branch: `feature/weather-phase4-fire-whirls`
  - PR: #___

### Final Verification

- [ ] All phases complete
- [ ] All tests pass
- [ ] `cargo clippy --all-targets --all-features` passes
- [ ] Documentation updated

**System Complete Date:** _______________  
**Verified By:** _______________
