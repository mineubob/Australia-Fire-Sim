# Type Precision Rationale

This document explains the precision choices for different types in the Australia Fire Simulation codebase.

## Overview

The codebase uses a context-specific approach to type precision:
- **f64**: High-precision physics calculations, accumulated values
- **f32**: Spatial coordinates, non-accumulated physical quantities
- **Integer types**: IDs, indices, counts

## Detailed Rationale

### f64 Usage (High Precision Required)

#### Accumulated Time
- **Type**: `FuelElement.time_above_ignition: f64`
- **Rationale**: Accumulated over simulation duration (hours/days). f32 would drift.
- **Example**: 3600 seconds (1 hour) accumulated in 0.01s timesteps = 360,000 additions

#### Temperature in Future Phases
- **Planned**: `Celsius`, `Kelvin` → f64 wrappers
- **Rationale**: Used in T^4 calculations (Stefan-Boltzmann law). Temperature^4 creates large values requiring f64 precision.
- **Example**: 1000K^4 = 1e12, beyond f32 accurate range

#### Time Units in Future Phases  
- **Planned**: `Seconds`, `Hours` → f64 wrappers
- **Rationale**: Simulation dt accumulated over time
- **Example**: dt=0.01s accumulated over 1 hour = 360,000 additions

#### Energy Units in Future Phases
- **Planned**: `Kilojoules`, `KjPerKg` → f64 wrappers
- **Rationale**: Heat transfer calculations accumulated over time
- **Example**: Heat transfer summed across 10,000+ elements per frame

### f32 Usage (Adequate Precision)

#### Spatial Coordinates
- **Type**: `Vec3 = Vector3<f32>`
- **Rationale**: Meter-scale positions. f32 provides ~0.001mm precision at 1000m scale.
- **Performance**: SIMD optimizations, 2x memory savings
- **Example**: Position (500.0, 500.0, 10.0) - f32 precision is 0.000061m (61 microns)

#### Distance Types
- **Types**: `Meters`, `Kilometers`
- **Rationale**: Spatial measurements, not accumulated. Precision adequate for fire simulation scale.
- **Example**: 1000m represented in f32 with 0.000061m precision

#### Mass Types
- **Type**: `Kilograms`
- **Rationale**: Physical quantity values, f32 precision (7 digits) sufficient
- **Example**: Fuel mass 0.1kg to 100kg - f32 handles this range easily

#### Angle Types
- **Types**: `Degrees`, `Radians`
- **Rationale**: Trigonometric calculations. f32 provides 0.00001° precision.
- **Example**: Wind direction 45.0° - f32 precision is more than adequate

#### Fraction/Percentage Types
- **Types**: `Fraction` (0-1), `Percent` (0-100)
- **Rationale**: Relative values in constrained range. f32 provides 7-digit precision.
- **Example**: Moisture content 0.15 (15%) - f32 precision is 0.0000001

#### Density Types
- **Type**: `KgPerCubicMeter`
- **Rationale**: Physical property values, not accumulated
- **Example**: Air density 1.225 kg/m³ - f32 sufficient

### Integer Types

#### Element IDs
- **Type**: `usize`
- **Rationale**: Array indices and counts must be integers
- **Change**: Migrated from `Vec<u32>` to `Vec<usize>` for consistency

#### Loop Counters
- **Pattern**: `for i in 0..n_i32` with explicit `(i as f32)` casts
- **Rationale**: Clearer intent, documented precision loss
- **Example**: Grid cell indices, iteration counts

## Performance Considerations

### Memory

- **f32**: 4 bytes per value
- **f64**: 8 bytes per value  
- **Impact**: With 10,000+ elements, f64 doubles memory footprint for large types

### CPU Performance

- **f32**: Better SIMD vectorization (8 values per 256-bit vector)
- **f64**: Fewer values per vector (4 values per 256-bit vector)
- **Impact**: Spatial operations (Vec3) benefit from f32 SIMD

### Precision Trade-offs

| Type | Precision | Range | Choice |
|------|-----------|-------|--------|
| f32 | ~7 decimal digits | ±3.4e38 | Spatial, physical quantities |
| f64 | ~15 decimal digits | ±1.7e308 | Accumulated values, T^4 |

## Migration Strategy

### Completed (Phase 1)
- ✅ Integer type hygiene (IDs, loop counters)
- ✅ Accumulated time precision (time_above_ignition)
- ✅ Clippy expect annotations with reasons

### Deferred (Phase 2)
- ⏸️ Temperature types → f64 (requires ~87 type conversion fixes)
- ⏸️ Time types → f64 (requires similar scope)
- ⏸️ Energy types → f64 (requires similar scope)

**Rationale for Deferral**: Large scope (87+ compilation errors), requires:
- Systematic conversion of f32 arithmetic to f64
- Update of all physics formulas
- Extensive testing for numerical behavior
- Separate focused PR recommended

### Future Work
- Physics formulas: Compute in f64, cast results with annotations
- FFI layer: Update dt and temperature parameters to f64
- Benchmarking: Measure performance impact of f64 migrations

## References

- IEEE 754 Floating Point Standard
- "What Every Computer Scientist Should Know About Floating-Point Arithmetic" (Goldberg, 1991)
- Rust f32/f64 documentation
- Stefan-Boltzmann law requirements (T^4 precision needs)
