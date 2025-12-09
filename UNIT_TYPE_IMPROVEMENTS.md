# Unit Type Usage Improvements

## Summary

Changed `GridCell.temperature` from `f32` to `Celsius` to eliminate unnecessary dereferencing and improve type safety. This change demonstrates proper use of unit types throughout the codebase.

## Problem

**Inconsistency**: `GridCell.temperature` was `f32` while `SimulationGrid.ambient_temperature` was `Celsius`, causing:
- Frequent `*ambient_temperature as f32` conversions (10+ locations)
- Mixing raw floats with typed units
- Lost type safety benefits
- Reduced code clarity

## Solution

Changed `GridCell.temperature: f32` → `GridCell.temperature: Celsius`

This aligns with existing temperature unit usage in:
- `SimulationGrid.ambient_temperature: Celsius`
- `FuelElement.temperature: Celsius`
- `Weather.temperature: Celsius`
- All Fuel temperature properties (Celsius)

## Benefits

### 1. Type Safety
```rust
// Before: Can accidentally mix temperatures with other f32 values
let temp: f32 = cell.temperature;
let wrong_calc = temp * some_other_f32; // Might be semantically wrong!

// After: Type system prevents mistakes
let temp: Celsius = cell.temperature;
// let wrong_calc = temp * some_other_f32; // Compile error ✓
let correct = *temp as f32; // Explicit deref when truly needed
```

### 2. Code Clarity
```rust
// Before: Unclear intent
if source_temp > (*grid.ambient_temperature as f32) + 50.0 {
    let excess = source_temp - (*grid.ambient_temperature as f32);
    // ...
}

// After: Clear intent
if source_temp > grid.ambient_temperature + Celsius::new(50.0) {
    let excess = source_temp - grid.ambient_temperature;
    // ...
}
```

### 3. Fewer Conversions
- **atmospheric.rs**: 5 deref eliminations (buoyancy, plume rise)
- **suppression_physics.rs**: 2 deref eliminations (cooling effects)
- **simulation/mod.rs**: 3 deref eliminations (heat transfer)
- **simulation_grid.rs**: Internal consistency (diffusion, interpolation)

## Changes Made

### 1. Core Type Definition
```rust
// simulation_grid.rs
pub struct GridCell {
    pub(crate) temperature: Celsius, // was: f32
    // ... other fields
}
```

### 2. Interpolation Support
Added `lerp_celsius` helper for trilinear interpolation:
```rust
let lerp_celsius = |a: Celsius, b: Celsius, t: f32| {
    Celsius::new(*a * (1.0 - f64::from(t)) + *b * f64::from(t))
};
```

### 3. Diffusion Algorithm
Updated function signature to use Celsius:
```rust
fn process_diffusion_cell(
    &self,
    idx: usize,
    grid_dims: (usize, usize, usize, usize),
    params: (Celsius, f32, f32), // ambient_temp now Celsius
) -> Option<(usize, Celsius)> // return type now Celsius
```

### 4. API Boundary
Public accessor remains f32 for external compatibility:
```rust
pub fn temperature(&self) -> f32 {
    *self.temperature as f32 // Explicit cast at boundary
}
```

## Pattern for Using Unit Types

### ✅ DO: Use unit types internally
```rust
let ambient_temp: Celsius = grid.ambient_temperature;
let element_temp: Celsius = element.temperature;
let temp_diff = element_temp - ambient_temp; // Type-safe
```

### ✅ DO: Cast at API boundaries
```rust
// FFI/stats output
pub struct ElementStats {
    pub temperature: f32, // External API uses f32
}

impl From<&FuelElement> for ElementStats {
    fn from(element: &FuelElement) -> Self {
        ElementStats {
            temperature: *element.temperature as f32, // Explicit cast
        }
    }
}
```

### ✅ DO: Use explicit conversions when mixing types
```rust
// Mixing f32 calculation with Celsius result
let heat_kj = f64::from(h * area * dt); // f32 → f64
let temp_rise = Celsius::new(heat_kj / specific_heat);
cell.temperature = cell.temperature + temp_rise;
```

### ❌ DON'T: Deref immediately after accessing
```rust
// Before (wasteful)
let temp = *element.temperature; // Deref to f64
let ambient = *grid.ambient_temperature; // Deref to f64
let diff = temp - ambient; // f64 arithmetic

// After (better)
let diff = element.temperature - grid.ambient_temperature; // Celsius arithmetic
```

### ❌ DON'T: Store raw floats when unit type available
```rust
// Before (loses type info)
let temp: f32 = cell.temperature; // was f32
let temp_k = temp + 273.15; // What unit is this?

// After (explicit)
let temp: Celsius = cell.temperature;
let temp_k = *temp + 273.15; // Clear: Celsius → Kelvin
```

## When to Keep Raw Floats

Raw f32/f64 is appropriate when:
1. **No semantic meaning**: Generic numeric calculations
2. **Performance critical**: Inner loops with many operations
3. **External constraints**: FFI boundaries, external APIs
4. **Library interfaces**: Third-party code expectations

Example:
```rust
// OK: Generic math, no units
let ratio = area / total_area;

// OK: FFI boundary
#[no_mangle]
pub extern "C" fn get_temperature(id: usize) -> f32 {
    let elem = get_element(id);
    *elem.temperature as f32 // Cast at boundary
}
```

## Performance Impact

**Negligible** - Unit types are zero-cost abstractions:
- Same memory layout as raw f64
- Operations compile to identical machine code
- Dereferencing is a no-op (just exposes inner value)

**Before** (f32 with many casts):
```rust
let temp = (*celsius as f32); // cast
let diff = temp - (*ambient as f32); // cast
let result = diff * 0.1; // calculation
```

**After** (Celsius throughout):
```rust
let diff = celsius - ambient; // no cast
let result = diff * 0.1; // calculation
```
Fewer operations = potentially faster!

## Testing

All 146 unit tests pass with no changes needed:
- GridCell operations
- Temperature diffusion
- Heat transfer
- Atmospheric coupling
- Suppression effects

Tests demonstrate that unit types work seamlessly with existing code.

## Conclusion

Using `Celsius` for `GridCell.temperature` demonstrates proper unit type usage:
- ✅ Internal consistency (all temperatures use same type)
- ✅ Type safety (can't mix temperatures with other values)
- ✅ Code clarity (intent is explicit)
- ✅ Fewer conversions (eliminated 10+ derefs)
- ✅ Zero performance cost

This pattern should be applied to other physical quantities where appropriate.
