# Type Migration Status Report

## Summary
Completed Phases 1 and 3 of the precision type migration pilot. Phase 2 (temperature/time/energy → f64) deferred to focused follow-up PR due to scope.

## Completed Work

### Phase 1: Integer Type Hygiene ✅
**Commits**: d5bb829, b942d23, e2f0b4d

#### Changes:
- `FuelElement.neighbors`: `Vec<u32>` → `Vec<usize>` for consistency with element IDs
- `FuelElement.time_above_ignition`: `f32` → `f64` for accumulated time precision  
- Fixed 17 loop index cast patterns with explicit annotations
- Updated 4 files: element.rs, simulation/mod.rs, integration_fire_behavior.rs, australian_bushfire_validation.rs

#### Impact:
- Integer type safety improved
- Accumulated time won't drift in long-running simulations
- Clearer code intent with explicit casts

### Clippy Annotations Update ✅
**Commit**: 502347d

#### Changes:
- Converted all `#[allow(clippy::...)]` to `#[expect(clippy::..., reason = "...")]`
- Added detailed reason strings for all 10 lint suppressions
- Updated files: simulation/mod.rs, integration_fire_behavior.rs, element_heat_transfer.rs, albini_spotting.rs

#### Examples:
```rust
// Before
#[allow(clippy::cast_precision_loss)] // Deliberate: small integer to position

// After  
#[expect(clippy::cast_precision_loss, reason = "Small integer (0-5) to position - precision loss acceptable for spatial coordinates")]
```

### Phase 3: Documentation ✅
**Commit**: 30731b8

#### Created:
- **PRECISION_RATIONALE.md**: Comprehensive documentation of type precision choices
  - f64 usage rationale (accumulated values, T^4 calculations)
  - f32 retention rationale (spatial coords, physical quantities)
  - Integer type usage (IDs, indices, counts)
  - Performance considerations (memory, CPU, SIMD)
  - Migration strategy and future work

## Deferred Work

### Phase 2: High-Precision Physics ⏸️

#### Scope Analysis:
**Temperature types (Celsius/Kelvin) migration to f64**:
- Requires 87+ type conversion fixes
- Affects units.rs, element.rs, atmospheric.rs, all physics modules
- Needs systematic f32→f64 arithmetic conversions
- Requires extensive testing for numerical behavior

#### Files Impacted (discovered during migration attempt):
- `crates/core/src/core_types/units.rs` - Type definitions
- `crates/core/src/core_types/element.rs` - Heat transfer calculations
- `crates/core/src/core_types/atmospheric.rs` - Temperature comparisons
- `crates/core/src/core_types/ember.rs` - Ember temperature
- All physics modules (rothermel.rs, crown_fire.rs, albini_spotting.rs, etc.)

#### Recommended Approach:
1. **Separate PR #1**: Temperature types (Celsius/Kelvin → f64)
   - Update type definitions in units.rs
   - Fix all arithmetic operations
   - Update all comparisons and conversions
   - Test numerical behavior

2. **Separate PR #2**: Time types (Seconds/Hours → f64)
   - Similar scope to temperature migration
   - Update simulation dt accumulation

3. **Separate PR #3**: Energy types (Kilojoules → f64)
   - Update heat transfer calculations
   - Test energy conservation

### Phase 4: FFI Layer ⏸️
- Depends on Phase 2 completion
- Update fire_sim_update(dt) parameter: f32 → f64
- Update temperature fields in FFI structs
- Regenerate FireSimFFI.h with cbindgen

## Testing Status

### All Tests Passing ✅
```
test result: ok. 146 passed; 0 failed; 0 ignored
```

### Clippy Clean ✅
```bash
cargo clippy --workspace -- -D warnings
✓ No warnings
```

### Build Status ✅
```bash
cargo build --workspace
✓ Successful
```

## Lessons Learned

### Temperature Migration Complexity
Initial attempt to migrate temperature types revealed:
- Deep integration throughout codebase (87+ error sites)
- Mixed f32/f64 arithmetic in physics calculations
- Need for systematic conversion strategy
- Benefit of incremental approach

### Documentation Value
Creating PRECISION_RATIONALE.md before completing full migration:
- Clarifies design decisions
- Guides future work
- Justifies f32 retention where appropriate
- Documents performance trade-offs

### Incremental Wins
Phase 1 provides immediate value:
- Integer type safety
- Accumulated time precision
- Foundation for future work
- Zero behavior regressions

## Recommendation

**Merge Current PR** with Phases 1 & 3:
- Provides tangible improvements
- Well-tested and documented
- Low risk

**Phase 2 as Focused Follow-up**:
- Dedicated PR for temperature/time/energy → f64
- Systematic approach to 87+ type conversions
- Comprehensive numerical testing
- Performance benchmarking

## Files Changed This PR

1. `crates/core/src/core_types/element.rs` - neighbors type, time precision
2. `crates/core/src/simulation/mod.rs` - loop casts, clippy expects
3. `crates/core/src/simulation/action_queue.rs` - loop cast
4. `crates/core/tests/integration_fire_behavior.rs` - loop casts, clippy expects
5. `crates/core/tests/australian_bushfire_validation.rs` - loop casts
6. `crates/core/src/physics/element_heat_transfer.rs` - clippy expects
7. `crates/core/src/physics/albini_spotting.rs` - clippy expect
8. `PRECISION_RATIONALE.md` - New documentation
9. `MIGRATION_STATUS.md` - This file

**Total**: 9 files, 5 commits, 146 tests passing
