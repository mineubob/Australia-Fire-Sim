# Phase 2 Completion Guide

## Current Status
**Completed**: ~82% of temperature f32→f64 migration
**Remaining**: 71 type conversion errors across 10 files

## Remaining Errors by File

### 1. atmospheric.rs (2 errors)
- Line 38:30 - temperature comparison
- Line 40:59 - temperature arithmetic

**Fix Pattern**:
```rust
// Before
if source_temp > *grid.ambient_temperature + 50.0
let temp_excess = source_temp - *grid.ambient_temperature

// After  
if f32::from(source_temp) > *grid.ambient_temperature + 50.0
let temp_excess = f32::from(source_temp - *grid.ambient_temperature)
```

### 2. element.rs (3 errors)
- Line 330:57 - ignition probability calculation
- Line 343:59 - temperature factor calculation
- Line 373:15 - burn rate temperature factor

**Fix Pattern**:
```rust
// Add f64::from() wrapper for f32 arithmetic results used in f64 context
let ignition_prob = f64::from(moisture_factor * temp_factor * dt * base_coefficient * ffdi_multiplier);
```

### 3. ember.rs (2 errors)
- Line 184:55 - cooling rate calculation
- Line 276:13 - temperature assignment

**Fix Pattern**:
```rust
*self.temperature -= f64::from(cooling_rate * dt);
```

### 4. weather.rs (5 errors)  
- Lines 1106, 1126, 1155, 1248, 1342 - temperature conversions in weather updates

**Fix Pattern**:
```rust
// When calling functions that expect f32 temperature
preset.get_humidity(day, f32::from(*self.temperature), pattern)
```

### 5. simulation_grid.rs (3 errors)
- Lines 115, 620, 631 - grid temperature handling

**Fix Pattern**:
```rust
// Convert grid temperatures (f64) to/from f32 at boundaries
Celsius::from(temperature_f32)
```

### 6. element_heat_transfer.rs (3 errors)
- Line 127 - T^4 calculation result conversion

**Fix Pattern**:
```rust
// Already computing in f64, just need consistent types
```

### 7. rothermel.rs (2 errors)
- Line 299 - temperature parameter passing

**Fix Pattern**:
```rust
function_call(f32::from(temperature), other_params)
```

### 8. suppression_physics.rs (1 error)
- Line 101 - temperature conversion

### 9. simulation/mod.rs (50 errors)
Most errors follow these patterns:

**Pattern A: Function calls expecting f32**
```rust
// Before
target.receive_heat(heat, ffdi, *ambient_temp, dt);

// After
target.receive_heat(heat, ffdi, f32::from(*ambient_temp), dt);
```

**Pattern B: Temperature arithmetic**
```rust
// Before
let temp_change = temp_diff * cooling_rate * dt;
element.temperature = Celsius::new(*element.temperature - temp_change);

// After
element.temperature = Celsius::new(*element.temperature - f64::from(temp_change));
```

**Pattern C: Celsius::new() → Celsius::from()**
```rust
// For f32 sources
Celsius::from(temperature_f32)
// For f64 sources  
Celsius::new(temperature_f64)
```

## Systematic Fix Strategy

1. **Search and Replace Patterns**:
   ```bash
   # Pattern 1: Celsius::new with f32 variable
   # Find: Celsius::new\(([a-z_]+)\)
   # Check if variable is f32, replace with: Celsius::from($1)
   
   # Pattern 2: Temperature in function calls
   # Find functions expecting f32 temperature
   # Add f32::from() wrapper
   ```

2. **File-by-File Approach**:
   - Start with files with fewest errors (suppression_physics.rs, rothermel.rs)
   - Build after each file to verify
   - Move to larger files (simulation/mod.rs last)

3. **Common Conversions Needed**:
   ```rust
   // When Celsius is f64 but function expects f32:
   f32::from(*celsius_value)
   
   // When f32 arithmetic result assigned to Celsius (f64):
   celsius = Celsius::new(*celsius + f64::from(f32_result))
   
   // When creating Celsius from f32:
   Celsius::from(f32_value)  // Uses From<f32> trait
   
   // When creating Celsius from f64:
   Celsius::new(f64_value)   // Direct constructor
   ```

## Testing After Completion

```bash
# 1. Build
cargo build --workspace

# 2. Run tests
cargo test --workspace --lib

# 3. Run clippy
cargo clippy --workspace -- -D warnings

# 4. Run integration tests
cargo test --workspace --test '*'
```

## Expected Impact

After completing Phase 2:
- All temperature calculations will use f64 precision
- T^4 calculations (Stefan-Boltzmann) will be more accurate
- Long-running simulations will have less accumulated error
- ~87 type conversions added throughout codebase
- No behavior changes expected (just precision improvements)

## Phase 4 (Next Steps)

After Phase 2 compiles successfully:
1. Update FFI layer (fire_sim_update dt parameter: f32→f64)
2. Update FFI temperature fields to f64
3. Regenerate FireSimFFI.h with cbindgen
4. Update C/C++ integration code if needed
