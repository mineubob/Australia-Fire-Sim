# Fire Physics Enhancements

**Status:** Not Started  
**Priority:** **HIGH** (Phase 0 is a critical gap fix)  
**Target:** Enhanced realism through terrain slope integration, vertical fuel stratification, fuel heterogeneity, crown fire transitions, and pyroconvection dynamics

---

## Phases Overview

This task is divided into **5 phases** (Phases 0-4) designed for incremental implementation by GitHub Coding Agent. Each phase is self-contained with clear deliverables and validation criteria.

| Phase | Name | Complexity | Est. Days | Dependency | Status |
|-------|------|------------|-----------|------------|--------|
| **0** | **Terrain Slope Integration** 🔥 | Medium | 2-3 | GPU Fire Front System | 🔴 CRITICAL GAP |
| **1** | **Vertical Fuel Layers** | High | 5-7 | Phase 0 | ⭐ Foundational |
| **2** | Fuel Heterogeneity | Medium | 3-4 | Phase 1 | — |
| **3** | Crown Fire Transition | High | 5-7 | Phase 1 (required) | — |
| **4** | Pyroconvection Dynamics | Very High | 8-12 | Phase 3 | — |

**Estimated Time: 23-33 days**

> 📋 **Additional advanced phases** (5-8: Junction Zones, VLS, Valley Channeling, Regime Detection) are documented in [FIRE_PHYSICS_ADVANCED.md](FIRE_PHYSICS_ADVANCED.md). Complete Phases 0-4 first.

> 🔥 **Phase 0 (Terrain Slope Integration) is a CRITICAL GAP FIX.** The legacy physics system has proper Rothermel slope effects, but the field solver does **NOT** use them. Fire currently spreads at the same rate uphill and downhill, which is physically incorrect. Fire should spread **~2× faster per 10° uphill** (McArthur 1967).

> ⚠️ **Phase 1 (Vertical Fuel Layers) is FOUNDATIONAL.** Without vertical fuel stratification, crown fire physics cannot be properly implemented. Surface fire must burn independently from canopy fuels.

---

## ⚠️ IMPORTANT: Prerequisites

This task requires the **GPU Fire Front System** (REALISTIC_GPU_FIRE_FRONT_SYSTEM.md) to be complete. The enhancements build on top of the field-based solver architecture.

### Per-Phase Build Expectations

| Phase | Build Status | Notes |
|-------|--------------|-------|
| 0 | ✅ Full | Wire existing terrain_physics.rs into field solver |
| 1 | ✅ Full | New fuel_layers module, FuelLayer struct, solver integration |
| 2 | ✅ Full | New noise module, integrates with layer-aware fuel fields |
| 3 | ✅ Full | Adds CanopyProperties, CrownFireState to solver |
| 4 | ✅ Full | New atmosphere module |

### Guidance for AI Agent

- All phases should compile and pass tests
- Each phase adds new functionality without breaking existing code
- Run `cargo clippy --all-targets --all-features` before marking complete
- Run `cargo test` to verify all tests pass

---

## Per-Phase Validation Requirements

### For GitHub Coding Agent (No GPU)

| Phase | Validation Command | Expected Result |
|-------|-------------------|-----------------|
| 1 | `cargo test --no-default-features noise` | Noise tests pass |
| 1 | `cargo test --no-default-features fuel_variation` | Fuel variation tests pass |
| 2 | `cargo test --no-default-features crown_fire` | Crown fire tests pass |
| 3 | `cargo test --no-default-features atmosphere` | Atmosphere tests pass |
| All | `cargo clippy --all-targets --no-default-features` | Zero warnings |
| All | `cargo fmt --all --check` | Formatting OK |

### For Local Development (With GPU)

| Phase | Validation Command | Expected Result |
|-------|-------------------|-----------------|
| All | `cargo clippy --all-targets --all-features` | Zero warnings |
| All | `cargo test --all-features` | All tests pass |

---

## Table of Contents

1. [Critical Architecture Rules](#critical-architecture-rules-inherited)
2. [Phase 0: Terrain Slope Integration](#phase-0-terrain-slope-integration) 🔥 **CRITICAL GAP FIX**
3. [Phase 1: Vertical Fuel Layers](#phase-1-vertical-fuel-layers) ⭐ **FOUNDATIONAL**
4. [Phase 2: Fuel Heterogeneity](#phase-2-fuel-heterogeneity)
5. [Phase 3: Crown Fire Transition](#phase-3-crown-fire-transition)
6. [Phase 4: Pyroconvection Dynamics](#phase-4-pyroconvection-dynamics)
7. [Dependencies](#dependencies)
8. [References](#references)
9. [Completion Checklist](#completion-checklist)

---

## Critical Architecture Rules (Inherited)

- [ ] **NEVER SIMPLIFY PHYSICS** - Implement formulas exactly as published in fire science literature
- [ ] **NEVER HARDCODE DYNAMIC VALUES** - Use fuel properties, weather conditions, grid state appropriately
- [ ] **FIX ROOT CAUSES, NOT SYMPTOMS** - Investigate WHY invalid values occur, don't clamp/mask
- [ ] **PUBLIC CODE MUST BE COMMENTED** - All public APIs need documentation
- [ ] **NO ALLOW MACROS** - Fix ALL clippy warnings by changing code

---

# PHASE IMPLEMENTATION DETAILS

The following sections detail each phase for GitHub Coding Agent implementation.

---

## PHASE 0: Terrain Slope Integration 🔥 CRITICAL GAP FIX

**Objective:** Wire existing terrain slope physics (`terrain_physics.rs`) into the field solver so fire spreads faster uphill and slower downhill, matching the well-established McArthur rule (~2× per 10° uphill).

**Branch Name:** `feature/fire-physics-phase0-terrain-slope`

**Estimated Time:** 2-3 days

**Depends On:** GPU Fire Front System (base solver architecture)

> 🔥 **This phase fixes a CRITICAL GAP.** The legacy physics system (`physics/terrain_physics.rs`) has proper Rothermel slope effects implemented, but they are **NOT** integrated into the field solver. Fire currently spreads at the same rate regardless of slope, which is physically incorrect.

### Phase 0 Problem Statement

The simulation has two physics systems:

| System | Slope Support | Status |
|--------|---------------|--------|
| **Legacy Element-Based** (`physics/terrain_physics.rs`) | ✅ Full Rothermel slope factor | Code exists but not used by main sim |
| **Field Solver** (`solver/heat_transfer.rs`, `solver/level_set.rs`) | ❌ None | Main simulation engine — missing slope |

Currently, the field solver uses flat 2D grids with no terrain elevation input. This means:
- Fire spreads at the same rate going uphill as downhill
- Hill terrain has no effect on fire behavior
- The simulation is missing a fundamental fire behavior principle

### Scientific Foundation

#### McArthur Rule (1967) — Industry Standard

Fire spread rate **doubles for every 10° of uphill slope**:

```
R_uphill = R_flat × 2^(θ/10)

Where:
- R_flat: Flat ground spread rate
- θ: Uphill slope angle in degrees
```

For downhill:
```
R_downhill = R_flat × 0.5^(|θ|/10)  (halves per 10°)
```

Minimum downhill rate: ~30% of flat (fire can still back slowly downhill).

#### Rothermel Slope Factor (1972)

The exact physics formula (already implemented in `rothermel.rs`):

```
Φ_s = 5.275 × β^(-0.3) × tan²(θ)

Where:
- β: Fuel packing ratio (~0.2 typical)
- θ: Slope angle in radians
```

#### Existing Implementation Reference

[terrain_physics.rs](crates/core/src/physics/terrain_physics.rs#L92-L100):
```rust
if effective_slope > 0.0 {
    // Uphill: exponential effect based on Rothermel (1972)
    // ~2x per 10° is a good approximation for typical fuels
    1.0 + (effective_slope / 10.0).powf(1.5) * 2.0
} else {
    // Downhill: reduced spread (minimum ~30% of flat ground rate)
    // Based on McArthur (1967) observations
    (1.0 + effective_slope / 30.0).max(0.3)
}
```

### Phase 0 Deliverables

- [ ] Add `elevation` field to `FieldData` struct
- [ ] Add `slope` and `aspect` fields (or compute from elevation)
- [ ] Modify `compute_spread_rate_cpu()` to apply terrain slope factor
- [ ] Update GPU shader `level_set.wgsl` with slope factor (if GPU backend)
- [ ] Initialize elevation field from `TerrainData` during solver creation
- [ ] Unit tests for slope effects on spread rate
- [ ] Integration test: fire on hill spreads faster uphill

### Phase 0 Files to Modify

```
crates/core/src/solver/
├── fields.rs              # Add elevation, slope, aspect fields
├── level_set.rs           # Apply slope factor in compute_spread_rate_cpu()
├── heat_transfer.rs       # (Optional) slope-based radiative view factor
├── cpu.rs                 # Initialize terrain fields
├── gpu.rs                 # Initialize terrain buffers
└── shaders/
    └── level_set.wgsl     # GPU slope factor
```

### Phase 0 Implementation

#### 0.1 Add Terrain Fields to FieldData

```rust
// In fields.rs, add to FieldData struct:

/// Terrain elevation at each cell (meters)
pub elevation: Vec<f32>,
/// Slope angle at each cell (degrees, 0-90)
pub slope: Vec<f32>,
/// Aspect direction at each cell (degrees, 0-360, 0=North)
pub aspect: Vec<f32>,
```

#### 0.2 Initialize Terrain Fields

```rust
// In cpu.rs or wherever solver is created:

impl CpuFieldSolver {
    pub fn new(terrain: &TerrainData, quality: QualityPreset) -> Self {
        let (width, height, cell_size) = quality.grid_dimensions(terrain);
        let num_cells = (width * height) as usize;
        
        // Sample terrain at each cell center
        let mut elevation = vec![0.0; num_cells];
        let mut slope = vec![0.0; num_cells];
        let mut aspect = vec![0.0; num_cells];
        
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let world_x = x as f32 * cell_size;
                let world_y = y as f32 * cell_size;
                
                elevation[idx] = *terrain.elevation_at(world_x, world_y);
                slope[idx] = *terrain.slope_at_horn(world_x, world_y);
                aspect[idx] = *terrain.aspect_at_horn(world_x, world_y);
            }
        }
        
        // ... rest of initialization
    }
}
```

#### 0.3 Apply Slope Factor to Spread Rate

```rust
// In level_set.rs, modify compute_spread_rate_cpu():

pub fn compute_spread_rate_cpu(
    temperature: &[f32],
    fuel_load: &[f32],
    moisture: &[f32],
    slope: &[f32],           // ADD: slope field
    aspect: &[f32],          // ADD: aspect field  
    wind_dir: f32,           // ADD: wind direction for aspect alignment
    spread_rate_out: &mut [f32],
    width: usize,
    height: usize,
    cell_size: f32,
) {
    // ... existing heat calculation ...
    
    // After calculating base spread_rate:
    
    // Calculate spread direction from temperature gradient
    let spread_direction = calculate_spread_direction(temperature, x, y, width, height);
    
    // Calculate effective slope (uphill vs downhill)
    let slope_angle = slope[idx];
    let aspect_angle = aspect[idx];
    let effective_slope = calculate_effective_slope(slope_angle, aspect_angle, spread_direction);
    
    // Apply slope factor (McArthur rule: ~2x per 10° uphill)
    let slope_factor = if effective_slope > 0.0 {
        // Uphill boost
        1.0 + (effective_slope / 10.0).powf(1.5) * 2.0
    } else {
        // Downhill reduction (min 30%)
        (1.0 + effective_slope / 30.0).max(0.3)
    };
    
    spread_rate_out[idx] = (base_spread_rate * slope_factor).clamp(0.0, 10.0);
}

/// Calculate effective slope based on fire spread direction
fn calculate_effective_slope(slope: f32, aspect: f32, spread_dir: f32) -> f32 {
    // Aspect points downslope, so upslope = aspect + 180°
    let upslope_dir = (aspect + 180.0) % 360.0;
    
    // Angular difference between spread and upslope direction
    let angle_diff = (spread_dir - upslope_dir).abs();
    let angle_diff = if angle_diff > 180.0 { 360.0 - angle_diff } else { angle_diff };
    
    // Alignment: 1.0 = spreading directly uphill, -1.0 = downhill, 0 = cross-slope
    let alignment = (180.0 - angle_diff) / 180.0;
    
    slope * alignment
}
```

#### 0.4 GPU Shader Update (level_set.wgsl)

```wgsl
// Add terrain bindings
@group(0) @binding(5) var<storage, read> slope: array<f32>;
@group(0) @binding(6) var<storage, read> aspect: array<f32>;

// In the main compute function:
fn calculate_slope_factor(idx: u32, spread_dir: f32) -> f32 {
    let slope_angle = slope[idx];
    let aspect_angle = aspect[idx];
    
    // Calculate upslope direction (aspect points downslope)
    let upslope_dir = fmod(aspect_angle + 180.0, 360.0);
    
    // Angular difference
    var angle_diff = abs(spread_dir - upslope_dir);
    if (angle_diff > 180.0) {
        angle_diff = 360.0 - angle_diff;
    }
    
    // Alignment (-1 to 1)
    let alignment = (180.0 - angle_diff) / 180.0;
    let effective_slope = slope_angle * alignment;
    
    // McArthur slope factor
    if (effective_slope > 0.0) {
        return 1.0 + pow(effective_slope / 10.0, 1.5) * 2.0;
    } else {
        return max(1.0 + effective_slope / 30.0, 0.3);
    }
}
```

### Phase 0 Validation Criteria

**For GitHub Coding Agent (no GPU):**
- [ ] `cargo build -p fire-sim-core --no-default-features` compiles
- [ ] `cargo test -p fire-sim-core --no-default-features terrain` passes
- [ ] `cargo test -p fire-sim-core --no-default-features uphill` passes

**For Local Validation (with GPU):**
- [ ] `cargo build -p fire-sim-core --all-features` compiles
- [ ] `cargo test -p fire-sim-core` passes all tests

**Common:**
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings
- [ ] `cargo fmt --all --check` passes

### Phase 0 Testing

- [ ] Unit test: `slope_factor_uphill_10_degrees` - ~2× boost at 10°
- [ ] Unit test: `slope_factor_uphill_20_degrees` - ~4× boost at 20°
- [ ] Unit test: `slope_factor_downhill` - reduction to 30% minimum
- [ ] Unit test: `slope_factor_cross_slope` - no effect (factor ~1.0)
- [ ] Integration test: `fire_spreads_faster_uphill` - fire on 15° hill spreads ~3× faster uphill
- [ ] Integration test: `hill_terrain_creates_asymmetric_perimeter` - fire perimeter elongates uphill
- [ ] Visual test: Fire on valley terrain shows expected V-shape spread

---

## PHASE 1: Vertical Fuel Layers ⭐ FOUNDATIONAL

**Objective:** Implement layered 2D fuel structure with discrete vertical layers (surface, shrub, canopy) to enable realistic fire behavior where ground-level fire does not immediately affect elevated fuels.

**Branch Name:** `feature/fire-physics-phase1-vertical-fuel-layers`

**Estimated Time:** 5-7 days

**Depends On:** GPU Fire Front System (base solver architecture)

> ⚠️ **This phase is CRITICAL.** Without vertical fuel stratification, crown fire physics (Phase 3) cannot be properly implemented. Surface fire must burn independently from canopy fuels until intensity threshold is reached.

### Phase 1 Problem Statement

The current simulation uses 2D fuel fields where fire at ground level immediately affects all fuel in a cell. In reality:

| Layer | Height Range | Fuel Types | Fire Behavior |
|-------|--------------|------------|---------------|
| **Surface** | 0-0.5m | Litter, grass, herbs | Burns first, spreads horizontally |
| **Shrub/Ladder** | 0.5-3m | Understory, bark, small trees | Ladder fuels connect surface to canopy |
| **Canopy** | 3m+ | Tree crowns | Only ignites when surface intensity exceeds Van Wagner threshold |

Current limitations:
- All fuel in a cell burns simultaneously
- No vertical heat transfer modeling
- Cannot model grass fire running under trees without igniting crowns
- Crown fire initiation threshold has no fuel layer to apply to

### Scientific Foundation

#### Layered Fuel Architecture (Rothermel 1972, Scott & Burgan 2005)

Operational fire models use discrete fuel strata:

```
Fuel Complex = Surface + Shrub + Canopy

Each layer has independent:
- Fuel load (kg/m²)
- Moisture content (%)
- Temperature (K)
- Fire state (burning/not burning)
- Consumption rate (kg/m²/s)
```

#### Vertical Heat Transfer

Heat flows upward from lower to higher layers via:

1. **Radiation** (Stefan-Boltzmann between layers):
   ```
   Q_rad = ε × σ × A × (T_lower^4 - T_upper^4)
   
   Where:
   - ε: Effective emissivity (~0.9 for vegetation)
   - σ: Stefan-Boltzmann constant (5.67e-8 W/m²K⁴)
   - A: Area factor (canopy cover fraction)
   ```

2. **Convection** (buoyant plume from surface fire):
   ```
   Q_conv = h × A × (T_flame - T_layer)
   
   Where:
   - h: Convective heat transfer coefficient (depends on flame height)
   - T_flame: Flame temperature (~1200 K for surface fire)
   ```

3. **Layer Coupling Factor** (height-dependent attenuation):
   ```
   f_coupling = exp(-k × (z_target - z_source) / L_flame)
   
   Where:
   - k: Attenuation coefficient (~0.5)
   - z: Layer heights
   - L_flame: Flame height from surface fire
   ```

#### Layer Ignition Criteria

Surface layer ignites based on fire spread (level set).

Shrub layer ignites when:
```
Q_received > Q_ignition_shrub
OR surface_intensity > I_shrub_threshold (~500 kW/m)
```

Canopy layer ignites when (Van Wagner 1977):
```
I_surface > (0.010 × CBH × (460 + 25.9 × FMC))^1.5
```

### Phase 1 Deliverables

- [ ] `FuelLayer` enum defining vertical fuel strata
- [ ] `LayeredFuelCell` struct with per-layer properties
- [ ] Extended solver fields for per-layer fuel_load, moisture, temperature, burning state
- [ ] `VerticalHeatTransfer` module for inter-layer heat flux
- [ ] Layer-aware fire spread (surface spreads via level set; upper layers ignite via threshold)
- [ ] Integration with existing `Fuel` struct (extract surface layer defaults)
- [ ] Unit tests for vertical heat transfer physics
- [ ] Integration tests verifying independent layer burning

### Phase 1 Files to Create

```
crates/core/src/solver/
├── fuel_layers.rs            # FuelLayer enum, LayeredFuelCell struct
├── vertical_heat_transfer.rs # Inter-layer heat flux calculations
└── mod.rs                    # Add fuel layer exports

crates/core/src/core_types/
└── fuel.rs                   # Extend Fuel with layer-specific defaults
```

### Phase 1 Implementation

#### 1.1 Fuel Layer Definitions

```rust
/// Discrete vertical fuel layers
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FuelLayer {
    /// Surface fuels: litter, grass, herbs (0-0.5m)
    Surface = 0,
    /// Shrub/ladder fuels: understory, bark (0.5-3m)
    Shrub = 1,
    /// Canopy fuels: tree crowns (3m+)
    Canopy = 2,
}

impl FuelLayer {
    /// Height range for this layer (min, max) in meters
    pub fn height_range(&self) -> (f32, f32) {
        match self {
            FuelLayer::Surface => (0.0, 0.5),
            FuelLayer::Shrub => (0.5, 3.0),
            FuelLayer::Canopy => (3.0, 30.0),
        }
    }
    
    /// Representative height for heat transfer calculations
    pub fn representative_height(&self) -> f32 {
        match self {
            FuelLayer::Surface => 0.25,
            FuelLayer::Shrub => 1.5,
            FuelLayer::Canopy => 10.0,
        }
    }
}
```

#### 1.2 Layered Fuel Cell

```rust
/// Per-layer fuel properties for a single cell
#[derive(Clone, Debug)]
pub struct LayerState {
    /// Fuel load remaining (kg/m²)
    pub fuel_load: f32,
    /// Moisture content (fraction, 0-1)
    pub moisture: f32,
    /// Temperature (K)
    pub temperature: f32,
    /// Is this layer currently burning?
    pub burning: bool,
    /// Heat received from lower layers this timestep (J/m²)
    pub heat_received: f32,
}

/// Complete layered fuel cell
#[derive(Clone, Debug)]
pub struct LayeredFuelCell {
    pub surface: LayerState,
    pub shrub: LayerState,
    pub canopy: LayerState,
}

impl LayeredFuelCell {
    /// Create from fuel type with default layer properties
    pub fn from_fuel(fuel: &Fuel) -> Self {
        Self {
            surface: LayerState {
                fuel_load: fuel.fuel_load,  // Existing property becomes surface
                moisture: fuel.moisture_of_extinction * 0.5,  // Default to 50% of extinction
                temperature: 300.0,  // Ambient
                burning: false,
                heat_received: 0.0,
            },
            shrub: LayerState {
                fuel_load: fuel.shrub_fuel_load.unwrap_or(0.0),
                moisture: fuel.moisture_of_extinction * 0.6,
                temperature: 300.0,
                burning: false,
                heat_received: 0.0,
            },
            canopy: LayerState {
                fuel_load: fuel.canopy.as_ref().map(|c| c.fuel_load).unwrap_or(0.0),
                moisture: fuel.canopy.as_ref().map(|c| c.foliar_moisture / 100.0).unwrap_or(1.0),
                temperature: 300.0,
                burning: false,
                heat_received: 0.0,
            },
        }
    }
    
    /// Get layer by enum
    pub fn layer(&self, layer: FuelLayer) -> &LayerState {
        match layer {
            FuelLayer::Surface => &self.surface,
            FuelLayer::Shrub => &self.shrub,
            FuelLayer::Canopy => &self.canopy,
        }
    }
    
    /// Get mutable layer by enum
    pub fn layer_mut(&mut self, layer: FuelLayer) -> &mut LayerState {
        match layer {
            FuelLayer::Surface => &mut self.surface,
            FuelLayer::Shrub => &mut self.shrub,
            FuelLayer::Canopy => &mut self.canopy,
        }
    }
}
```

#### 1.3 Vertical Heat Transfer

```rust
/// Vertical heat transfer between fuel layers
pub struct VerticalHeatTransfer {
    /// Stefan-Boltzmann constant
    stefan_boltzmann: f32,
    /// Effective emissivity for vegetation
    emissivity: f32,
    /// Convective heat transfer coefficient base
    convective_coeff_base: f32,
}

impl Default for VerticalHeatTransfer {
    fn default() -> Self {
        Self {
            stefan_boltzmann: 5.67e-8,
            emissivity: 0.9,
            convective_coeff_base: 25.0,  // W/m²K
        }
    }
}

impl VerticalHeatTransfer {
    /// Calculate heat flux from source layer to target layer (J/m² per timestep)
    pub fn calculate_flux(
        &self,
        source: &LayerState,
        source_layer: FuelLayer,
        target: &LayerState,
        target_layer: FuelLayer,
        flame_height: f32,        // Flame height from source layer (m)
        canopy_cover: f32,        // Fraction (0-1)
        dt: f32,                  // Timestep (s)
    ) -> f32 {
        // Only transfer upward
        if target_layer.representative_height() <= source_layer.representative_height() {
            return 0.0;
        }
        
        // Only transfer if source is burning
        if !source.burning {
            return 0.0;
        }
        
        let height_diff = target_layer.representative_height() - source_layer.representative_height();
        
        // Coupling factor based on flame height reaching target
        let coupling = if flame_height > height_diff {
            1.0  // Flames reach target layer directly
        } else {
            // Exponential decay with distance
            (-0.5 * (height_diff - flame_height) / flame_height.max(0.1)).exp()
        };
        
        // Radiative heat transfer (Stefan-Boltzmann)
        let t_source = source.temperature;
        let t_target = target.temperature;
        let q_rad = self.emissivity * self.stefan_boltzmann 
                  * (t_source.powi(4) - t_target.powi(4));
        
        // Convective heat transfer
        let h = self.convective_coeff_base * (flame_height / 1.0).sqrt().min(5.0);
        let t_flame = 1200.0;  // Typical flame temperature K
        let q_conv = h * (t_flame - t_target);
        
        // Total flux, attenuated by coupling and canopy cover
        let area_factor = if target_layer == FuelLayer::Canopy { canopy_cover } else { 1.0 };
        
        (q_rad + q_conv) * coupling * area_factor * dt
    }
    
    /// Apply heat to layer, handling moisture evaporation first
    pub fn apply_heat_to_layer(
        layer: &mut LayerState,
        heat_received: f32,      // J/m²
        fuel_heat_capacity: f32, // J/(kg·K)
        latent_heat_water: f32,  // J/kg (2.26e6)
    ) {
        if heat_received <= 0.0 {
            return;
        }
        
        let fuel_mass = layer.fuel_load;  // kg/m²
        let water_mass = fuel_mass * layer.moisture;  // kg/m² of water
        
        // Heat required to evaporate all moisture
        let heat_for_evaporation = water_mass * latent_heat_water;
        
        if heat_received <= heat_for_evaporation {
            // All heat goes to evaporation
            let water_evaporated = heat_received / latent_heat_water;
            layer.moisture -= water_evaporated / fuel_mass.max(0.001);
            layer.moisture = layer.moisture.max(0.0);
        } else {
            // Evaporate all moisture, remainder heats fuel
            layer.moisture = 0.0;
            let remaining_heat = heat_received - heat_for_evaporation;
            
            // Temperature rise: ΔT = Q / (m × c_p)
            if fuel_mass > 0.0 {
                let delta_t = remaining_heat / (fuel_mass * fuel_heat_capacity);
                layer.temperature += delta_t;
            }
        }
    }
}
```

#### 1.4 Layer Ignition Logic

```rust
impl LayeredFuelCell {
    /// Check if shrub layer should ignite based on surface fire intensity
    pub fn check_shrub_ignition(&mut self, surface_intensity: f32, threshold: f32) {
        // Threshold ~500 kW/m for shrub ignition
        if !self.shrub.burning 
           && self.shrub.fuel_load > 0.0 
           && surface_intensity > threshold {
            self.shrub.burning = true;
        }
    }
    
    /// Check if canopy should ignite using Van Wagner criterion
    pub fn check_canopy_ignition(
        &mut self,
        surface_intensity: f32,  // kW/m
        canopy_base_height: f32, // m
        foliar_moisture: f32,    // %
    ) {
        // Van Wagner (1977) critical intensity
        let i_critical = (0.010 * canopy_base_height * (460.0 + 25.9 * foliar_moisture)).powf(1.5);
        
        if !self.canopy.burning 
           && self.canopy.fuel_load > 0.0 
           && surface_intensity > i_critical {
            self.canopy.burning = true;
        }
    }
}
```

#### 1.5 Solver Integration

```rust
// In CpuFieldSolver or similar:

/// Layered fuel state per cell (replaces monolithic fuel_load)
pub struct LayeredSolverFields {
    /// Per-cell layered fuel data
    pub cells: Vec<LayeredFuelCell>,
    /// Vertical heat transfer calculator
    pub vertical_heat: VerticalHeatTransfer,
}

impl LayeredSolverFields {
    /// Update vertical heat transfer for one timestep
    pub fn step_vertical_heat(
        &mut self,
        surface_intensity: &[f32],  // Surface fire intensity per cell
        flame_height: &[f32],       // Flame height per cell
        canopy_cover: &[f32],       // Canopy cover fraction per cell
        dt: f32,
    ) {
        for (idx, cell) in self.cells.iter_mut().enumerate() {
            let flame_h = flame_height[idx];
            let cover = canopy_cover[idx];
            
            // Surface → Shrub
            let flux_to_shrub = self.vertical_heat.calculate_flux(
                &cell.surface, FuelLayer::Surface,
                &cell.shrub, FuelLayer::Shrub,
                flame_h, cover, dt,
            );
            cell.shrub.heat_received += flux_to_shrub;
            
            // Surface → Canopy (direct, if tall flames)
            let flux_to_canopy_direct = self.vertical_heat.calculate_flux(
                &cell.surface, FuelLayer::Surface,
                &cell.canopy, FuelLayer::Canopy,
                flame_h, cover, dt,
            );
            
            // Shrub → Canopy
            let shrub_flame_h = if cell.shrub.burning { 2.0 } else { 0.0 };
            let flux_to_canopy_from_shrub = self.vertical_heat.calculate_flux(
                &cell.shrub, FuelLayer::Shrub,
                &cell.canopy, FuelLayer::Canopy,
                shrub_flame_h, cover, dt,
            );
            
            cell.canopy.heat_received += flux_to_canopy_direct + flux_to_canopy_from_shrub;
            
            // Apply accumulated heat to each layer
            VerticalHeatTransfer::apply_heat_to_layer(
                &mut cell.shrub, cell.shrub.heat_received, 1800.0, 2.26e6
            );
            VerticalHeatTransfer::apply_heat_to_layer(
                &mut cell.canopy, cell.canopy.heat_received, 1800.0, 2.26e6
            );
            
            // Reset heat accumulators
            cell.shrub.heat_received = 0.0;
            cell.canopy.heat_received = 0.0;
            
            // Check ignition thresholds
            let intensity = surface_intensity[idx];
            cell.check_shrub_ignition(intensity, 500.0);  // 500 kW/m threshold
            
            // For canopy, need CBH and FMC from fuel properties
            // (This would come from the Fuel LUT in real implementation)
            cell.check_canopy_ignition(intensity, 8.0, 100.0);  // Default CBH=8m, FMC=100%
        }
    }
}
```

### Phase 1 Validation Criteria

**For GitHub Coding Agent (no GPU):**
- [ ] `cargo build -p fire-sim-core --no-default-features` compiles
- [ ] `cargo test -p fire-sim-core --no-default-features fuel_layer` passes
- [ ] `cargo test -p fire-sim-core --no-default-features vertical_heat` passes

**For Local Validation (with GPU):**
- [ ] `cargo build -p fire-sim-core --all-features` compiles
- [ ] `cargo test -p fire-sim-core` passes all tests

**Common:**
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings
- [ ] `cargo fmt --all --check` passes

### Phase 1 Testing

- [ ] Unit test: `fuel_layer_heights` - correct height ranges for each layer
- [ ] Unit test: `vertical_heat_flux_upward_only` - no downward heat transfer
- [ ] Unit test: `vertical_heat_flux_zero_when_not_burning` - no flux from unburning layer
- [ ] Unit test: `moisture_evaporates_before_temperature_rise` - latent heat properly applied
- [ ] Unit test: `shrub_ignition_threshold` - shrub ignites at correct intensity
- [ ] Unit test: `canopy_ignition_van_wagner` - canopy ignites at Van Wagner threshold
- [ ] Integration test: `grass_fire_under_trees` - surface burns, canopy stays unburned for low intensity
- [ ] Integration test: `crown_fire_initiation` - canopy ignites when surface intensity exceeds threshold

---

## PHASE 2: Fuel Heterogeneity

**Objective:** Add sub-grid fuel variation using spatially correlated noise to create more realistic fire spread patterns.

**Branch Name:** `feature/fire-physics-phase2-fuel-heterogeneity`

**Estimated Time:** 3-4 days

**Depends On:** Phase 1 (Vertical Fuel Layers) — variation is applied per-layer

### Phase 2 Problem Statement

The current system uses uniform fuel properties per cell. Real fuel beds exhibit significant sub-grid variation:
- Patchy fuel loading (bare ground, dense shrubs, grass tussocks)
- Variable moisture content (shaded vs exposed areas)
- Mixed fuel types within a single cell (e.g., grass under eucalyptus)

This uniformity leads to:
- Overly smooth fire spread within fuel types
- Missing small-scale fingering from fuel variation
- Unrealistic rate of spread in heterogeneous landscapes

### Scientific Foundation

#### Sub-Grid Fuel Variation (Finney 2003)

Real fuel beds can be modeled with spatial correlation:

```
F(x,y) = F_base × (1 + σ_fuel × η(x,y))

Where:
- F_base: Mean fuel load for fuel type (kg/m²)
- σ_fuel: Coefficient of variation (typically 0.2-0.5)
- η(x,y): Spatially correlated noise field (-1 to 1)
```

#### Perlin/Simplex Noise for Fuel Variation

```
η(x,y) = Σᵢ (amplitude_i × noise(x × frequency_i, y × frequency_i))

Typical parameters:
- Octave 1: frequency=0.1, amplitude=0.6 (large patches)
- Octave 2: frequency=0.3, amplitude=0.3 (medium variation)
- Octave 3: frequency=0.9, amplitude=0.1 (fine detail)
```

#### Moisture Microclimate Variation (Bradshaw 1984)

```
M(x,y) = M_base × (1 + σ_moisture × (slope_factor + shade_factor + noise))

Where:
- slope_factor: South-facing slopes drier (Australia: north-facing drier)
- shade_factor: Canopy cover increases moisture retention
- noise: Random variation (σ ~ 0.15)
```

### Phase 2 Deliverables

- [ ] `NoiseGenerator` struct with simplex/Perlin noise
- [ ] `HeterogeneityConfig` configuration struct
- [ ] `apply_fuel_heterogeneity()` function (layer-aware)
- [ ] Aspect-based moisture variation per layer
- [ ] Unit tests for noise generation
- [ ] Integration with `CpuFieldSolver` initialization

### Phase 2 Files to Create

```
crates/core/src/solver/
├── noise.rs              # Simplex/Perlin noise implementation
├── fuel_variation.rs     # Sub-grid fuel heterogeneity
└── shaders/
    └── fuel_noise.wgsl   # GPU noise generation (optional)
```

### Phase 2 Implementation

#### 2.1 Noise Field Generator

```rust
/// Generates spatially correlated noise for fuel variation
pub struct NoiseGenerator {
    seed: u64,
    octaves: Vec<NoiseOctave>,
}

pub struct NoiseOctave {
    frequency: f32,
    amplitude: f32,
}

impl NoiseGenerator {
    /// Generate noise value at position
    pub fn sample(&self, x: f32, y: f32) -> f32;
    
    /// Generate noise field for entire grid
    pub fn generate_field(&self, width: u32, height: u32, cell_size: f32) -> Vec<f32>;
}
```

#### 2.2 Fuel Variation Application

```rust
/// Apply sub-grid variation to fuel fields
pub fn apply_fuel_heterogeneity(
    fuel_load: &mut [f32],
    moisture: &mut [f32],
    terrain: &TerrainData,
    fuel_types: &[u8],
    fuel_lut: &[Fuel],
    noise: &NoiseGenerator,
    config: &HeterogeneityConfig,
) {
    for (idx, &fuel_type) in fuel_types.iter().enumerate() {
        let fuel = &fuel_lut[fuel_type as usize];
        let x = (idx % width) as f32 * cell_size;
        let y = (idx / width) as f32 * cell_size;
        
        // Fuel load variation
        let fuel_noise = noise.sample(x * 0.1, y * 0.1);
        fuel_load[idx] *= 1.0 + fuel.load_cv * fuel_noise;
        
        // Moisture variation (aspect + noise)
        let aspect_factor = calculate_aspect_moisture(terrain, idx);
        let moisture_noise = noise.sample(x * 0.2 + 1000.0, y * 0.2);
        moisture[idx] *= 1.0 + 0.15 * (aspect_factor + moisture_noise);
    }
}
```

#### 2.3 Configuration

```rust
pub struct HeterogeneityConfig {
    /// Enable fuel load variation
    pub fuel_variation_enabled: bool,
    /// Coefficient of variation for fuel load (0.0-1.0)
    pub fuel_cv: f32,
    /// Enable moisture microclimate variation
    pub moisture_variation_enabled: bool,
    /// Coefficient of variation for moisture (0.0-0.5)
    pub moisture_cv: f32,
    /// Noise seed for reproducibility
    pub seed: u64,
}

impl Default for HeterogeneityConfig {
    fn default() -> Self {
        Self {
            fuel_variation_enabled: true,
            fuel_cv: 0.3,
            moisture_variation_enabled: true,
            moisture_cv: 0.15,
            seed: 42,
        }
    }
}
```

### Phase 2 Validation Criteria

**For GitHub Coding Agent (no GPU):**
- [ ] `cargo build -p fire-sim-core --no-default-features` compiles
- [ ] `cargo test -p fire-sim-core --no-default-features noise` passes
- [ ] `cargo test -p fire-sim-core --no-default-features fuel_variation` passes

**For Local Validation (with GPU):**
- [ ] `cargo build -p fire-sim-core --all-features` compiles
- [ ] `cargo test -p fire-sim-core` passes all tests

**Common:**
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings
- [ ] `cargo fmt --all --check` passes

### Phase 2 Testing

- [ ] Unit test: `noise_generator_produces_valid_range` - values in [-1, 1]
- [ ] Unit test: `fuel_variation_preserves_mean` - mean fuel load unchanged
- [ ] Unit test: `aspect_calculation_cardinal` - correct for N/S/E/W slopes
- [ ] Unit test: `noise_spatial_correlation` - nearby values correlated
- [ ] Visual test: Fire perimeter more irregular with heterogeneity enabled

---

## PHASE 3: Crown Fire Transition

**Objective:** Implement surface-to-crown fire transitions using Van Wagner (1977) and Cruz et al. (2005) models.

**Branch Name:** `feature/fire-physics-phase3-crown-fire`

**Estimated Time:** 5-7 days

**Depends On:** Phase 1 (Vertical Fuel Layers) — **REQUIRED** for layer-based fire state tracking

### Phase 3 Problem Statement

The current system models surface fire only. Real bushfires transition between:
1. **Surface fire** - Burns ground fuels (grass, litter, shrubs)
2. **Passive crown fire** - Individual tree crowns ignite (torching)
3. **Active crown fire** - Continuous crown-to-crown spread

Crown fires spread 2-10× faster than surface fires and are responsible for most extreme fire behavior (Black Saturday, 2009 Victorian bushfires).

### Phase 3 Scientific Foundation

#### Van Wagner Crown Fire Initiation (1977)

Crown fire initiates when surface fire intensity exceeds critical threshold:

```
I_critical = (0.010 × CBH × (460 + 25.9 × FMC))^1.5

Where:
- I_critical: Critical surface intensity (kW/m)
- CBH: Canopy base height (m)
- FMC: Foliar moisture content (%)
```

#### Crown Fire Types (Van Wagner 1977, Alexander 1988)

```
Active Crown Fire Criterion:
R_surface > R_critical = 3.0 / CBD

Where:
- R_surface: Surface fire rate of spread (m/min)
- CBD: Canopy bulk density (kg/m³)
- Typical CBD: 0.05-0.3 kg/m³ for eucalyptus
```

#### Crown Fire Spread Rate (Cruz et al. 2005)

Australian crown fire model:

```
R_crown = 11.02 × U₁₀^0.90 × (1 - 0.95 × e^(-0.17 × M_dead))

Where:
- R_crown: Crown fire rate of spread (m/min)
- U₁₀: 10-m wind speed (km/h)
- M_dead: Dead fine fuel moisture (%)
```

#### Flame Height Transition

```
Surface: L = 0.0775 × I^0.46 (Byram 1959)
Crown: L = CBH + 0.1 × I^0.5 (reaching into canopy)
```

### Phase 3 Deliverables

- [ ] `CanopyProperties` struct added to `Fuel`
- [ ] `CrownFireState` enum (Surface, Passive, Active)
- [ ] `CrownFirePhysics` with Van Wagner initiation
- [ ] `crown_spread_rate()` using Cruz et al. (2005)
- [ ] GPU shader `crown_fire.wgsl` (optional)
- [ ] Integration with level set effective ROS
- [ ] Unit tests for all physics calculations

### Phase 3 Files to Create

```
crates/core/src/solver/
├── crown_fire.rs                 # Crown fire physics
├── shaders/
│   └── crown_fire.wgsl           # GPU crown fire shader
└── mod.rs                        # Add crown fire exports

crates/core/src/core_types/
└── fuel.rs                       # Add canopy properties
```

### Phase 3 Implementation

#### 3.1 Canopy Properties (Add to Fuel struct)

```rust
/// Canopy properties for crown fire modeling
#[derive(Clone, Debug)]
pub struct CanopyProperties {
    /// Canopy base height - height to live crown base (m)
    pub base_height: f32,
    /// Canopy bulk density - mass of available fuel per volume (kg/m³)
    pub bulk_density: f32,
    /// Foliar moisture content - live fuel moisture (%)
    pub foliar_moisture: f32,
    /// Canopy cover fraction (0-1)
    pub cover_fraction: f32,
    /// Canopy fuel load (kg/m²)
    pub fuel_load: f32,
    /// Heat content of canopy fuels (kJ/kg)
    pub heat_content: f32,
}

impl CanopyProperties {
    /// Eucalyptus forest defaults (Cruz et al. 2005)
    pub fn eucalyptus_forest() -> Self {
        Self {
            base_height: 8.0,           // 8m typical
            bulk_density: 0.15,         // kg/m³
            foliar_moisture: 100.0,     // % (live foliage)
            cover_fraction: 0.7,
            fuel_load: 1.5,             // kg/m²
            heat_content: 21_000.0,     // kJ/kg (with oils)
        }
    }
    
    /// Calculate critical intensity for crown ignition
    pub fn critical_intensity(&self) -> f32 {
        // Van Wagner (1977)
        let cbh = self.base_height;
        let fmc = self.foliar_moisture;
        (0.010 * cbh * (460.0 + 25.9 * fmc)).powf(1.5)
    }
    
    /// Calculate critical rate of spread for active crown fire
    pub fn critical_ros(&self) -> f32 {
        // Van Wagner (1977)
        3.0 / self.bulk_density
    }
}
```

#### 3.2 Crown Fire State

```rust
/// Crown fire state for each cell
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CrownFireState {
    /// No crown fire activity
    Surface = 0,
    /// Individual trees torching (passive crown fire)
    Passive = 1,
    /// Continuous crown-to-crown spread (active crown fire)
    Active = 2,
}
```

#### 3.3 Crown Fire Physics Module

```rust
/// Crown fire transition and spread calculations
pub struct CrownFirePhysics;

impl CrownFirePhysics {
    /// Determine crown fire state based on surface intensity
    pub fn evaluate_transition(
        surface_intensity: f32,  // kW/m
        surface_ros: f32,        // m/s
        canopy: &CanopyProperties,
    ) -> CrownFireState {
        let i_crit = canopy.critical_intensity();
        let r_crit = canopy.critical_ros();
        
        if surface_intensity < i_crit {
            CrownFireState::Surface
        } else if surface_ros * 60.0 < r_crit {  // Convert m/s to m/min
            CrownFireState::Passive
        } else {
            CrownFireState::Active
        }
    }
    
    /// Calculate crown fire rate of spread (Cruz et al. 2005)
    pub fn crown_spread_rate(
        wind_speed_10m: f32,    // km/h
        dead_fuel_moisture: f32, // fraction
    ) -> f32 {
        // R in m/min, convert to m/s at end
        let m_percent = dead_fuel_moisture * 100.0;
        let r_crown = 11.02 * wind_speed_10m.powf(0.90) 
                    * (1.0 - 0.95 * (-0.17 * m_percent).exp());
        r_crown / 60.0  // m/s
    }
    
    /// Calculate combined surface + crown intensity
    pub fn total_intensity(
        surface_intensity: f32,
        canopy: &CanopyProperties,
        crown_ros: f32,
    ) -> f32 {
        // Byram's intensity with crown fuel contribution
        let crown_intensity = canopy.heat_content * canopy.fuel_load * crown_ros;
        surface_intensity + crown_intensity * canopy.cover_fraction
    }
}
```

#### 3.4 GPU Shader (crown_fire.wgsl)

```wgsl
struct CrownFireParams {
    width: u32,
    height: u32,
    cell_size: f32,
    wind_speed_10m: f32,
    dead_fuel_moisture: f32,
}

struct CanopyData {
    base_height: f32,
    bulk_density: f32,
    foliar_moisture: f32,
    cover_fraction: f32,
}

@group(0) @binding(0) var<storage, read> surface_intensity: array<f32>;
@group(0) @binding(1) var<storage, read> surface_ros: array<f32>;
@group(0) @binding(2) var<storage, read> canopy: array<CanopyData>;
@group(0) @binding(3) var<storage, read_write> crown_state: array<u32>;
@group(0) @binding(4) var<storage, read_write> effective_ros: array<f32>;
@group(0) @binding(5) var<uniform> params: CrownFireParams;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.y * params.width + id.x;
    
    let intensity = surface_intensity[idx];
    let ros = surface_ros[idx];
    let can = canopy[idx];
    
    // Van Wagner (1977) critical intensity
    let i_crit = pow(0.010 * can.base_height * (460.0 + 25.9 * can.foliar_moisture), 1.5);
    
    // Van Wagner (1977) critical ROS
    let r_crit = 3.0 / can.bulk_density;
    
    // Determine state
    var state: u32 = 0u;  // Surface
    var r_effective = ros;
    
    if (intensity >= i_crit) {
        if (ros * 60.0 >= r_crit) {
            // Active crown fire
            state = 2u;
            // Cruz et al. (2005) crown fire ROS
            let m = params.dead_fuel_moisture * 100.0;
            let r_crown = 11.02 * pow(params.wind_speed_10m, 0.90) 
                        * (1.0 - 0.95 * exp(-0.17 * m)) / 60.0;
            r_effective = max(ros, r_crown);
        } else {
            // Passive crown fire (torching)
            state = 1u;
            r_effective = ros * 1.5;  // Moderate increase
        }
    }
    
    crown_state[idx] = state;
    effective_ros[idx] = r_effective;
}
```

#### 3.5 Integration with Level Set

Modify level set evolution to use effective ROS that includes crown fire:

```rust
// In level_set.rs or simulation update
fn step_level_set(&mut self, dt: f32) {
    // First, evaluate crown fire transitions
    self.evaluate_crown_fire_transitions();
    
    // Then evolve level set with effective ROS
    // effective_ros incorporates crown fire contribution
    self.solver.step_level_set(dt);
}
```

### Phase 3 Validation Criteria

**For GitHub Coding Agent (no GPU):**
- [ ] `cargo build -p fire-sim-core --no-default-features` compiles
- [ ] `cargo test -p fire-sim-core --no-default-features crown_fire` passes

**For Local Validation (with GPU):**
- [ ] `cargo build -p fire-sim-core --all-features` compiles
- [ ] `cargo test -p fire-sim-core` passes all tests
- [ ] Crown fire shader compiles and dispatches correctly

**Common:**
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings
- [ ] `cargo fmt --all --check` passes

### Phase 3 Testing

- [ ] Unit test: `van_wagner_critical_intensity` - I_critical matches published values
- [ ] Unit test: `crown_ros_wind_speed` - ROS scales with wind per Cruz (2005)
- [ ] Unit test: `state_transition_thresholds` - Surface→Passive→Active correct
- [ ] Unit test: `total_intensity_includes_canopy` - Crown fuel contribution
- [ ] Integration test: Crown fire doubles ROS in forest fuel type
- [ ] Validation: Compare to Cruz et al. (2005) Table 2 experimental data

---

## PHASE 4: Pyroconvection Dynamics

**Objective:** Implement convective column, atmospheric stability, downdrafts, fire whirls, and pyroCb detection.

**Branch Name:** `feature/fire-physics-phase4-pyroconvection`

**Estimated Time:** 8-12 days

**Depends On:** Phase 3 (crown fire intensity needed for accurate plume height)

### Phase 4 Problem Statement

Extreme fires generate their own weather through pyroconvection:
- **Convection column** rises 5-15 km into atmosphere
- **Pyrocumulonimbus (pyroCb)** generates lightning, downbursts
- **Downdrafts** create erratic, dangerous fire behavior
- **Fire whirls** occur at column base

The current system has basic plume coupling but lacks:
- True vertical convection modeling
- Atmospheric instability effects
- Downdraft and gust fronts
- pyroCb detection

### Phase 4 Deliverables

- [ ] New `atmosphere/` module with proper exports
- [ ] `ConvectionColumn` struct with Byram/Briggs physics
- [ ] `AtmosphericStability` with Haines Index calculation
- [ ] `Downdraft` struct with outflow physics
- [ ] `FireWhirlDetector` with vorticity calculation
- [ ] `PyroCbSystem` for pyroCb lifecycle
- [ ] Wind field integration for entrainment/downdrafts
- [ ] Unit tests for all physics calculations

### Phase 4 Files to Create

```
crates/core/src/atmosphere/
├── mod.rs                    # Atmosphere module
├── convection.rs             # Convective column dynamics
├── instability.rs            # Atmospheric stability indices
├── downdraft.rs              # Downdraft and gust front physics
├── pyrocb.rs                 # Pyrocumulonimbus detection
└── fire_whirl.rs             # Fire whirl formation

crates/core/src/grid/
└── vertical.rs               # 3D atmospheric grid (optional)
```

### Phase 4 Scientific Foundation

#### Byram's Convective Column (1959)

Plume rise height:

```
z_max = (I / (ρ × c_p × ΔT))^(1/3) × (g/T_amb)^(-1/3)

Where:
- z_max: Maximum plume height (m)
- I: Fire intensity (W/m²)
- ρ: Air density (kg/m³)
- c_p: Specific heat of air (1005 J/kg·K)
- ΔT: Temperature excess (K)
- g: Gravity (9.81 m/s²)
- T_amb: Ambient temperature (K)
```

#### Atmospheric Instability (Haines Index)

```
Haines Index (HI) = Stability Term + Moisture Term

Stability (low level):
- A = 1 if (T_950 - T_850) < 4
- A = 2 if 4 ≤ (T_950 - T_850) < 8
- A = 3 if (T_950 - T_850) ≥ 8

pyroCb likelihood increases with HI ≥ 5
```

#### Downdraft Velocity (Byers & Braham 1949)

```
w_down = -sqrt(2 × g × H × (θ_env - θ_parcel) / θ_env)

Where:
- w_down: Downdraft velocity (m/s, negative = downward)
- H: Downdraft depth (m)
- θ: Potential temperature (K)
```

#### Fire Whirl Formation (Clark et al. 1996)

```
Vorticity condition:
ω = (∂v/∂x - ∂u/∂y) > ω_critical

Fire whirl forms when:
1. Strong horizontal wind shear exists
2. High convective intensity (I > 10 MW/m)
3. Fuel configuration creates channeling
```

### Phase 4 Implementation

#### 4.1 Convection Column Model

```rust
/// Convective column dynamics for extreme fires
pub struct ConvectionColumn {
    /// Fire intensity (W/m)
    pub intensity: f32,
    /// Plume height (m)
    pub height: f32,
    /// Updraft velocity at base (m/s)
    pub updraft_velocity: f32,
    /// Column center position
    pub position: Vec2,
    /// Column radius at base (m)
    pub base_radius: f32,
}

impl ConvectionColumn {
    /// Calculate plume height using Byram (1959)
    pub fn calculate_plume_height(
        intensity: f32,        // W/m of fire front
        fire_length: f32,      // m
        ambient_temp: f32,     // K
        wind_speed: f32,       // m/s
    ) -> f32 {
        let rho = 1.225;  // kg/m³
        let cp = 1005.0;  // J/(kg·K)
        let g = 9.81;     // m/s²
        let delta_t = 300.0;  // Temperature excess K
        
        // Total power
        let power = intensity * fire_length;  // W
        
        // Buoyancy flux
        let fb = (g * power) / (rho * cp * ambient_temp);
        
        // Plume rise (Briggs 1975 for buoyancy-dominated)
        let z = 3.8 * fb.powf(0.6) / wind_speed;
        
        z.min(15000.0)  // Cap at ~15 km (tropopause)
    }
    
    /// Calculate updraft velocity at plume base
    pub fn updraft_velocity(intensity: f32, radius: f32) -> f32 {
        // Simplified updraft: w = sqrt(2 * g * H * ΔT / T)
        let delta_t = 300.0 * (intensity / 100_000.0).min(1.0);
        (2.0 * 9.81 * 100.0 * delta_t / 300.0).sqrt()
    }
}
```

#### 4.2 Atmospheric Instability

```rust
/// Atmospheric stability indices
pub struct AtmosphericStability {
    /// Haines Index (1-6)
    pub haines_index: u8,
    /// Continuous Haines Index (for finer resolution)
    pub c_haines: f32,
    /// Lifted Index
    pub lifted_index: f32,
    /// Mixing height (m)
    pub mixing_height: f32,
}

impl AtmosphericStability {
    /// Calculate Haines Index from temperature profile
    pub fn haines_index(
        t_950: f32,  // Temperature at 950 hPa (°C)
        t_850: f32,  // Temperature at 850 hPa (°C)
        td_850: f32, // Dew point at 850 hPa (°C)
    ) -> u8 {
        // Stability term (A)
        let lapse = t_950 - t_850;
        let a = if lapse < 4.0 { 1 } 
               else if lapse < 8.0 { 2 } 
               else { 3 };
        
        // Moisture term (B) - dew point depression
        let depression = t_850 - td_850;
        let b = if depression < 6.0 { 1 }
               else if depression < 10.0 { 2 }
               else { 3 };
        
        (a + b) as u8
    }
    
    /// Likelihood of pyroCb development
    pub fn pyrocb_potential(&self, fire_intensity: f32) -> f32 {
        // High Haines + High intensity = pyroCb likely
        let haines_factor = (self.haines_index as f32 - 2.0) / 4.0;
        let intensity_factor = (fire_intensity / 50_000.0).min(1.0);
        
        haines_factor * intensity_factor
    }
}
```

#### 4.3 Downdraft Physics

```rust
/// Downdraft and gust front dynamics
pub struct Downdraft {
    /// Center position
    pub position: Vec2,
    /// Velocity (negative = downward)
    pub vertical_velocity: f32,
    /// Radius of influence
    pub radius: f32,
    /// Outflow velocity at surface
    pub outflow_velocity: f32,
    /// Outflow direction (radial)
    pub outflow_direction: Vec2,
}

impl Downdraft {
    /// Create downdraft from pyroCb collapse
    pub fn from_pyrocb(
        pyrocb_position: Vec2,
        column_height: f32,
        ambient_temp: f32,
        precipitation_loading: f32,  // kg/m³
    ) -> Self {
        // Downdraft velocity from negative buoyancy
        let delta_theta = -10.0 * precipitation_loading;  // Cooling from evaporation
        let w_down = -(2.0 * 9.81 * column_height * delta_theta.abs() / ambient_temp).sqrt();
        
        // Outflow velocity (momentum conservation)
        let outflow = (-w_down * 0.8).max(5.0);
        
        Self {
            position: pyrocb_position,
            vertical_velocity: w_down,
            radius: 500.0,  // Initial radius
            outflow_velocity: outflow,
            outflow_direction: Vec2::ZERO,  // Radial
        }
    }
    
    /// Update outflow spreading
    pub fn update(&mut self, dt: f32) {
        // Expand radius as downdraft spreads
        self.radius += self.outflow_velocity * dt * 0.5;
        
        // Decay outflow velocity
        self.outflow_velocity *= 0.99_f32.powf(dt);
    }
    
    /// Get wind modification at position
    pub fn wind_effect_at(&self, position: Vec2) -> Vec2 {
        let to_pos = position - self.position;
        let distance = to_pos.length();
        
        if distance > self.radius || distance < 0.1 {
            return Vec2::ZERO;
        }
        
        // Radial outflow, decaying with distance
        let direction = to_pos.normalize();
        let strength = self.outflow_velocity * (1.0 - distance / self.radius);
        
        direction * strength
    }
}
```

#### 4.4 Fire Whirl Detection

```rust
/// Fire whirl formation conditions
pub struct FireWhirlDetector {
    /// Vorticity threshold for whirl formation
    pub vorticity_threshold: f32,
    /// Minimum intensity for whirl formation (kW/m)
    pub intensity_threshold: f32,
}

impl FireWhirlDetector {
    /// Detect potential fire whirl locations
    pub fn detect(
        &self,
        wind_field: &WindField,
        intensity_field: &[f32],
        width: u32,
        height: u32,
        cell_size: f32,
    ) -> Vec<Vec2> {
        let mut whirl_locations = Vec::new();
        
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let idx = (y * width + x) as usize;
                
                // Skip low-intensity cells
                if intensity_field[idx] < self.intensity_threshold * 1000.0 {
                    continue;
                }
                
                // Calculate vorticity: ω = ∂v/∂x - ∂u/∂y
                let u_up = wind_field.get(x, y - 1).x;
                let u_down = wind_field.get(x, y + 1).x;
                let v_left = wind_field.get(x - 1, y).y;
                let v_right = wind_field.get(x + 1, y).y;
                
                let dvdx = (v_right - v_left) / (2.0 * cell_size);
                let dudy = (u_down - u_up) / (2.0 * cell_size);
                let vorticity = (dvdx - dudy).abs();
                
                if vorticity > self.vorticity_threshold {
                    whirl_locations.push(Vec2::new(
                        x as f32 * cell_size,
                        y as f32 * cell_size,
                    ));
                }
            }
        }
        
        whirl_locations
    }
}
```

#### 4.5 pyroCb Detection and Effects

```rust
/// Pyrocumulonimbus (pyroCb) detection and effects
pub struct PyroCbSystem {
    /// Active pyroCb events
    pub active_events: Vec<PyroCbEvent>,
    /// Detection threshold (fire power in GW)
    pub detection_threshold: f32,
}

pub struct PyroCbEvent {
    /// Position
    pub position: Vec2,
    /// Cloud top height (m)
    pub cloud_top: f32,
    /// Formation time
    pub start_time: f32,
    /// Collapse pending
    pub collapse_pending: bool,
    /// Associated downdrafts
    pub downdrafts: Vec<Downdraft>,
}

impl PyroCbSystem {
    /// Check for new pyroCb formation
    pub fn check_formation(
        &mut self,
        total_fire_power: f32,        // GW
        convection_columns: &[ConvectionColumn],
        stability: &AtmosphericStability,
        sim_time: f32,
    ) {
        // pyroCb forms when:
        // 1. High fire power (>5 GW typically)
        // 2. Unstable atmosphere (Haines >= 5)
        // 3. Tall convection column (>8 km)
        
        if total_fire_power < self.detection_threshold {
            return;
        }
        
        for column in convection_columns {
            if column.height > 8000.0 && stability.haines_index >= 5 {
                // Create pyroCb event
                let event = PyroCbEvent {
                    position: column.position,
                    cloud_top: column.height * 1.2,  // Overshooting
                    start_time: sim_time,
                    collapse_pending: false,
                    downdrafts: Vec::new(),
                };
                
                self.active_events.push(event);
            }
        }
    }
    
    /// Update pyroCb events (collapse, downdrafts)
    pub fn update(&mut self, dt: f32, sim_time: f32) {
        for event in &mut self.active_events {
            // pyroCb typically collapses after 20-60 minutes
            if sim_time - event.start_time > 1800.0 && !event.collapse_pending {
                event.collapse_pending = true;
                
                // Generate downdrafts from collapse
                let downdraft = Downdraft::from_pyrocb(
                    event.position,
                    event.cloud_top * 0.5,
                    288.0,  // Ambient temp
                    0.003,  // Precipitation loading
                );
                event.downdrafts.push(downdraft);
            }
            
            // Update active downdrafts
            for downdraft in &mut event.downdrafts {
                downdraft.update(dt);
            }
        }
        
        // Remove expired events
        self.active_events.retain(|e| {
            e.downdrafts.iter().any(|d| d.outflow_velocity > 1.0)
            || !e.collapse_pending
        });
    }
}
```

### Integration with Wind Field

```rust
impl WindField {
    /// Apply pyroconvection effects to wind field
    pub fn apply_pyroconvection(
        &mut self,
        pyrocb_system: &PyroCbSystem,
        convection_columns: &[ConvectionColumn],
    ) {
        // Apply entrainment toward convection columns
        for column in convection_columns {
            self.apply_entrainment(column.position, column.updraft_velocity, column.base_radius);
        }
        
        // Apply downdraft outflows
        for event in &pyrocb_system.active_events {
            for downdraft in &event.downdrafts {
                self.apply_downdraft(downdraft);
            }
        }
    }
    
    fn apply_entrainment(&mut self, center: Vec2, updraft: f32, radius: f32) {
        // Wind flows toward column base
        // v_entrain = 0.1 × (updraft)^(1/3) × (r/R)
        for y in 0..self.height {
            for x in 0..self.width {
                let pos = Vec2::new(x as f32, y as f32) * self.cell_size;
                let to_center = center - pos;
                let dist = to_center.length();
                
                if dist < radius * 3.0 && dist > radius {
                    let entrain_vel = 0.1 * updraft.powf(0.33) * (radius / dist);
                    let direction = to_center.normalize();
                    self.add_wind_at(x, y, direction * entrain_vel);
                }
            }
        }
    }
    
    fn apply_downdraft(&mut self, downdraft: &Downdraft) {
        for y in 0..self.height {
            for x in 0..self.width {
                let pos = Vec2::new(x as f32, y as f32) * self.cell_size;
                let effect = downdraft.wind_effect_at(pos);
                self.add_wind_at(x, y, effect);
            }
        }
    }
}
```

### Phase 4 Validation Criteria

**For GitHub Coding Agent (no GPU):**
- [ ] `cargo build -p fire-sim-core --no-default-features` compiles
- [ ] `cargo test -p fire-sim-core --no-default-features atmosphere` passes
- [ ] `cargo test -p fire-sim-core --no-default-features convection` passes
- [ ] `cargo test -p fire-sim-core --no-default-features downdraft` passes

**For Local Validation (with GPU):**
- [ ] `cargo build -p fire-sim-core --all-features` compiles
- [ ] `cargo test -p fire-sim-core` passes all tests

**Common:**
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings
- [ ] `cargo fmt --all --check` passes

### Phase 4 Testing

- [ ] Unit test: `plume_height_byram` - matches Byram (1959) calculations
- [ ] Unit test: `plume_height_briggs` - matches Briggs (1975) for buoyancy
- [ ] Unit test: `haines_index_calculation` - correct for test profiles
- [ ] Unit test: `downdraft_velocity` - 10-30 m/s for typical conditions
- [ ] Unit test: `downdraft_outflow_spreading` - radius increases over time
- [ ] Unit test: `vorticity_calculation` - correct for shear patterns
- [ ] Unit test: `pyrocb_formation_conditions` - triggers at correct thresholds
- [ ] Integration test: `pyrocb_lifecycle` - formation → collapse → downdraft
- [ ] Visual test: Wind vectors show entrainment toward convection column

---

## Dependencies

No new dependencies required. All enhancements use existing libraries:
- `rayon` for parallel computation
- `bytemuck` for GPU buffer packing
- `wgpu` for GPU shaders (if needed)

---

## References

### Terrain Slope Effects
1. **McArthur, A.G. (1967)**. "Fire Behaviour in Eucalypt Forests." Forestry and Timber Bureau Leaflet 107.
2. **Rothermel, R.C. (1972)**. "A mathematical model for predicting fire spread in wildland fuels." USDA Forest Service INT-115. (Slope factor: Φ_s = 5.275 × β^(-0.3) × tan²(θ))
3. **Horn, B.K.P. (1981)**. "Hill Shading and the Reflectance Map." Proc. IEEE 69(1), 14-47. (3×3 kernel for slope/aspect)
4. **Butler, B.W. et al. (2004)**. "Fire behavior on slopes." Fire Management Today 64(1).
5. **NSW Fire & Rescue**. "Bushfire Research — Slope Effects." https://www.fire.nsw.gov.au/page.php?id=132

### Vertical Fuel Layers
6. **Rothermel, R.C. (1972)**. "A mathematical model for predicting fire spread in wildland fuels." USDA Forest Service INT-115.
7. **Scott, J.H. & Burgan, R.E. (2005)**. "Standard fire behavior fuel models." USDA Forest Service RMRS-GTR-153.

### Fuel Heterogeneity
8. **Finney, M.A. (2003)**. "Calculation of fire spread rates across random landscapes." IJWF.
9. **Bradshaw, L.S. et al. (1984)**. "The 1978 National Fire-Danger Rating System." USDA.

### Crown Fire
10. **Van Wagner, C.E. (1977)**. "Conditions for the start and spread of crown fire." Can. J. For. Res.
11. **Alexander, M.E. (1988)**. "Crown fire thresholds in exotic pine plantations." CJFR.
12. **Cruz, M.G. et al. (2005)**. "Development and testing of models for predicting crown fire rate of spread." Can. J. For. Res.

### Pyroconvection
13. **Byram, G.M. (1959)**. "Combustion of forest fuels." Forest Fires: Control and Use.
14. **Briggs, G.A. (1975)**. "Plume rise predictions." NOAA.
15. **Byers, H.R. & Braham, R.R. (1949)**. "The Thunderstorm." U.S. Weather Bureau.
16. **Clark, T.L. et al. (1996)**. "Coupled atmosphere-fire model." IJWF.
17. **Fromm, M. et al. (2006)**. "Pyrocumulonimbus injection of smoke to the stratosphere." JGR.
18. **McRae, R.H.D. et al. (2013)**. "Fire weather and fire danger in the 2003 Canberra fires." Aust. For.

---

## Completion Checklist

### Phase Completion

- [ ] **Phase 0:** Terrain Slope Integration 🔥 **CRITICAL GAP FIX**
  - Branch: `feature/fire-physics-phase0-terrain-slope`
  - PR: #___
  - Merged: _______________

- [ ] **Phase 1:** Vertical Fuel Layers ⭐ **FOUNDATIONAL**
  - Branch: `feature/fire-physics-phase1-vertical-fuel-layers`
  - PR: #___
  - Merged: _______________

- [ ] **Phase 2:** Fuel Heterogeneity
  - Branch: `feature/fire-physics-phase2-fuel-heterogeneity`
  - PR: #___
  - Merged: _______________

- [ ] **Phase 3:** Crown Fire Transition
  - Branch: `feature/fire-physics-phase3-crown-fire`
  - PR: #___
  - Merged: _______________

- [ ] **Phase 4:** Pyroconvection Dynamics
  - Branch: `feature/fire-physics-phase4-pyroconvection`
  - PR: #___
  - Merged: _______________

> 📋 **Advanced Phases (5-8)** tracked in [FIRE_PHYSICS_ADVANCED.md](FIRE_PHYSICS_ADVANCED.md)

### Phase 0 Checklist: Terrain Slope Integration 🔥 **CRITICAL GAP FIX**
- [ ] `elevation`, `slope`, `aspect` fields added to FieldData
- [ ] Terrain fields initialized from TerrainData during solver creation
- [ ] `compute_spread_rate_cpu()` applies slope factor
- [ ] GPU shader `level_set.wgsl` updated with slope factor
- [ ] Unit tests pass (`cargo test terrain uphill`)
- [ ] Fire spreads ~2× faster per 10° uphill
- [ ] Fire spreads slower downhill (min 30% of flat rate)
- [ ] Cross-slope spread has no slope effect

### Phase 1 Checklist: Vertical Fuel Layers ⭐ **FOUNDATIONAL**
- [ ] `FuelLayer` enum (Surface, Shrub, Canopy)
- [ ] `LayeredFuelCell` struct with per-layer properties
- [ ] Layered fuel fields in solver (fuel_load_surface, fuel_load_shrub, fuel_load_canopy)
- [ ] Per-layer fire state tracking (burning/not burning)
- [ ] Vertical heat transfer (radiation + convection upward between layers)
- [ ] Integration with existing `Fuel` struct (extract surface layer defaults)
- [ ] Unit tests pass (`cargo test fuel_layer vertical_heat`)
- [ ] Surface fire burns without affecting canopy until threshold reached

### Phase 2 Checklist: Fuel Heterogeneity
- [ ] `NoiseGenerator` implemented with simplex noise
- [ ] `HeterogeneityConfig` configuration struct
- [ ] Fuel load variation applied to each layer
- [ ] Moisture microclimate variation (aspect-based, per layer)
- [ ] Unit tests pass (`cargo test noise fuel_variation`)
- [ ] Visual validation shows increased fingering

### Phase 3 Checklist: Crown Fire Transition
- [ ] `CanopyProperties` added to `Fuel` struct
- [ ] `CrownFireState` enum (Surface, Passive, Active) per cell
- [ ] Van Wagner initiation threshold uses surface layer intensity
- [ ] Cruz et al. (2005) crown fire ROS
- [ ] Integration with level set effective ROS
- [ ] GPU shader `crown_fire.wgsl` (optional)
- [ ] Unit tests pass (`cargo test crown_fire`)
- [ ] Validation against Cruz et al. (2005) Table 2

### Phase 4 Checklist: Pyroconvection Dynamics
- [ ] `atmosphere/` module created
- [ ] `ConvectionColumn` with Byram/Briggs physics
- [ ] `AtmosphericStability` with Haines Index
- [ ] `Downdraft` with outflow physics
- [ ] `FireWhirlDetector` with vorticity calculation
- [ ] `PyroCbSystem` lifecycle management
- [ ] Wind field integration (entrainment + downdrafts)
- [ ] Unit tests pass (`cargo test atmosphere convection downdraft`)
- [ ] Visual validation of wind entrainment

### Final Verification
- [ ] All phases integrate cleanly
- [ ] `cargo clippy --all-targets --all-features` passes with ZERO warnings
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings
- [ ] `cargo test --all-features` passes
- [ ] `cargo test --no-default-features` passes
- [ ] `cargo fmt --all --check` passes
- [ ] Documentation complete

**System Complete Date:** _______________  
**Verified By:** _______________
