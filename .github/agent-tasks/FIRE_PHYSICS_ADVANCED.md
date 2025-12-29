# Fire Physics Advanced Enhancements

**Status:** Not Started  
**Priority:** **MEDIUM-LOW** (Post Phase 4 work)  
**Target:** Advanced fire behaviors for extreme realism — junction zones, VLS, valley channeling, regime detection

---

## Prerequisites

> ⚠️ **This task requires completion of [FIRE_PHYSICS_ENHANCEMENTS.md](FIRE_PHYSICS_ENHANCEMENTS.md) Phases 0-4 first.**

| Prerequisite Phase | Why Required |
|--------------------|--------------|
| **Phase 0: Terrain Slope** | Junction zones, VLS, and valley channeling all depend on terrain integration |
| **Phase 4: Pyroconvection** | Regime detection builds on convection column physics |

---

## Phases Overview

These phases were identified during the comprehensive bushfire behavior audit (29 Dec 2025).  
See full catalog: [docs/research/COMPREHENSIVE_BUSHFIRE_BEHAVIORS_CATALOG.md](../../docs/research/COMPREHENSIVE_BUSHFIRE_BEHAVIORS_CATALOG.md)

| Phase | Name | Complexity | Est. Days | Dependency | Priority |
|-------|------|------------|-----------|------------|----------|
| **5** | Junction Zone Physics | Medium | 3-5 | Phase 0 | MEDIUM |
| **6** | VLS (Vorticity Lateral Spread) | High | 5-7 | Phase 0 | MEDIUM |
| **7** | Valley Channeling | Medium | 3-4 | Phase 0 | LOW |
| **8** | Plume/Wind Regime Detection | Medium | 2-3 | Phase 4 | LOW |

**Total Estimated Time: 13-19 days**

---

## Table of Contents

1. [Critical Architecture Rules](#critical-architecture-rules-inherited)
2. [Phase 5: Junction Zone Physics](#phase-5-junction-zone-physics)
3. [Phase 6: VLS (Vorticity-Driven Lateral Spread)](#phase-6-vls-vorticity-driven-lateral-spread)
4. [Phase 7: Valley Channeling](#phase-7-valley-channeling--chimney-effect)
5. [Phase 8: Plume/Wind Regime Detection](#phase-8-plume-dominated-vs-wind-driven-regime-detection)
6. [References](#references)
7. [Completion Checklist](#completion-checklist)

---

## Critical Architecture Rules (Inherited)

- [ ] **NEVER SIMPLIFY PHYSICS** - Implement formulas exactly as published in fire science literature
- [ ] **NEVER HARDCODE DYNAMIC VALUES** - Use fuel properties, weather conditions, grid state appropriately
- [ ] **FIX ROOT CAUSES, NOT SYMPTOMS** - Investigate WHY invalid values occur, don't clamp/mask
- [ ] **PUBLIC CODE MUST BE COMMENTED** - All public APIs need documentation
- [ ] **NO ALLOW MACROS** - Fix ALL clippy warnings by changing code

---

# PHASE IMPLEMENTATION DETAILS

---

## PHASE 5: Junction Zone Physics

**Objective:** Model accelerated fire behavior when two fire fronts merge.

**Branch Name:** `feature/fire-physics-phase5-junction-zones`

**Estimated Time:** 3-5 days

**Depends On:** Phase 0 (Terrain Slope Integration)

### Phase 5 Problem Statement

When two fire fronts approach each other and merge (junction zone), the fire behavior becomes significantly more intense:
- Rate of spread can increase 2-5× at the junction point
- Combined radiant heat flux preheats fuel from both directions
- Air is entrained from both sides, creating enhanced updrafts
- This is a major factor in firefighter fatalities (Viegas et al. 2012)

Current simulation does NOT detect or model junction zone acceleration.

### Phase 5 Scientific Foundation

#### Junction Zone Acceleration (Viegas et al. 2012)

When two fire fronts converge at angle θ:

```
Acceleration factor depends on junction angle:
- θ = 0° (parallel): No acceleration (fires just merge)
- θ = 30-60°: Maximum acceleration (2-5× ROS)
- θ = 90°: Moderate acceleration
- θ = 180° (head-on): Brief intense interaction, then extinguish

Peak acceleration occurs at acute angles (< 45°) due to:
1. Combined radiant heating from both fronts
2. Converging indrafts creating enhanced convection
3. Fuel preheating from multiple directions
```

#### Junction Zone Heat Flux

```
Q_junction = Q_front1 + Q_front2 + Q_interaction

Where:
- Q_front1, Q_front2: Individual radiant heat fluxes
- Q_interaction: Additional flux from converging convection
- Q_interaction ≈ 0.5 × min(Q_front1, Q_front2) × cos(θ/2)
```

#### Junction Detection from Level Set

```
Two fire fronts are converging when:
1. Multiple disconnected φ < 0 regions exist
2. Distance between regions is decreasing
3. Normal vectors point toward each other

Junction angle θ = acos(n1 · n2)
Where n1, n2 are fire front normal vectors at closest points
```

### Phase 5 Deliverables

- [ ] `JunctionZoneDetector` struct for detecting converging fires
- [ ] `JunctionAccelerationFactor` calculation based on angle and distance
- [ ] Integration with level set spread rate
- [ ] Unit tests for junction detection and acceleration
- [ ] Integration test with two ignition points

### Phase 5 Files to Create

```
crates/core/src/solver/
├── junction_zone.rs          # Junction zone detection and physics
└── mod.rs                    # Add junction zone exports
```

### Phase 5 Implementation

#### 5.1 Junction Zone Detection

```rust
//! Junction zone fire behavior
//!
//! Implements detection and acceleration modeling for converging fire fronts.
//!
//! # Scientific References
//!
//! - Viegas, D.X. et al. (2012). "Fire behaviour and fatalities in wildfires."
//!   Int. J. Wildland Fire.
//! - Thomas, C.M. et al. (2017). "Investigation of firebrand generation from
//!   burning vegetation." Fire Safety Journal.

use crate::core_types::vec3::Vec3;

/// Detected junction zone between converging fire fronts
#[derive(Debug, Clone)]
pub struct JunctionZone {
    /// Position of junction point
    pub position: Vec3,
    /// Angle between converging fronts (radians)
    pub angle: f32,
    /// Distance between fronts (m)
    pub distance: f32,
    /// Estimated time to contact (s)
    pub time_to_contact: f32,
    /// Acceleration factor to apply
    pub acceleration_factor: f32,
}

/// Detector for junction zone conditions
pub struct JunctionZoneDetector {
    /// Minimum distance to consider as potential junction (m)
    pub detection_distance: f32,
    /// Minimum angle for junction acceleration (radians)
    pub min_angle: f32,
}

impl Default for JunctionZoneDetector {
    fn default() -> Self {
        Self {
            detection_distance: 100.0,  // Detect junctions within 100m
            min_angle: 0.1,             // ~6° minimum angle
        }
    }
}

impl JunctionZoneDetector {
    /// Detect junction zones from level set field
    ///
    /// Analyzes the level set to find regions where:
    /// 1. Two separate fire fronts (φ = 0 contours) exist
    /// 2. They are approaching each other
    /// 3. The junction angle is acute enough for acceleration
    pub fn detect(
        &self,
        phi: &[f32],
        width: usize,
        height: usize,
        cell_size: f32,
        dt: f32,
    ) -> Vec<JunctionZone> {
        let mut junctions = Vec::new();
        
        // Find fire front cells (φ ≈ 0 with φ < 0 neighbors)
        let front_cells = self.extract_fire_front_cells(phi, width, height);
        
        // Group into connected components (separate fire fronts)
        let components = self.find_connected_components(&front_cells, width, height);
        
        // For each pair of components, check for junction conditions
        for i in 0..components.len() {
            for j in (i + 1)..components.len() {
                if let Some(junction) = self.analyze_junction(
                    &components[i],
                    &components[j],
                    phi,
                    width,
                    height,
                    cell_size,
                    dt,
                ) {
                    junctions.push(junction);
                }
            }
        }
        
        junctions
    }
    
    /// Extract cells on fire front (φ ≈ 0)
    fn extract_fire_front_cells(
        &self,
        phi: &[f32],
        width: usize,
        height: usize,
    ) -> Vec<(usize, usize)> {
        let mut front_cells = Vec::new();
        
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let idx = y * width + x;
                let p = phi[idx];
                
                // Cell is on front if φ is small and has sign change with neighbor
                if p.abs() < 2.0 {
                    let has_positive = phi[idx - 1] > 0.0 
                        || phi[idx + 1] > 0.0
                        || phi[idx - width] > 0.0 
                        || phi[idx + width] > 0.0;
                    let has_negative = phi[idx - 1] < 0.0 
                        || phi[idx + 1] < 0.0
                        || phi[idx - width] < 0.0 
                        || phi[idx + width] < 0.0;
                    
                    if has_positive && has_negative {
                        front_cells.push((x, y));
                    }
                }
            }
        }
        
        front_cells
    }
    
    /// Group front cells into connected components
    fn find_connected_components(
        &self,
        front_cells: &[(usize, usize)],
        width: usize,
        height: usize,
    ) -> Vec<Vec<(usize, usize)>> {
        use std::collections::HashSet;
        
        let mut remaining: HashSet<_> = front_cells.iter().copied().collect();
        let mut components = Vec::new();
        
        while let Some(&start) = remaining.iter().next() {
            let mut component = Vec::new();
            let mut stack = vec![start];
            
            while let Some(cell) = stack.pop() {
                if remaining.remove(&cell) {
                    component.push(cell);
                    
                    // Check 8-connected neighbors
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            if dx == 0 && dy == 0 { continue; }
                            let nx = (cell.0 as i32 + dx) as usize;
                            let ny = (cell.1 as i32 + dy) as usize;
                            if nx < width && ny < height && remaining.contains(&(nx, ny)) {
                                stack.push((nx, ny));
                            }
                        }
                    }
                }
            }
            
            if !component.is_empty() {
                components.push(component);
            }
        }
        
        components
    }
    
    /// Analyze potential junction between two fire front components
    fn analyze_junction(
        &self,
        front1: &[(usize, usize)],
        front2: &[(usize, usize)],
        phi: &[f32],
        width: usize,
        height: usize,
        cell_size: f32,
        dt: f32,
    ) -> Option<JunctionZone> {
        // Find closest points between the two fronts
        let mut min_dist = f32::MAX;
        let mut closest1 = (0, 0);
        let mut closest2 = (0, 0);
        
        for &(x1, y1) in front1 {
            for &(x2, y2) in front2 {
                let dx = x2 as f32 - x1 as f32;
                let dy = y2 as f32 - y1 as f32;
                let dist = (dx * dx + dy * dy).sqrt() * cell_size;
                
                if dist < min_dist {
                    min_dist = dist;
                    closest1 = (x1, y1);
                    closest2 = (x2, y2);
                }
            }
        }
        
        // Only consider junctions within detection distance
        if min_dist > self.detection_distance {
            return None;
        }
        
        // Calculate fire front normals at closest points
        let n1 = self.calculate_normal(phi, closest1.0, closest1.1, width, height, cell_size);
        let n2 = self.calculate_normal(phi, closest2.0, closest2.1, width, height, cell_size);
        
        // Check if fronts are converging (normals point toward each other)
        let to_front2 = Vec3::new(
            (closest2.0 as f32 - closest1.0 as f32) * cell_size,
            (closest2.1 as f32 - closest1.1 as f32) * cell_size,
            0.0,
        ).normalize();
        
        let converging1 = n1.dot(&to_front2) > 0.0;
        let converging2 = n2.dot(&(-to_front2)) > 0.0;
        
        if !converging1 || !converging2 {
            return None;  // Fronts not converging
        }
        
        // Calculate junction angle
        let angle = n1.dot(&(-n2)).acos();
        
        if angle < self.min_angle {
            return None;  // Angle too small
        }
        
        // Estimate spread rates to calculate time to contact
        // Using simple estimate based on level set gradient
        let spread_rate = 0.5;  // m/s estimate, should use actual ROS field
        let time_to_contact = min_dist / (2.0 * spread_rate);
        
        // Calculate acceleration factor
        let acceleration = self.calculate_acceleration_factor(angle, min_dist);
        
        let position = Vec3::new(
            (closest1.0 as f32 + closest2.0 as f32) * 0.5 * cell_size,
            (closest1.1 as f32 + closest2.1 as f32) * 0.5 * cell_size,
            0.0,
        );
        
        Some(JunctionZone {
            position,
            angle,
            distance: min_dist,
            time_to_contact,
            acceleration_factor: acceleration,
        })
    }
    
    /// Calculate fire front normal from level set gradient
    fn calculate_normal(
        &self,
        phi: &[f32],
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        cell_size: f32,
    ) -> Vec3 {
        let idx = y * width + x;
        
        // Central differences for gradient
        let dx = if x > 0 && x < width - 1 {
            (phi[idx + 1] - phi[idx - 1]) / (2.0 * cell_size)
        } else {
            0.0
        };
        
        let dy = if y > 0 && y < height - 1 {
            (phi[idx + width] - phi[idx - width]) / (2.0 * cell_size)
        } else {
            0.0
        };
        
        let mag = (dx * dx + dy * dy).sqrt().max(1e-6);
        Vec3::new(dx / mag, dy / mag, 0.0)
    }
    
    /// Calculate acceleration factor based on junction geometry
    ///
    /// Based on Viegas et al. (2012):
    /// - Maximum acceleration at acute angles (30-60°)
    /// - Factor increases as distance decreases
    /// - Peak factor of 2-5× observed in field studies
    fn calculate_acceleration_factor(&self, angle: f32, distance: f32) -> f32 {
        // Angle effect: peak at ~45° (π/4 radians)
        let angle_factor = if angle < std::f32::consts::FRAC_PI_4 {
            // Below 45°: increasing effect
            1.0 + 3.0 * (angle / std::f32::consts::FRAC_PI_4)
        } else if angle < std::f32::consts::FRAC_PI_2 {
            // 45-90°: decreasing effect
            4.0 - 3.0 * (angle - std::f32::consts::FRAC_PI_4) / std::f32::consts::FRAC_PI_4
        } else {
            // > 90°: minimal effect
            1.0
        };
        
        // Distance effect: stronger as fronts approach
        let distance_factor = (1.0 - distance / self.detection_distance).max(0.0);
        
        // Combined: interpolate toward peak as distance closes
        1.0 + (angle_factor - 1.0) * distance_factor
    }
}
```

#### 5.2 Integration with Level Set

```rust
// In solver/level_set.rs or simulation update

impl FieldSolver {
    /// Apply junction zone acceleration to spread rates
    pub fn apply_junction_acceleration(
        &mut self,
        junctions: &[JunctionZone],
        cell_size: f32,
    ) {
        for junction in junctions {
            // Apply acceleration in a radius around junction point
            let radius = junction.distance * 0.5;
            let center_x = (junction.position.x / cell_size) as usize;
            let center_y = (junction.position.y / cell_size) as usize;
            
            let radius_cells = (radius / cell_size).ceil() as i32;
            
            for dy in -radius_cells..=radius_cells {
                for dx in -radius_cells..=radius_cells {
                    let x = (center_x as i32 + dx) as usize;
                    let y = (center_y as i32 + dy) as usize;
                    
                    if x >= self.width || y >= self.height {
                        continue;
                    }
                    
                    let dist = ((dx * dx + dy * dy) as f32).sqrt() * cell_size;
                    if dist > radius {
                        continue;
                    }
                    
                    // Acceleration falls off with distance from junction center
                    let falloff = 1.0 - dist / radius;
                    let local_acceleration = 1.0 + (junction.acceleration_factor - 1.0) * falloff;
                    
                    let idx = y * self.width + x;
                    self.spread_rate[idx] *= local_acceleration;
                }
            }
        }
    }
}
```

### Phase 5 Validation Criteria

- [ ] `cargo build -p fire-sim-core --no-default-features` compiles
- [ ] `cargo test -p fire-sim-core junction_zone` passes
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings
- [ ] `cargo fmt --all --check` passes

### Phase 5 Testing

- [ ] Unit test: `detect_two_converging_fronts` - detects junction when two fires approach
- [ ] Unit test: `no_junction_for_parallel_fronts` - parallel fires don't trigger junction
- [ ] Unit test: `acceleration_factor_peaks_at_45_degrees` - maximum at acute angles
- [ ] Unit test: `acceleration_increases_as_distance_decreases` - proximity effect
- [ ] Integration test: Two ignition points create junction with measurable acceleration

---

## PHASE 6: VLS (Vorticity-Driven Lateral Spread)

**Objective:** Model lateral fire spread on steep lee slopes driven by wind-terrain vorticity.

**Branch Name:** `feature/fire-physics-phase6-vls`

**Estimated Time:** 5-7 days

**Depends On:** Phase 0 (Terrain Slope Integration)

### Phase 6 Problem Statement

VLS (Vorticity-driven Lateral Spread) was identified during the 2003 Canberra fires as a major mechanism for rapid fire runs on steep terrain:

- Fire spreads **laterally** along steep lee slopes instead of uphill
- Caused by horizontal vortices formed when wind flows over ridges
- Creates rapid, unexpected fire runs across slopes
- Responsible for firefighter fatalities and home losses

Current simulation does NOT detect or model VLS conditions.

### Phase 6 Scientific Foundation

#### VLS Conditions (Sharples et al. 2012)

VLS occurs when:
```
1. Slope angle > 20° (lee slope facing away from wind)
2. Wind speed > 5 m/s at 10m height
3. Wind direction approximately perpendicular to ridge
4. Fire is near ridge crest or upper lee slope

VLS indicator:
χ = tan(θ) × sin(|aspect - wind_dir|) × U / U_ref

Where:
- θ: Slope angle
- aspect: Slope aspect (direction slope faces)
- wind_dir: Wind direction
- U: Wind speed
- U_ref: Reference wind speed (5 m/s)

VLS likely when χ > 0.6
```

#### Lateral Spread Enhancement

```
When VLS conditions are met:
- Spread direction shifts from upslope to lateral (along contour)
- Spread rate enhancement: 2-3× normal rate
- Effect strongest on upper third of lee slope
```

### Phase 6 Deliverables

- [ ] `VLSDetector` struct for detecting VLS-prone areas
- [ ] `calculate_vls_index()` for VLS likelihood
- [ ] Spread direction modification under VLS conditions
- [ ] Integration with level set evolution
- [ ] Unit tests for VLS detection

### Phase 6 Files to Create

```
crates/core/src/solver/
├── vls.rs                    # VLS detection and physics
└── mod.rs                    # Add VLS exports
```

### Phase 6 Implementation

```rust
//! Vorticity-Driven Lateral Spread (VLS)
//!
//! Implements detection and modeling of lateral fire spread on steep lee slopes.
//!
//! # Scientific References
//!
//! - Sharples, J.J. et al. (2012). "Wind-terrain effects on the propagation of
//!   wildfires in rugged terrain: fire channelling." Int. J. Wildland Fire 21:282-296.
//! - Simpson, C.C. et al. (2013). "Resolving vorticity-driven lateral fire spread."
//!   Int. J. Wildland Fire.

use crate::core_types::vec3::Vec3;
use crate::grid::TerrainData;

/// VLS detection parameters
pub struct VLSDetector {
    /// Minimum slope angle for VLS (degrees)
    pub min_slope: f32,
    /// Minimum wind speed for VLS (m/s)
    pub min_wind_speed: f32,
    /// VLS index threshold
    pub vls_threshold: f32,
}

impl Default for VLSDetector {
    fn default() -> Self {
        Self {
            min_slope: 20.0,        // 20° minimum slope
            min_wind_speed: 5.0,    // 5 m/s minimum wind
            vls_threshold: 0.6,     // χ > 0.6 indicates VLS
        }
    }
}

/// VLS conditions at a point
#[derive(Debug, Clone, Copy)]
pub struct VLSCondition {
    /// VLS index (χ)
    pub vls_index: f32,
    /// Whether VLS is active
    pub is_active: bool,
    /// Lateral spread direction (radians)
    pub lateral_direction: f32,
    /// Spread rate multiplier
    pub rate_multiplier: f32,
}

impl VLSDetector {
    /// Calculate VLS index at a position
    ///
    /// χ = tan(θ) × sin(|aspect - wind_dir|) × U / U_ref
    pub fn calculate_vls_index(
        &self,
        slope_degrees: f32,
        aspect_degrees: f32,
        wind_direction_degrees: f32,
        wind_speed: f32,
    ) -> f32 {
        if slope_degrees < self.min_slope || wind_speed < self.min_wind_speed {
            return 0.0;
        }
        
        let slope_rad = slope_degrees.to_radians();
        let tan_slope = slope_rad.tan();
        
        // Angular difference between aspect and wind direction
        let angle_diff = (aspect_degrees - wind_direction_degrees).to_radians();
        let sin_diff = angle_diff.sin().abs();
        
        // Wind factor
        let wind_factor = wind_speed / self.min_wind_speed;
        
        tan_slope * sin_diff * wind_factor
    }
    
    /// Detect VLS conditions across the terrain
    pub fn detect(
        &self,
        terrain: &TerrainData,
        wind: Vec3,
        width: usize,
        height: usize,
        cell_size: f32,
    ) -> Vec<Vec<VLSCondition>> {
        let wind_speed = (wind.x * wind.x + wind.y * wind.y).sqrt();
        let wind_dir = wind.y.atan2(wind.x).to_degrees();
        
        let mut conditions = vec![vec![VLSCondition::default(); width]; height];
        
        for y in 0..height {
            for x in 0..width {
                let world_x = x as f32 * cell_size;
                let world_y = y as f32 * cell_size;
                
                let slope = *terrain.slope_at_horn(world_x, world_y);
                let aspect = *terrain.aspect_at_horn(world_x, world_y);
                
                // Check if this is a lee slope (wind coming from opposite direction)
                let is_lee_slope = self.is_lee_slope(aspect, wind_dir);
                
                if !is_lee_slope {
                    conditions[y][x] = VLSCondition {
                        vls_index: 0.0,
                        is_active: false,
                        lateral_direction: 0.0,
                        rate_multiplier: 1.0,
                    };
                    continue;
                }
                
                let vls_index = self.calculate_vls_index(slope, aspect, wind_dir, wind_speed);
                let is_active = vls_index > self.vls_threshold;
                
                // Lateral direction is perpendicular to aspect (along contour)
                let lateral_direction = (aspect + 90.0) % 360.0;
                
                // Rate multiplier: 1.0 to 3.0 based on VLS index
                let rate_multiplier = if is_active {
                    1.0 + 2.0 * (vls_index - self.vls_threshold).min(1.0)
                } else {
                    1.0
                };
                
                conditions[y][x] = VLSCondition {
                    vls_index,
                    is_active,
                    lateral_direction: lateral_direction.to_radians(),
                    rate_multiplier,
                };
            }
        }
        
        conditions
    }
    
    /// Check if slope is on lee side of wind
    fn is_lee_slope(&self, aspect: f32, wind_direction: f32) -> bool {
        // Lee slope faces away from wind (aspect roughly opposite to wind direction)
        let angle_diff = ((aspect - wind_direction + 180.0) % 360.0 - 180.0).abs();
        angle_diff < 60.0  // Within 60° of downwind
    }
}

impl Default for VLSCondition {
    fn default() -> Self {
        Self {
            vls_index: 0.0,
            is_active: false,
            lateral_direction: 0.0,
            rate_multiplier: 1.0,
        }
    }
}
```

### Phase 6 Integration with Level Set

```rust
// In level set spread rate calculation

impl FieldSolver {
    /// Modify spread direction and rate for VLS
    pub fn apply_vls_effects(
        &mut self,
        vls_conditions: &[Vec<VLSCondition>],
    ) {
        for y in 0..self.height {
            for x in 0..self.width {
                let vls = &vls_conditions[y][x];
                
                if !vls.is_active {
                    continue;
                }
                
                let idx = y * self.width + x;
                
                // Apply rate multiplier
                self.spread_rate[idx] *= vls.rate_multiplier;
                
                // Modify spread direction toward lateral
                // Blend between normal direction and lateral based on VLS strength
                let blend = (vls.vls_index - 0.6).min(0.4) / 0.4;  // 0 to 1
                // Direction modification would be done in the level set velocity field
            }
        }
    }
}
```

### Phase 6 Validation Criteria

- [ ] `cargo build -p fire-sim-core --no-default-features` compiles
- [ ] `cargo test -p fire-sim-core vls` passes
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings

### Phase 6 Testing

- [ ] Unit test: `vls_index_zero_for_flat_terrain` - no VLS on flat ground
- [ ] Unit test: `vls_index_zero_for_windward_slope` - no VLS on upwind slopes
- [ ] Unit test: `vls_index_high_for_steep_lee_slope` - VLS detected correctly
- [ ] Unit test: `lateral_direction_perpendicular_to_aspect` - direction is along contour
- [ ] Integration test: Fire on lee slope spreads laterally under strong wind

---

## PHASE 7: Valley Channeling / Chimney Effect

**Objective:** Model accelerated fire spread in confined valleys due to wind channeling and heat accumulation.

**Branch Name:** `feature/fire-physics-phase7-valley-channeling`

**Estimated Time:** 3-4 days

**Depends On:** Phase 0 (Terrain Slope Integration)

### Phase 7 Problem Statement

Valleys create dangerous fire conditions:
- Wind is funneled and accelerated through narrow valleys
- Heat radiating from both valley walls preheats fuel in the center
- Strong updrafts at valley head ("chimney effect")
- Fire can race up valleys at extreme speeds

Current simulation does NOT detect valley geometry or apply these effects.

### Phase 7 Scientific Foundation

#### Valley Wind Acceleration

```
Wind speed in valley:
U_valley = U_ambient × (W_open / W_valley)^0.5

Where:
- W_open: Width of open terrain
- W_valley: Valley width at position
- Acceleration typically 1.5-2.5×
```

#### Cross-Valley Radiant Heat

```
Heat flux from opposite wall:
Q_cross = (σ × T_wall^4 × F_12) × 2

Where:
- F_12: View factor between valley walls
- Factor of 2 for both walls contributing
- Significant when valley width < 100m
```

#### Chimney Effect

```
Updraft velocity at valley head:
w_chimney = sqrt(2 × g × H × ΔT / T_ambient)

Where:
- H: Valley depth
- ΔT: Temperature excess from fire
- Creates strong lofting at valley terminus
```

### Phase 7 Deliverables

- [ ] `ValleyDetector` for identifying valley geometry
- [ ] `ValleyAcceleration` for wind and heat effects
- [ ] Integration with wind field and heat transfer
- [ ] Unit tests for valley detection

### Phase 7 Implementation (Outline)

```rust
//! Valley channeling and chimney effect
//!
//! Detects valley geometry and applies fire behavior modifications.

/// Valley geometry at a position
pub struct ValleyGeometry {
    /// Valley width (m)
    pub width: f32,
    /// Valley depth (m)  
    pub depth: f32,
    /// Valley orientation (radians)
    pub orientation: f32,
    /// Distance from valley head (m)
    pub distance_from_head: f32,
    /// Is this position in a valley?
    pub in_valley: bool,
}

/// Detect valley geometry from terrain
pub fn detect_valley_geometry(
    terrain: &TerrainData,
    x: f32,
    y: f32,
) -> ValleyGeometry {
    // Implementation would use terrain analysis:
    // 1. Sample elevation in radial pattern
    // 2. Identify if surrounded by higher terrain
    // 3. Calculate valley width from distance to ridges
    // 4. Calculate valley orientation from lowest path
    todo!()
}

/// Calculate wind acceleration in valley
pub fn valley_wind_factor(geometry: &ValleyGeometry, reference_width: f32) -> f32 {
    if !geometry.in_valley {
        return 1.0;
    }
    
    (reference_width / geometry.width).sqrt().clamp(1.0, 2.5)
}

/// Calculate chimney updraft velocity
pub fn chimney_updraft(
    geometry: &ValleyGeometry,
    fire_temperature: f32,
    ambient_temperature: f32,
) -> f32 {
    if !geometry.in_valley || geometry.distance_from_head > 100.0 {
        return 0.0;
    }
    
    let delta_t = fire_temperature - ambient_temperature;
    let g = 9.81;
    
    (2.0 * g * geometry.depth * delta_t / (ambient_temperature + 273.15)).sqrt()
}
```

### Phase 7 Validation Criteria

- [ ] `cargo build -p fire-sim-core --no-default-features` compiles
- [ ] `cargo test -p fire-sim-core valley` passes
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings

---

## PHASE 8: Plume-Dominated vs Wind-Driven Regime Detection

**Objective:** Detect and model transitions between plume-dominated and wind-driven fire regimes.

**Branch Name:** `feature/fire-physics-phase8-regime-detection`

**Estimated Time:** 2-3 days

**Depends On:** Phase 4 (Pyroconvection Dynamics)

### Phase 8 Problem Statement

Fires operate in two fundamental regimes:
1. **Wind-Driven:** Ambient wind controls fire behavior (predictable)
2. **Plume-Dominated:** Fire's convection controls behavior (erratic)

The transition between regimes is particularly dangerous because:
- Fire direction can change suddenly
- Spread rate can accelerate unpredictably
- Standard fire behavior predictions become unreliable

### Phase 8 Scientific Foundation

#### Regime Discrimination (Byram Number)

```
Byram Number (N_c):
N_c = (2 × g × I) / (ρ × c_p × T × U³)

Where:
- g: Gravity (9.81 m/s²)
- I: Fire intensity (W/m)
- ρ: Air density (kg/m³)
- c_p: Specific heat (J/kg·K)
- T: Ambient temperature (K)
- U: Wind speed (m/s)

Interpretation:
- N_c < 1: Wind-driven regime
- N_c > 10: Plume-dominated regime
- 1 < N_c < 10: Transitional (most dangerous)
```

### Phase 8 Implementation (Outline)

```rust
//! Fire regime detection
//!
//! Detects whether fire is wind-driven or plume-dominated.

/// Fire behavior regime
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireRegime {
    /// Wind controls fire direction and spread
    WindDriven,
    /// Fire convection controls behavior
    PlumeDominated,
    /// Transitional - most dangerous
    Transitional,
}

/// Calculate Byram number for regime detection
pub fn byram_number(
    fire_intensity: f32,  // W/m
    wind_speed: f32,      // m/s
    ambient_temp: f32,    // °C
) -> f32 {
    const G: f32 = 9.81;
    const RHO: f32 = 1.225;
    const CP: f32 = 1005.0;
    
    let t_kelvin = ambient_temp + 273.15;
    let u_cubed = (wind_speed.max(0.5)).powi(3);
    
    (2.0 * G * fire_intensity) / (RHO * CP * t_kelvin * u_cubed)
}

/// Determine fire regime from conditions
pub fn detect_regime(
    fire_intensity: f32,
    wind_speed: f32,
    ambient_temp: f32,
) -> FireRegime {
    let nc = byram_number(fire_intensity, wind_speed, ambient_temp);
    
    if nc < 1.0 {
        FireRegime::WindDriven
    } else if nc > 10.0 {
        FireRegime::PlumeDominated
    } else {
        FireRegime::Transitional
    }
}

/// Get spread direction uncertainty for regime
pub fn direction_uncertainty(regime: FireRegime) -> f32 {
    match regime {
        FireRegime::WindDriven => 15.0,      // ±15° uncertainty
        FireRegime::Transitional => 60.0,    // ±60° uncertainty
        FireRegime::PlumeDominated => 180.0, // Can go any direction
    }
}
```

### Phase 8 Validation Criteria

- [ ] `cargo build -p fire-sim-core --no-default-features` compiles
- [ ] `cargo test -p fire-sim-core regime` passes
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings

---

## References

### Junction Zones
1. **Viegas, D.X. et al. (2012)**. "Fire behaviour and fatalities in wildfires." Int. J. Wildland Fire.
2. **Thomas, C.M. et al. (2017)**. "Investigation of firebrand generation." Fire Safety Journal.

### VLS (Vorticity-Driven Lateral Spread)
3. **Sharples, J.J. et al. (2012)**. "Wind-terrain effects on the propagation of wildfires in rugged terrain: fire channelling." Int. J. Wildland Fire 21:282-296.
4. **Simpson, C.C. et al. (2013)**. "Resolving vorticity-driven lateral fire spread." Int. J. Wildland Fire.
5. **McRae, R.H.D. et al. (2015)**. "An Australian pyro-tornadogenesis event." Natural Hazards and Earth System Sciences.

### Valley Channeling
6. **Butler, B.W. et al. (1998)**. "Fire behavior associated with the 1994 South Canyon Fire." USDA Forest Service Research Paper RMRS-RP-9.
7. **Sharples, J.J. (2009)**. "An overview of mountain meteorological effects relevant to fire behaviour and bushfire risk." Int. J. Wildland Fire 18:737-754.

### Regime Detection
8. **Byram, G.M. (1959)**. "Combustion of forest fuels." Forest Fires: Control and Use.
9. **Nelson, R.M. (2003)**. "Power of the fire—a thermodynamic analysis." Int. J. Wildland Fire 12:51-65.
10. **Finney, M.A. & McAllister, S.S. (2011)**. "A review of fire interactions and mass fires." J. Combustion.

---

## Completion Checklist

### Phase Completion

- [ ] **Phase 5:** Junction Zone Physics
  - Branch: `feature/fire-physics-phase5-junction-zones`
  - PR: #___
  - Merged: _______________

- [ ] **Phase 6:** VLS (Vorticity-Driven Lateral Spread)
  - Branch: `feature/fire-physics-phase6-vls`
  - PR: #___
  - Merged: _______________

- [ ] **Phase 7:** Valley Channeling / Chimney Effect
  - Branch: `feature/fire-physics-phase7-valley-channeling`
  - PR: #___
  - Merged: _______________

- [ ] **Phase 8:** Plume/Wind Regime Detection
  - Branch: `feature/fire-physics-phase8-regime-detection`
  - PR: #___
  - Merged: _______________

### Final Verification

- [ ] All phases integrate cleanly with FIRE_PHYSICS_ENHANCEMENTS.md phases
- [ ] `cargo clippy --all-targets --all-features` passes with ZERO warnings
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings
- [ ] `cargo test --all-features` passes
- [ ] `cargo test --no-default-features` passes
- [ ] `cargo fmt --all --check` passes
- [ ] Documentation complete

**System Complete Date:** _______________  
**Verified By:** _______________
