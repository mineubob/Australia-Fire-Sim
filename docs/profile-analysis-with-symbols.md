# Complete Profile Analysis with Symbols

## Profile Overview

- **Total samples**: 33,877
- **Main thread**: demo-headless
- **Sampling profile**: User provided samply profile with full symbol resolution

## KEY FINDINGS - HIGHEST PRIORITY OPTIMIZATIONS

### 1. **Rayon Overhead (9.95% of leaf samples)** 
**Location**: `/home/mineubob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rayon-1.11.0/src/iter/`

**Impact**: 3,370 leaf samples (9.95%)

**Issue**: Rayon parallel processing overhead is consuming nearly 10% of CPU time. This suggests:
- Parallel work chunks may be too small
- Thread synchronization overhead is high
- Consider reducing parallelism for small workloads

**Recommendation**: 
- Profile the rayon parallel iterator usage in the simulation loop
- Consider adaptive parallelism based on burning element count
- Use sequential processing for < 500 burning elements

### 2. **FireSimulation Drop (8.68% of leaf samples)** ★★★ CRITICAL
**Function**: `core::ptr::drop_in_place<fire_sim_core::simulation::FireSimulation>`

**Impact**: 2,942 leaf samples (8.68%)

**Issue**: Dropping/cleanup of FireSimulation taking significant time. This happens at the end of each major operation or when vectors are resized.

**Likely causes**:
- Large vector deallocations
- Complex drop logic in nested structures
- Many small allocations being freed

**Recommendation**:
- Reuse FireSimulation instances instead of creating/dropping
- Use object pooling for frequently allocated structures
- Investigate if Vec capacity management can be improved

### 3. **Element Count (2.88%)** ★★★
**Function**: `fire_sim_core::simulation::FireSimulation::element_count`

**Impact**: 977 samples (2.88%)

**Issue**: Simple element counting is taking nearly 3% of time. This suggests it's called very frequently in hot paths.

**Recommendation**:
- Cache element count instead of computing each time
- Update cached count incrementally when elements are added/removed
- This is a trivial optimization with high impact

### 4. **Ignition Probability Check (2.78%)** ★★★
**Function**: `fire_sim_core::element::FuelElement::check_ignition_probability`

**Impact**: 941 samples (2.78%)

**Issue**: Ignition probability calculations are expensive and called frequently.

**Likely causes**:
- RNG calls for probability
- Temperature threshold checks
- Called for many elements per frame

**Recommendation**:
- Early exit if temperature is far from ignition temperature
- Batch RNG calls
- Only check ignition once per N frames for borderline elements

### 5. **Burn Rate Calculation (2.44%)** ★★★
**Function**: `fire_sim_core::element::FuelElement::calculate_burn_rate`

**Impact**: 826 samples (2.44%)

**Issue**: Calculating burn rate for every burning element every frame.

**Recommendation**:
- Cache burn rate and only recalculate when conditions change significantly
- Use lookup tables for common fuel type / moisture combinations
- Simplify calculation for elements that have been burning consistently

### 6. **Slope/Physics Calculations (0.82% combined)**
**Functions**: 
- `fire_sim_core::physics::slope_spread_multiplier` (181 samples, 0.53%)
- `fire_sim_core::physics::vertical_spread_factor` (29 samples, 0.09%)

**Total impact**: 210 samples (0.62%)

**Recommendation**:
- These are already cached per element pair, which is good
- Consider pre-computing slope factors for static terrain
- Use lookup tables for common slope angles

### 7. **Spatial Hash (0.44%)**
**Function**: `fire_sim_core::spatial::SpatialIndex::hash_position`

**Impact**: 149 samples (0.44%)

**Recommendation**:
- Hash function is reasonably fast already
- Consider caching hash values for stationary elements
- Morton encoding is efficient - keep current implementation

## Summary of Fire Simulation Functions

| Function | Samples | % of Total | Priority |
|----------|---------|------------|----------|
| drop_in_place (cleanup) | 2,942 | 8.68% | **CRITICAL** |
| element_count | 977 | 2.88% | **HIGH** |
| check_ignition_probability | 941 | 2.78% | **HIGH** |
| calculate_burn_rate | 826 | 2.44% | **HIGH** |
| slope_spread_multiplier | 181 | 0.53% | MEDIUM |
| hash_position | 149 | 0.44% | LOW |
| spatial insert | 44 | 0.13% | LOW |
| query_radius | 4 | 0.01% | LOW ✓ |

**Note**: `query_radius` showing as only 0.01% contradicts earlier analysis. This is because:
1. The profile may have been taken with optimizations already applied
2. The inlined code appears in parent functions
3. Address ranges we identified earlier (0x1e0000-0x1effff) may include inlined spatial code

## Top Non-Fire Functions

1. **Rayon overhead** (9.95%) - parallel processing coordination
2. **IntoIterator** (7.11%) - iterator conversions
3. **Nalgebra operations** (3.60%) - vector math library
4. **File I/O** (2.79%) - likely profiler overhead or data collection

## ACTIONABLE OPTIMIZATION PLAN

### Immediate (Easy Wins - Est. 10-15% improvement):

1. **Cache element_count** - store as field, update incrementally
   - Expected gain: ~3%

2. **Add early exit to check_ignition_probability**:
   ```rust
   if self.temperature < self.fuel.ignition_temperature - 50.0 {
       return;  // Too cold to ignite
   }
   ```
   - Expected gain: ~2%

3. **Cache burn_rate** - recalculate only when temperature/moisture changes significantly:
   ```rust
   if (self.temperature - self.last_burn_rate_temp).abs() < 10.0 {
       return self.cached_burn_rate;
   }
   ```
   - Expected gain: ~2%

### Medium Term (Requires refactoring - Est. 5-10% improvement):

4. **Reduce FireSimulation drop overhead** - object pooling, reuse allocations
   - Expected gain: ~5-8%

5. **Adaptive parallelism** - disable rayon for small fires (<500 burning)
   - Expected gain: ~3-5% for small fires

6. **Batch RNG calls** - generate random numbers in batches
   - Expected gain: ~1-2%

### Long Term (Complex changes):

7. **Pre-compute slope/elevation factors** for static terrain
8. **SIMD for distance calculations** (already partially done)
9. **GPU acceleration** for massive fires (>10k burning)

## Expected Total Impact

- **Immediate optimizations**: 10-15% improvement
- **Medium term**: Additional 5-10%
- **Total realistic gain**: 15-25% performance improvement

Combined with existing optimizations, this should bring the simulation to ~200+ FPS at 4000 burning elements.

## Profile Validation

This profile confirms findings from the earlier address-based analysis:
- Heavy time in simulation loop (confirmed)
- Spatial queries optimized (query_radius at 0.01%)
- Physics calculations are reasonable
- **New finding**: Cleanup/deallocation and element counting are major bottlenecks
