# Performance Metrics Guide

The demo-headless application now includes detailed performance metrics to help analyze simulation performance.

## Enabling Metrics

Add the `--show-metrics` flag when running the simulation:

```bash
./target/release/demo-headless -d 300 -i 1 -p perth-metro --num-trees 150 --show-metrics
```

## Metrics Display

### During Simulation

With metrics enabled, the output table includes additional columns:

```
Time(s) | Burning | Embers | PyroCb | Lightning | Fuel Consumed(kg) | Update(ms) | FPS
--------|---------|--------|--------|-----------|-------------------|------------|-----
    0.1 |       1 |      0 |      0 |         0 |              0.01 |       0.41 | 2427.3
   30.0 |    1027 |    307 |      0 |         0 |             11.22 |       2.01 |  498.2
   60.1 |    2155 |    655 |      0 |         0 |             29.35 |       3.66 |  273.4
```

**Column Descriptions:**
- **Update(ms)**: Time taken for the last simulation update in milliseconds
- **FPS**: Frames per second (1000 / Update(ms))

### Summary Statistics

At the end of simulation, detailed metrics are displayed:

```
=== Performance Metrics ===
Total simulation time: 27.91s
Total updates: 3000
Average update time: 9.30 ms
Min update time: 0.01 ms
Max update time: 19.95 ms
Average FPS: 107.5
Simulated time / Real time: 10.75x
Total elements: 196921
Elements processed per second: 21163530
```

**Metric Definitions:**

- **Total simulation time**: Wall clock time for entire simulation run
- **Total updates**: Number of simulation steps executed
- **Average update time**: Mean time per simulation step
- **Min/Max update time**: Fastest and slowest updates
- **Average FPS**: Mean frames per second (inverse of average update time)
- **Simulated time / Real time**: Speedup factor (how much faster than real-time)
- **Total elements**: Number of fuel elements in simulation
- **Elements processed per second**: Throughput metric (elements × updates / time)

## Interpreting Results

### Good Performance Indicators

- **Average FPS > 60**: Simulation runs faster than real-time video
- **Simulated time / Real time > 1.0**: Faster than real-time
- **Update time stays consistent**: No major performance degradation
- **Low max/average ratio**: Consistent frame times

### Performance Bottlenecks

- **Update time increases with burning elements**: Normal - more fire = more calculations
- **Large max/average ratio**: Some frames take much longer (check for spikes)
- **FPS drops below 30**: May need optimization or reduced scenario complexity

## Example Scenarios

### Small Test (Fast)
```bash
./target/release/demo-headless -d 60 -i 1 --num-trees 5 --show-metrics
```
Expected: ~200-500 FPS, 20-60x speedup

### Standard Scenario (Balanced)
```bash
./target/release/demo-headless -d 300 -i 1 --num-trees 150 --show-metrics
```
Expected: ~100-150 FPS, 10-15x speedup

### Stress Test (Heavy)
```bash
./target/release/demo-headless -d 300 -i 100 --num-trees 200 --show-metrics
```
Expected: ~50-100 FPS, 5-10x speedup

## Using Metrics for Optimization

### Identifying Hotspots

1. Run with metrics enabled
2. Note when update times spike
3. Check burning element count at those times
4. Compare with ember count and pyroCb activity

### Comparing Changes

To measure impact of code changes:

```bash
# Before changes
./target/release/demo-headless -d 300 -i 1 --num-trees 150 --show-metrics > before.txt

# Make changes and rebuild
cargo build --release

# After changes  
./target/release/demo-headless -d 300 -i 1 --num-trees 150 --show-metrics > after.txt

# Compare
grep "Average update time" before.txt after.txt
```

### Profiling Specific Scenarios

For targeted performance testing:

```bash
# High fire spread
./target/release/demo-headless -d 120 -i 100 --show-metrics

# High wind (faster spread)
./target/release/demo-headless -d 180 -w 50 --wind-direction 0 --show-metrics

# Large area
./target/release/demo-headless -d 300 --map-size 2000 --num-trees 300 --show-metrics
```

## Technical Details

### Update Cycle

Each update performs:
1. Weather calculations
2. Heat transfer for all burning elements (parallel)
3. Ember physics (parallel)
4. Ignition checks
5. PyroCb system updates
6. Lightning strike processing

### Performance Scaling

Performance scales with:
- **Number of burning elements** (O(n) where n is burning count)
- **Spatial density** (more elements near each other = more heat transfer calculations)
- **Active embers** (parallel processing helps but still has overhead)
- **PyroCb activity** (lightning strikes require spatial queries)

### Platform Considerations

Expected performance varies by hardware:
- **High-end desktop** (Ryzen 9, i9): 100-150 FPS average
- **Mid-range** (Ryzen 5, i5): 60-100 FPS average
- **Laptop** (mobile CPUs): 30-60 FPS average

The simulation uses Rayon for parallel processing, so performance scales well with core count.
