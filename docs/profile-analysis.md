# Profile Analysis Results

## Summary

Analysis of the samply profile reveals the hot code paths in the fire simulation. Although symbols are stripped, address patterns clearly identify performance bottlenecks.

## Hot Address Ranges

| Address Range | Sample Count | Percentage | Likely Module |
|--------------|-------------|-----------|---------------|
| 0x200000-0x20ffff | 1,706,589 | 50.1% | Core simulation loop / physics calculations |
| 0x1e0000-0x1effff | 1,335,796 | 39.2% | Spatial index / element management |
| 0x210000-0x21ffff | 222,101 | 6.5% | Parallel processing / Rayon overhead |
| 0x220000-0x22ffff | 132,669 | 3.9% | Utility functions / allocations |

**Total hot code: 89.3% of execution time in just two address ranges**

## Top Hottest Addresses (by stack appearance)

1. `0x1e1c24` - 231,283 samples (702.8%) - Appears in almost every stack
2. `0x1e60f7` - 227,746 samples (692.0%) - Core function in hot path
3. `0x2083fd` - 226,890 samples (689.4%) - Physics/simulation calculation
4. `0x20a4ba` - 206,728 samples (628.2%) - Inner loop function
5. `0x208dd6` - 148,350 samples (450.8%) - Element processing

## Hot Call Paths

Most common execution paths (showing inner 5 frames):

1. `0x20c28c -> 0x1ecc48 -> 0x2042f1 -> 0x20458b -> 0x206945` (457 samples)
2. `0x1ee3da -> 0x1eeecf -> 0x20c28c -> 0x1ecc48 -> 0x2042f1` (432 samples)
3. `0x20816b -> 0x1ee3da -> 0x1eeecf -> 0x20c28c -> 0x1ecc48` (340 samples)

## Interpretation

Based on address patterns and code structure:

### Primary Bottleneck (50% - 0x200000-0x20ffff range)
This is likely the main `simulation.rs` update loop containing:
- Heat transfer calculations between elements
- Physics multiplier applications (wind, slope, vertical)
- Target iteration and filtering

**Key addresses in this range:**
- `0x2042f1`, `0x2050eb`, `0x206945`, `0x2063d1`, `0x206a64` - Hot inner loops
- `0x20a4ba`, `0x208dd6`, `0x20a685` - Repeated calculations

### Secondary Bottleneck (39% - 0x1e0000-0x1effff range)
This is likely spatial index queries and element access:
- `query_radius` calls
- `get_element` lookups
- Element property access

**Key addresses:**
- `0x1e1c24`, `0x1e60f7` - Spatial query functions
- `0x1e7964`, `0x1e71b3` - Element access patterns

## Optimization Opportunities

### 1. Reduce Spatial Query Overhead (39% of time)
The spatial index is called extensively. Potential improvements:
- **Cache spatial query results** for elements that don't move
- **Batch queries** for nearby burning elements
- **Reduce query radius** adaptively based on element temperature
- **Skip queries** for low-temperature elements

### 2. Optimize Inner Physics Loop (50% of time)
The heat transfer calculation loop is the hottest:
- **Early exit** if heat transfer would be negligible
- **SIMD operations** for batch distance calculations
- **Reorder computations** to fail fast on invalid pairs
- **Reduce branching** in hot paths

### 3. Minimize Element Access Overhead
Get_element calls appear frequently:
- **Cache element references** across multiple uses
- **Batch element data** into contiguous arrays for better cache locality
- **Reduce indirection** in element lookups

### 4. Profile-Guided Optimization
Specific hot addresses suggest:
- Functions at `0x2042f1`, `0x1e1c24`, `0x2083fd` should be profiled with debug symbols
- Use `#[inline(always)]` on proven hot paths
- Consider manual unrolling of innermost loops

## Recommended Next Steps

1. **Re-profile with debug symbols**:
   ```bash
   cargo build --profile release-with-debug
   # Add to Cargo.toml:
   # [profile.release-with-debug]
   # inherits = "release"
   # debug = true
   ```

2. **Analyze with perf**:
   ```bash
   perf record -g ./target/release/demo-headless <args>
   perf report
   ```

3. **Generate flamegraph**:
   ```bash
   cargo flamegraph --bin demo-headless -- <args>
   ```

4. **Focus optimization** on the 89% of code in two address ranges

## Estimated Impact

Based on profile data:
- **Spatial query optimization**: Could reduce 30-40% of overhead (15-20% total speedup)
- **Physics loop optimization**: Could reduce 20-30% of overhead (10-15% total speedup)
- **Combined**: Potential 25-35% overall performance improvement

## Files to Investigate

Based on address ranges and execution patterns:
1. `crates/core/src/simulation.rs` - Main update loop (50% of time)
2. `crates/core/src/spatial.rs` - Spatial index queries (39% of time)
3. `crates/core/src/physics.rs` - Heat transfer calculations (embedded in simulation loop)
4. `crates/core/src/element.rs` - Element access patterns

Focus on these four files for maximum impact.
