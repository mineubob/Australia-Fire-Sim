# Direct Suppression Application Guide

## Overview

The Australia Fire Sim now supports **direct suppression application**, which allows you to apply water, retardant, or foam immediately at specified coordinates without physics simulation.

This is useful for:
- Ground crew operations (hose lines, backpack sprayers)
- Instant suppression effects
- Simplified testing and demonstration
- Scenarios where physics simulation is not needed

The physics-based droplet system is still available for scenarios requiring realistic aerial drops.

## Rust API

### Core Library

```rust
use fire_sim_core::{FireSimulation, SuppressionAgent, Vec3, TerrainData};

// Create simulation
let terrain = TerrainData::flat(200.0, 200.0, 5.0, 0.0);
let mut sim = FireSimulation::new(5.0, terrain);

// Apply water directly at a position (300 kg)
sim.apply_suppression_direct(
    Vec3::new(100.0, 100.0, 5.0),  // Position (x, y, z)
    300.0,                          // Mass in kg
    SuppressionAgent::Water,        // Agent type
);

// Apply retardant
sim.apply_suppression_direct(
    Vec3::new(120.0, 100.0, 5.0),
    200.0,
    SuppressionAgent::LongTermRetardant,
);

// Apply foam
sim.apply_suppression_direct(
    Vec3::new(80.0, 100.0, 5.0),
    150.0,
    SuppressionAgent::Foam,
);
```

### Circular Pattern Application

For area coverage, you can apply suppression in a circular pattern:

```rust
let center_x = 100.0;
let center_y = 100.0;
let elevation = 5.0;
let radius = 25.0;
let total_mass = 300.0;
let num_points = 30;

for i in 0..num_points {
    let angle = i as f32 * std::f32::consts::PI * 2.0 / num_points as f32;
    let x = center_x + angle.cos() * radius;
    let y = center_y + angle.sin() * radius;
    
    sim.apply_suppression_direct(
        Vec3::new(x, y, elevation),
        total_mass / num_points as f32,
        SuppressionAgent::Water,
    );
}
```

## C/C++ FFI API

### Basic Usage

```c
#include "fire_sim_ffi.h"

// Create simulation
usize sim_id;
int result = fire_sim_create(200.0, 200.0, 5.0, 0, &sim_id);

// Apply water directly (300 kg at position)
result = fire_sim_apply_water_direct(
    sim_id,
    100.0,  // x
    100.0,  // y
    5.0,    // z
    300.0   // mass (kg)
);

// Apply other suppression agents
// agent_type: 0=Water, 1=ShortTermRetardant, 2=LongTermRetardant, 3=Foam
result = fire_sim_apply_suppression_direct(
    sim_id,
    120.0,  // x
    100.0,  // y
    5.0,    // z
    200.0,  // mass (kg)
    2       // LongTermRetardant
);
```

### Error Handling

```c
int result = fire_sim_apply_water_direct(sim_id, x, y, z, mass);

switch (result) {
    case FIRE_SIM_SUCCESS:
        printf("Suppression applied successfully\n");
        break;
    case FIRE_SIM_INVALID_ID:
        printf("Error: Invalid simulation ID\n");
        break;
    default:
        printf("Error: Unknown error code %d\n", result);
        break;
}
```

## Suppression Agent Types

### Water
- **Best for**: Quick cooling, immediate fire knockdown
- **Cooling capacity**: ~2590 kJ/kg
- **Coverage**: Moderate (evaporates quickly)
- **Side effects**: Increases humidity

### Short-Term Retardant
- **Best for**: Temporary fire control
- **Cooling capacity**: ~2500 kJ/kg
- **Coverage**: Good
- **Duration**: Short-term protection

### Long-Term Retardant
- **Best for**: Creating firebreaks, protecting structures
- **Cooling capacity**: ~2300 kJ/kg
- **Coverage**: Excellent (forms coating)
- **Duration**: Long-lasting protection

### Foam
- **Best for**: Suppressing flammable liquids, insulation
- **Cooling capacity**: ~3000 kJ/kg (highest)
- **Coverage**: Best (expands and adheres)
- **Side effects**: Increases humidity, forms insulating layer

## Effects on Grid Cells

When suppression is applied directly:

1. **Suppression Agent Concentration**: Increases in the affected cell(s)
   - Formula: `concentration_increase = mass / cell_volume`

2. **Temperature Reduction**: Immediate cooling effect
   - Based on agent cooling capacity
   - Example: 10 kg water can cool a 5m³ cell by ~40°C

3. **Humidity Increase**: For water and foam
   - Some agent immediately evaporates
   - Increases water vapor concentration
   - Raises relative humidity

4. **Fire Suppression**: Ongoing fire is suppressed
   - Burning elements in affected cells receive cooling
   - Reduces fire intensity
   - Can prevent ignition of nearby elements

## Comparison with Physics-Based Droplets

| Feature | Direct Application | Physics-Based Droplets |
|---------|-------------------|----------------------|
| **Application time** | Immediate | Takes time to fall |
| **Wind effects** | No | Yes - droplets drift |
| **Gravity effects** | No | Yes - droplets fall |
| **Realism** | Ground crews | Aerial drops |
| **Performance** | Faster | Slower (physics sim) |
| **Use case** | Ground operations | Aircraft operations |

## Best Practices

1. **Mass Selection**: Use realistic amounts
   - Ground hose: 50-100 kg/s
   - Fire truck: 500-2000 L/min (~8-33 kg/s)
   - Aerial drop: 3000-15000 L (~3000-15000 kg)

2. **Coverage Pattern**: Distribute over area
   - Use circular or grid patterns
   - 10-20 points for typical coverage
   - Radius: 20-30m for aerial drops

3. **Agent Selection**: Choose based on scenario
   - Water: Quick knockdown, abundant
   - Long-term retardant: Firebreaks, structure protection
   - Foam: Fuel coverage, hazmat situations

4. **Timing**: Apply when most effective
   - Before fire spreads to target area
   - At fire perimeter for containment
   - On structures for protection

## Examples in Repository

- **Demo GUI**: `demo-gui/src/main.rs` - Mouse click water drops
- **Demo Headless**: `demo-headless/src/main.rs` - CLI with suppression flag
- **Tests**: `crates/core/src/physics/suppression_physics.rs` - Unit tests

## Migration from Physics-Based System

**Old code** (physics-based droplets):
```rust
for i in 0..30 {
    let angle = i as f32 * std::f32::consts::PI * 2.0 / 30.0;
    let droplet = SuppressionDroplet::new(
        Vec3::new(x + angle.cos() * radius, y + angle.sin() * radius, altitude),
        Vec3::new(0.0, 0.0, -5.0),
        10.0,
        SuppressionAgent::Water,
    );
    sim.add_suppression_droplet(droplet);
}
```

**New code** (direct application):
```rust
for i in 0..30 {
    let angle = i as f32 * std::f32::consts::PI * 2.0 / 30.0;
    sim.apply_suppression_direct(
        Vec3::new(x + angle.cos() * radius, y + angle.sin() * radius, elevation),
        10.0,
        SuppressionAgent::Water,
    );
}
```

Benefits:
- Simpler code (no velocity parameter)
- Immediate effect (no waiting for droplets to fall)
- Better performance (no physics simulation per droplet)
- Same suppression effects on grid
