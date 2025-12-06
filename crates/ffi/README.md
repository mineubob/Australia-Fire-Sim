# Fire Simulation FFI

C-compatible Foreign Function Interface (FFI) for the Australia Fire Simulation.

## Overview

This crate provides a C API for integrating the Rust fire simulation core into game engines and other C/C++ applications like Unreal Engine and Godot.

## Automatic C Header Generation

The C header file (`FireSimFFI.h`) is **automatically generated** during compilation via `build.rs` using [cbindgen](https://github.com/mozilla/cbindgen).

### Building

```bash
# Build the FFI library (automatically generates FireSimFFI.h in crate directory `crates/ffi`)
cargo build --release -p fire-sim-ffi

# Output files:
# - Linux: target/release/libfire_sim_ffi.so
# - Windows: target/release/fire_sim_ffi.dll
# - macOS: target/release/libfire_sim_ffi.dylib
# - Header: FireSimFFI.h (crate root of this crate)
```

> Note: `FireSimFFI.h` is generated automatically by the build (via `cbindgen` + `build.rs`) and is ignored by git (see `.gitignore`). If you must ship the header to a consumer who cannot run cbindgen, add it explicitly with `git add -f crates/ffi/FireSimFFI.h`.
```

### Configuration

Header generation is configured in `cbindgen.toml`. The build script:
- Runs automatically on every build
- Generates `FireSimFFI.h` in the repository root
- Reruns if `src/lib.rs` or `cbindgen.toml` changes

## Integration Guides

See the integration documentation for step-by-step instructions:

- **Unreal Engine 5**: `docs/integration/unreal-engine-integration.md`
- **Godot 4.x**: `docs/integration/godot-integration.md`

## API Overview

The FFI provides:

### Lifecycle Functions
- `fire_sim_create()` - Create simulation with terrain
- `fire_sim_destroy()` - Clean up simulation
- `fire_sim_update()` - Advance simulation by timestep

### Fuel Management
- `fire_sim_add_fuel()` - Add fuel element to simulation
- `fire_sim_ignite()` - Ignite a fuel element
- `fire_sim_get_element_state()` - Query element fire state

### Suppression
- `fire_sim_apply_suppression_to_elements()` - Apply suppression agent to area
- `fire_sim_apply_water_direct()` - Direct water application

### Terrain Queries
- `fire_sim_get_elevation()` - Get terrain elevation
- `fire_sim_terrain_slope()` - Get slope in degrees
- `fire_sim_terrain_aspect()` - Get aspect in degrees
- `fire_sim_terrain_slope_multiplier()` - Calculate slope spread effect

### Weather
- `fire_sim_set_weather_from_live()` - Update weather conditions
- `fire_sim_get_haines_index()` - Get fire weather severity (2-6)

### Statistics
- `fire_sim_get_stats()` - Get simulation statistics
- `fire_sim_get_burning_elements()` - Get array of burning elements
- `fire_sim_get_snapshot()` - Get complete simulation state

### Multiplayer
- `fire_sim_submit_player_action()` - Submit player action for replication
- `fire_sim_get_frame_number()` - Get current frame for synchronization

## Error Codes

All functions return error codes for robust error handling:

```c
#define FIRE_SIM_SUCCESS           0   // Success
#define FIRE_SIM_INVALID_ID       -1   // Invalid simulation ID
#define FIRE_SIM_NULL_POINTER     -2   // Null pointer passed
#define FIRE_SIM_INVALID_FUEL     -3   // Invalid fuel/agent type
#define FIRE_SIM_INVALID_TERRAIN  -4   // Invalid terrain type
#define FIRE_SIM_LOCK_ERROR       -5   // Internal lock error
```

## Thread Safety

The FFI uses thread-safe interior mutability with `RwLock` for concurrent access. Multiple simulations can run simultaneously, identified by unique `sim_id` values.

## Example Usage (C++)

```cpp
#include "FireSimFFI.h"

// Create simulation
uintptr_t sim_id = 0;
int result = fire_sim_create(1000.0f, 1000.0f, 2.0f, 0, &sim_id);
if (result != FIRE_SIM_SUCCESS) {
    // Handle error
}

// Add fuel element
uint32_t elem_id = 0;
fire_sim_add_fuel(sim_id, 500.0f, 500.0f, 0.0f, 2, 10, 0.5f, -1, &elem_id);

// Ignite
fire_sim_ignite(sim_id, elem_id, 600.0f);

// Update simulation
fire_sim_update(sim_id, 0.016f); // 60 FPS

// Query burning elements
ElementFireState states[100];
uint32_t count = 0;
fire_sim_get_burning_elements(sim_id, states, 100, &count);

// Clean up
fire_sim_destroy(sim_id);
```

## Scientific Accuracy

This FFI exposes the full scientific fire simulation including:
- Rothermel (1972) surface fire spread
- Van Wagner (1977) crown fire transitions
- Albini (1979, 1983) ember spotting physics
- Nelson (2000) fuel moisture dynamics
- McArthur FFDI fire danger rating

See the core crate documentation for details on the physics models.
