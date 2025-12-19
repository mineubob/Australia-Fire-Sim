# Operational Limits and Valid Ranges

**Document:** Phase 3.3 - Model Operational Limits and Valid Input Ranges  
**Standard:** Professional Fire Behavior Model Documentation  
**Date:** January 2025  

---

## Executive Summary

This document defines the valid operating envelope for the Bushfire Simulation engine, including:

1. **Valid input ranges** - Where the model is calibrated and validated
2. **Extrapolation zones** - Beyond validation data but physically reasonable
3. **Known limitations** - Features not fully modeled or validated
4. **Model breakdown conditions** - Where numerical or physical assumptions fail

### Quick Reference

| Parameter | Valid Range | Extrapolation Zone | Hard Limits |
|-----------|-------------|-------------------|-------------|
| Temperature | -10°C to 50°C | 50°C to 55°C | 55°C max |
| Humidity | 5% to 100% | 2% to 5% | 2% min |
| Wind speed | 0 to 100 km/h | 100-120 km/h | 150 km/h max |
| Fuel moisture | 3% to 35% | 2-3% or 35-40% | 2% min, 40% max |
| Slope | 0° to 40° | 40° to 45° | 60° max |
| FFDI | 0 to 150 | 150 to 200 | 300 max |

---

## 1. Valid Input Ranges

### 1.1 Temperature

| Zone | Range | Status | Notes |
|------|-------|--------|-------|
| **Validated** | -5°C to 50°C | ✅ Full confidence | Australian climate range |
| Cold extension | -10°C to -5°C | ⚠️ Limited data | Rare fire conditions |
| Hot extension | 50°C to 55°C | ⚠️ Extrapolation | Rare, extreme heatwaves |
| **Hard limit** | 55°C | ❌ | Beyond physical recording |

**Physical basis:**
- Below -5°C: Fires possible but rare; fuel moisture dominates
- Above 50°C: Record Australian temperature = 50.7°C (Oodnadatta)
- FFDI formula validated to ~50°C

**Recommendation:**
- Use with confidence: -5°C to 48°C
- Use with caution: 48°C to 55°C
- Reject inputs: > 55°C

### 1.2 Relative Humidity

| Zone | Range | Status | Notes |
|------|-------|--------|-------|
| **Validated** | 5% to 100% | ✅ Full confidence | Normal range |
| Extreme dry | 2% to 5% | ⚠️ Limited data | Rare conditions |
| **Hard limit** | 2% min | ❌ | Instrument/physical limit |

**Physical basis:**
- Below 5% RH: Instrument accuracy degrades
- Black Saturday recorded 6% (extreme)
- FFDI validated to 5% RH

**Recommendation:**
- Use with confidence: 8% to 100%
- Use with caution: 2% to 8%
- Clamp minimum: 2%

### 1.3 Wind Speed

| Zone | Range | Status | Notes |
|------|-------|--------|-------|
| **Validated** | 0 to 80 km/h | ✅ Full confidence | Majority of fires |
| Extended | 80 to 100 km/h | ⚠️ Some data | Major fire events |
| Extreme | 100 to 120 km/h | ⚠️ Limited data | Rare events |
| Theoretical | 120 to 150 km/h | ❌ Extrapolation | Pyroconvective/cyclonic |

**Physical basis:**
- Rothermel wind factor calibrated to ~80 km/h
- Black Saturday gusts exceeded 100 km/h
- Above 120 km/h: fire-atmosphere coupling dominates

**Recommendation:**
- Use with confidence: 0 to 80 km/h
- Use with caution: 80 to 100 km/h
- Limited confidence: 100 to 120 km/h
- Reject/flag: > 120 km/h

### 1.4 Fuel Moisture Content

| Zone | Range | Status | Notes |
|------|-------|--------|-------|
| **Validated** | 4% to 30% | ✅ Full confidence | Normal fire conditions |
| Very dry | 3% to 4% | ⚠️ Some data | Extreme fire days |
| Extreme dry | 2% to 3% | ⚠️ Extrapolation | Theoretical limit |
| High moisture | 30% to 35% | ⚠️ Limited data | Marginal fire spread |
| No spread | > 35% | ❌ | Above extinction |

**Physical basis:**
- Eucalyptus moisture of extinction: ~30%
- Black Saturday: 3-4% moisture observed
- Below 2%: Unrealistic (oven-dry conditions)

**Recommendation:**
- Use with confidence: 5% to 25%
- Use with caution: 3-5% or 25-30%
- Flag as extreme: < 3%
- Fire unlikely: > 30%

### 1.5 Slope

| Zone | Range | Status | Notes |
|------|-------|--------|-------|
| **Validated** | 0° to 35° | ✅ Full confidence | Common terrain |
| Steep | 35° to 45° | ⚠️ Limited data | Complex terrain |
| Very steep | 45° to 60° | ❌ Extrapolation | Cliffs, gullies |
| Vertical | > 60° | ❌ Invalid | Model not designed |

**Physical basis:**
- Rothermel slope factor: $\phi_s = 5.275 \times \tan^2(\theta)$
- Validated primarily on slopes < 30°
- Above 45°: Convective column dynamics differ

**Recommendation:**
- Use with confidence: 0° to 30°
- Use with caution: 30° to 45°
- Limited confidence: > 45°
- Consider specialized models: > 60°

### 1.6 Fuel Load

| Zone | Range | Status | Notes |
|------|-------|--------|-------|
| **Validated** | 2 to 40 t/ha | ✅ Full confidence | Australian fuels |
| Very light | 0.5 to 2 t/ha | ⚠️ Limited data | Grazed/sparse |
| Very heavy | 40 to 60 t/ha | ⚠️ Extrapolation | Rainforest edge |
| Extreme | > 60 t/ha | ❌ | Beyond calibration |

**Recommendation:**
- Use with confidence: 5 to 35 t/ha
- Use with caution: 2-5 or 35-50 t/ha
- Requires special consideration: > 50 t/ha

### 1.7 FFDI (Calculated)

| Zone | Range | Status | Notes |
|------|-------|--------|-------|
| Low-Moderate | 0-12 | ✅ Validated | Normal conditions |
| High | 12-24 | ✅ Validated | Elevated fire danger |
| Very High | 24-50 | ✅ Validated | Significant fires |
| Severe | 50-75 | ✅ Validated | Major fires |
| Extreme | 75-100 | ✅ Validated | Catastrophic potential |
| Catastrophic | 100-150 | ⚠️ Some data | Black Saturday range |
| Beyond validation | 150-200 | ⚠️ Extrapolation | Theoretical only |
| Extreme theoretical | > 200 | ❌ | Pyroconvective limit |

**Recommendation:**
- Full confidence: FFDI 0 to 120
- Caution: FFDI 120 to 175 (Black Saturday peak)
- Limited confidence: FFDI > 175

---

## 2. Known Limitations

### 2.1 Features Not Modeled

| Feature | Status | Impact | Mitigation |
|---------|--------|--------|------------|
| Pyroconvection (full) | Not modeled | Underestimates extreme fires | Apply correction factor |
| Fire whirls | Not modeled | Safety risk underestimated | Operational awareness |
| Smoke-weather interaction | Not modeled | Spread timing affected | Use ensemble approach |
| Fire coalescence | Partial | Multiple fire merging underestimated | Treat merged fires separately |
| Terrain channeling | Partial | Valley acceleration underestimated | Local calibration |
| **Fuel spatial heterogeneity** | **Default: uniform** | **Fire shape too circular** | **Add fuel variation at creation** |
| **Turbulent wind gusting** | **Not modeled** | **Perimeter too smooth** | **Implement wind fluctuation model** |

**Note on Fire Shape:** With uniform fuel and steady wind, fire spread appears more circular than real fires. Real fires have irregular, fractal-like perimeters due to fuel heterogeneity, wind gusting, and micro-terrain effects. See `calibration_recommendations.md` for enhancement guidance.

### 2.2 Partially Validated Features

| Feature | Validation Level | Confidence | Notes |
|---------|------------------|------------|-------|
| Ember spotting | Validated to 25 km | High | Black Saturday data |
| Crown fire transition | Moderate | Medium | Van Wagner model |
| Stringybark ladder fuel | Limited | Medium | Pausas et al. (2017) |
| Suppression effectiveness | Limited | Low | Limited field data |
| Smoldering combustion | Theoretical | Low | Rein (2009) model |

### 2.3 Geographic Limitations

| Region | Status | Notes |
|--------|--------|-------|
| **Australia** | ✅ Primary validation | Full fuel type calibration |
| Mediterranean climates | ⚠️ Applicable | Similar conditions |
| Temperate forests (global) | ⚠️ Applicable | Adjust fuel properties |
| Tropical forests | ⚠️ Limited | Different fuel dynamics |
| Boreal forests | ❌ Not validated | Different fire behavior |
| Grasslands (global) | ✅ Applicable | Universal physics |

### 2.4 Temporal Limitations

| Duration | Status | Notes |
|----------|--------|-------|
| 0-6 hours | ✅ Validated | Good accuracy |
| 6-24 hours | ⚠️ Degraded | Weather change uncertainty |
| 24-72 hours | ⚠️ Limited | Cumulative errors grow |
| > 72 hours | ❌ Not validated | Use for scenarios only |

---

## 3. Model Breakdown Conditions

### 3.1 Numerical Instability

| Condition | Symptom | Detection | Mitigation |
|-----------|---------|-----------|------------|
| Very small timestep | Slow computation | dt < 0.001s | Increase dt |
| Very large timestep | NaN values | dt > 10s | Decrease dt |
| Extreme wind + slope | Spread rate overflow | R > 500 m/min | Cap at physical max |
| Near-zero fuel moisture | Division instability | M < 0.5% | Clamp to 2% |

### 3.2 Physical Assumption Violations

| Condition | Violated Assumption | Impact |
|-----------|---------------------|--------|
| Wind > 150 km/h | Steady-state wind field | Model invalid |
| Slope > 60° | Horizontal spread model | Use specialized model |
| Temperature < -20°C | Fire chemistry | Fire unlikely anyway |
| Multiple fire fronts | Independent fires | Interaction underestimated |
| Urban interface | Vegetative fuels only | Structure ignition not modeled |

### 3.3 Boundary Conditions

| Boundary | Behavior | Notes |
|----------|----------|-------|
| Grid edge | Fire stops | No wrap-around |
| Water bodies | Fire stops | Hard barrier |
| Roads | Fire crosses (reduced) | Not perfect barrier |
| Fuel discontinuity | Reduced spread | Spotting may bridge |

---

## 4. Uncertainty by Condition

### 4.1 Prediction Uncertainty Ranges

| Condition | Spread Rate Uncertainty | Intensity Uncertainty |
|-----------|------------------------|----------------------|
| FFDI 0-12 | ±50-60% | ±40-50% |
| FFDI 12-24 | ±40-50% | ±30-40% |
| FFDI 24-50 | ±30-40% | ±25-30% |
| FFDI 50-75 | ±25-35% | ±20-25% |
| FFDI 75-100 | ±25-30% | ±20-25% |
| FFDI 100+ | ±30-35% | ±25-30% |

### 4.2 Confidence Level by Fuel Type

| Fuel Type | Confidence | Notes |
|-----------|------------|-------|
| Grassland | High | Well-validated, simple |
| Eucalyptus forest | High | Primary calibration |
| Mixed forest | Medium | Complex structure |
| Heath/shrubland | Medium | Limited data |
| Mallee | Medium | Sparse validation |
| Alpine | Low | Limited events |
| Rainforest margins | Low | Rare fires |

---

## 5. Warning Flags

### 5.1 Input Validation Flags

The simulation should flag the following conditions:

```rust
// Warning flags for input validation
pub enum InputWarning {
    TemperatureExtreme,      // T > 48°C
    HumidityVeryLow,         // RH < 5%
    WindSpeedExtreme,        // V > 80 km/h
    FuelMoistureVeryLow,     // M < 4%
    FuelMoistureHigh,        // M > 25%
    SlopeVeryStep,           // θ > 35°
    FFDICatastrophic,        // FFDI > 100
    FFDIBeyondValidation,    // FFDI > 150
    FuelLoadExtreme,         // Load > 40 t/ha
}
```

### 5.2 Output Validation Flags

```rust
// Warning flags for output validation
pub enum OutputWarning {
    SpreadRateExtreme,       // R > 200 m/min
    IntensityExtreme,        // I > 100,000 kW/m
    SpottingDistanceLong,    // D > 20 km
    FlameHeightExtreme,      // L > 40 m
    NumericalInstability,    // NaN or Inf detected
}
```

---

## 6. Recommended Operating Procedures

### 6.1 Pre-Run Validation

1. **Check all inputs against valid ranges**
2. **Flag any extrapolation zone inputs**
3. **Reject hard limit violations**
4. **Log all warnings**

### 6.2 Runtime Monitoring

1. **Monitor for NaN/Inf values**
2. **Check spread rate reasonableness**
3. **Validate energy conservation**
4. **Flag extreme outputs**

### 6.3 Post-Run Analysis

1. **Report uncertainty bounds**
2. **Document any warnings triggered**
3. **Compare against historical events if applicable**
4. **Note any model limitations relevant to scenario**

---

## 7. Comparison with Other Models

### 7.1 Operating Envelope Comparison

| Parameter | This Model | BehavePlus | Phoenix | Prometheus |
|-----------|------------|------------|---------|------------|
| Wind max | 120 km/h | 80 km/h | 100 km/h | 80 km/h |
| Slope max | 45° | 40° | 45° | 40° |
| FFDI max | 200 | N/A | 200 | N/A |
| Spotting max | 25 km | 3 km | 20 km | 10 km |

### 7.2 Unique Capabilities

| Feature | Status | Notes |
|---------|--------|-------|
| Eucalyptus oil volatilization | ✅ | Unique to this model |
| Stringybark ladder fuel | ✅ | Australian-specific |
| 25 km spotting | ✅ | Black Saturday validated |
| FFDI > 100 | ✅ | Extreme events |
| Smoldering/glowing | ✅ | Rein (2009) model |

---

## 8. Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | Jan 2025 | Initial operational limits document |

---

## 9. Summary Tables

### 9.1 Quick Reference: Valid Ranges

| Parameter | Min | Max (Validated) | Max (Extrapolation) | Hard Max |
|-----------|-----|-----------------|---------------------|----------|
| Temperature | -10°C | 50°C | 55°C | 55°C |
| Humidity | 2% | 100% | - | 100% |
| Wind speed | 0 | 80 km/h | 120 km/h | 150 km/h |
| Fuel moisture | 2% | 30% | 35% | 40% |
| Slope | 0° | 35° | 45° | 60° |
| Fuel load | 0.5 t/ha | 40 t/ha | 60 t/ha | 80 t/ha |
| FFDI | 0 | 150 | 200 | 300 |

### 9.2 Quick Reference: Confidence Levels

| Condition | Confidence | Use Case |
|-----------|------------|----------|
| Typical Australian fire day | High | Operational planning |
| Severe fire day (FFDI 50-75) | High | Tactical decisions |
| Extreme fire day (FFDI 75-100) | Medium-High | Strategic planning |
| Catastrophic (FFDI 100+) | Medium | Scenario analysis |
| Beyond validation (FFDI 150+) | Low | Research only |

---

*Document generated as part of Phase 3 Operational Limits*
