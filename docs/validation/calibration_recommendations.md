# Calibration Recommendations

**Document:** Phase 3 - Parameters Needing Adjustment Based on Validation  
**Standard:** Professional Fire Behavior Model Calibration  
**Date:** December 2024  

---

## Executive Summary

This document identifies parameters and model features that require adjustment or enhancement based on validation results. It provides scientific justification and implementation guidance for each recommendation.

### Priority Classifications

| Priority | Definition | Action Required |
|----------|------------|-----------------|
| **Critical** | Affects core physics accuracy | Implement before production use |
| **High** | Significant impact on realism | Implement in next development cycle |
| **Medium** | Noticeable improvement to behavior | Schedule for future enhancement |
| **Low** | Minor refinement | Consider for long-term roadmap |

---

## 1. Critical Priority Recommendations

### 1.1 Fire Spread Stochasticity (Perimeter Irregularity)

**Issue:** Fire spread appears too uniform/circular, even under wind conditions that should create irregular, elongated fire shapes.

**Current Behavior:**
- At 25 km/h (6.9 m/s) wind: Fire spreads in a near-circular pattern
- Wind asymmetry creates ~3:1 downwind:upwind ratio, but perimeter remains smooth
- Real fires have fractal-like, irregular perimeters

**Root Cause Analysis:**
1. **Fuel is uniform** - Demo places identical fuel elements in a grid pattern
   - Same fuel type (dry_grass everywhere)
   - Same mass (3.0 kg)
   - Same moisture (uniform from weather preset)
   - No spatial variation in fuel properties

2. **Heat transfer is deterministic** - While ignition uses `rand::random()`, heat accumulation is purely deterministic via Stefan-Boltzmann + wind multipliers

3. **No turbulent wind fluctuation** - Wind is steady direction and speed without gusting or direction wobble

4. **Grid placement** - Regular 1m spacing creates uniform neighbor distances

**Scientific Basis:**

Real fire spread irregularity comes from:

| Factor | Real Fire | Current Implementation |
|--------|-----------|------------------------|
| Fuel load | Varies 50-300% spatially | Uniform (configurable but not default) |
| Moisture content | Patchy 5-25% | Uniform from weather |
| Wind | Gusty ±20-40% with ±15-30° direction wobble | Steady |
| Fine fuel distribution | Clumpy/irregular | Regular grid |
| Ignition timing | Stochastic at perimeter | Probabilistic but spatially uniform |
| Local turbulence | Eddies and vortices | Not modeled |

**References:**
- Finney, M.A. (2003) "Calculation of fire spread rates across random landscapes" - IJWF
- Albini, F.A. (1976) "Estimating wildfire behavior and effects" - USDA INT-30
- Anderson, H.E. (1982) "Aids to determining fuel models" - USDA INT-122
- Richards, G.D. (1995) "A general mathematical framework for modeling two-dimensional wildland fire spread"

**Recommendations:**

#### 1.1.1 Add Fuel Property Spatial Variation (HIGH PRIORITY)

When creating fuel elements, apply Perlin noise or random variation to:

```rust
// Example implementation concept
let base_moisture = weather.get_fuel_moisture();
let moisture_variation = perlin_noise(x, y) * 0.1; // ±10%
element.moisture_fraction = base_moisture * (1.0 + moisture_variation);

let base_load = 3.0; // kg
let load_variation = perlin_noise(x * 0.5, y * 0.5) * 0.5; // ±50%
element.fuel_remaining = base_load * (1.0 + load_variation);
```

**Expected Impact:** Fire perimeter becomes irregular based on local fuel conditions

#### 1.1.2 Add Turbulent Wind Fluctuation (HIGH PRIORITY)

Implement wind gusting with direction wobble:

```rust
// Wind fluctuation model
struct TurbulentWind {
    base_speed: f32,
    base_direction: f32,
    gust_intensity: f32,      // 0.0-0.4 (0-40% variation)
    direction_wobble: f32,    // 0.0-30.0 (degrees)
    update_frequency: f32,    // How often gusts change (0.5-2.0 seconds)
}

// Each timestep:
let speed_factor = 1.0 + gust_intensity * (perlin_noise(time) * 2.0 - 1.0);
let direction_offset = direction_wobble * perlin_noise(time * 0.7);
current_wind = base_wind * speed_factor rotated by direction_offset;
```

**References:**
- Byram, G.M. (1954) "Atmospheric conditions related to blowup fires"
- Schroeder, M.J. & Buck, C.C. (1970) "Fire weather" - USDA Handbook 360

**Expected Impact:** Fire spread becomes locally variable as gusts push flames in shifting directions

#### 1.1.3 Add Ignition Delay Variation (MEDIUM PRIORITY)

Add small random delays to heat-based ignition timing:

```rust
// In check_ignition_probability()
let base_prob = calculate_ignition_probability(temp, moisture);
let noise_factor = 0.8 + 0.4 * rand::random::<f32>(); // 0.8-1.2x
let adjusted_prob = base_prob * noise_factor;
```

**Expected Impact:** Fire perimeter becomes less smooth as some elements ignite slightly faster/slower

---

## 2. High Priority Recommendations

### 2.1 Wind Effect Scaling at Moderate Speeds

**Issue:** Wind asymmetry at 25-50 km/h produces less elongated fires than observed in real events.

**Current Implementation:**
```rust
// At 6.9 m/s (25 km/h):
downwind_multiplier = 1.0 + 1.0 × sqrt(6.9) × 0.8 ≈ 3.1×
upwind_suppression = exp(-6.9 × 0.35) ≈ 0.09, clamped to 0.05
// Ratio: ~62:1
```

**Observation:** While the ratio is correct, the absolute spread rate difference may not create sufficient visible elongation in the time frames observed.

**Recommendation:** Consider:
1. Increase base wind coefficient from 0.8 to 1.0-1.2 for stronger elongation
2. Add fuel-type-specific wind sensitivity (grass > forest)
3. Document expected length:width ratios at different wind speeds

**References:**
- McArthur, A.G. (1967) "Fire behaviour in eucalypt forests"
- Anderson, H.E. (1983) "Predicting wind-driven wildland fire size and shape" - USDA INT-305

### 2.2 FFDI Scaling Threshold

**Issue:** Current FFDI multiplier may over-amplify at lower FFDI values and under-amplify at extreme values.

**Current Implementation:**
- FFDI 0-12: 0.5× to 1.0×
- FFDI 12-50: 1.0× to 4.0×
- FFDI 50-100: 4.0× to 8.0×
- FFDI 100+: 8.0× capped

**Recommendation:** 
- Review scaling curve against CSIRO fire behavior data
- Consider logarithmic rather than linear interpolation in mid-ranges
- Validate against documented spread rates at known FFDI values

### 2.3 Ember Transport Probability

**Issue:** Ember ignition success rate may be too low at extreme distances.

**Current Implementation:** 
- Landing ignition probability decreases with distance
- Maximum validated spotting: 25 km (Black Saturday)

**Recommendation:**
- Increase base ignition probability for embers landing in very dry fuels (< 5% moisture)
- Add spot fire coalescence modeling for multiple nearby landings
- Consider firebrand mass-dependent ignition success

---

## 3. Medium Priority Recommendations

### 3.1 Vertical Spread Enhancement

**Issue:** Vertical spread factor of 1.8× to 2.5× may be conservative for eucalyptus forests.

**Scientific Basis:**
- Eucalyptus stringybark creates ladder fuels with continuous vertical fuel
- Crown fire transition can be near-instantaneous once ladder fuels ignite
- Van Wagner critical intensity thresholds may need fuel-specific calibration

**Recommendation:**
- Add fuel-type-specific vertical spread multipliers
- Eucalyptus stringybark: 2.5-3.5×
- Other eucalyptus: 2.0-2.5×
- Non-eucalyptus: 1.5-2.0×

### 3.2 Diurnal Moisture Response

**Issue:** Fuel moisture response to diurnal humidity changes may be too fast.

**Current Implementation:** Nelson timelag model with 1-hour, 10-hour, 100-hour classes

**Observation:** Fine fuel moisture may equilibrate faster than modeled during afternoon fire conditions.

**Recommendation:**
- Validate Nelson EMC calculations against Australian field measurements
- Consider adding "fire-accelerated drying" near flame zone
- Document expected moisture trajectories over 24-hour cycle

### 3.3 Smoldering to Flaming Transition

**Issue:** Transition from smoldering to flaming combustion may need refinement.

**Scientific Basis:**
- Rein (2009) model implemented but thresholds are empirical
- Australian fuel types may have different transition characteristics
- Eucalyptus oil presence affects transition dynamics

**Recommendation:**
- Collect field data on smoldering/flaming transitions in Australian fuels
- Calibrate oxygen and temperature thresholds per fuel type
- Add eucalyptus oil volatilization effect on transition

---

## 4. Low Priority Recommendations

### 4.1 Atmospheric Oxygen Depletion

**Issue:** Local oxygen depletion near flame zone may be over- or under-estimated.

**Recommendation:** Validate against fire tunnel experiments if data available.

### 4.2 Radiation View Factor Refinement

**Issue:** Planar radiator assumption may not capture flame geometry variations.

**Recommendation:** Consider fuel-type-specific flame shape models for view factor calculations.

### 4.3 Cooling Rate Calibration

**Issue:** Post-fire cooling rates may differ by fuel type.

**Recommendation:** Add fuel-specific cooling rate multipliers based on ash depth and residual fuel.

---

## 5. Implementation Priority Matrix

| Recommendation | Priority | Effort | Impact | Suggested Phase |
|---------------|----------|--------|--------|-----------------|
| 1.1.1 Fuel spatial variation | Critical | Medium | High | Next release |
| 1.1.2 Turbulent wind | Critical | Medium | High | Next release |
| 1.1.3 Ignition delay variation | Medium | Low | Medium | Next release |
| 2.1 Wind scaling | High | Low | Medium | Next release |
| 2.2 FFDI scaling | High | Medium | Medium | v1.1 |
| 2.3 Ember probability | High | Low | Medium | v1.1 |
| 3.1 Vertical spread | Medium | Low | Low | v1.2 |
| 3.2 Diurnal moisture | Medium | Medium | Medium | v1.2 |
| 3.3 Smoldering transition | Medium | Medium | Low | v1.2 |
| 4.x Low priority items | Low | Varies | Low | Backlog |

---

## 6. Validation Metrics for Recommendations

After implementing recommendations, validate using:

### 6.1 Fire Shape Analysis

| Metric | Target | Measurement |
|--------|--------|-------------|
| Length:width ratio at 25 km/h | 2:1 to 4:1 | Measure fire perimeter ellipse |
| Length:width ratio at 50 km/h | 4:1 to 8:1 | Measure fire perimeter ellipse |
| Perimeter fractal dimension | 1.1-1.3 | Box-counting method |
| Spread rate coefficient of variation | 15-30% | Measure sector-by-sector |

### 6.2 Temporal Variability

| Metric | Target | Measurement |
|--------|--------|-------------|
| Spread rate standard deviation | 20-40% of mean | Time series analysis |
| Intensity fluctuation frequency | 0.1-1.0 Hz | Spectral analysis |
| Gust response lag | 2-10 seconds | Cross-correlation |

---

## 7. References

1. Finney, M.A. (2003) "Calculation of fire spread rates across random landscapes" International Journal of Wildland Fire
2. Anderson, H.E. (1983) "Predicting wind-driven wildland fire size and shape" USDA Research Paper INT-305
3. Richards, G.D. (1995) "A general mathematical framework for modeling two-dimensional wildland fire spread"
4. Albini, F.A. (1976) "Estimating wildfire behavior and effects" USDA General Technical Report INT-30
5. Byram, G.M. (1954) "Atmospheric conditions related to blowup fires" USDA Forest Service
6. Schroeder, M.J. & Buck, C.C. (1970) "Fire weather" USDA Agriculture Handbook 360
7. McArthur, A.G. (1967) "Fire behaviour in eucalypt forests" Leaflet 107

---

*Document generated as part of Phase 3 Calibration Recommendations*
