# Australia Fire Simulation System

A scientifically accurate wildfire simulation system built in Rust, based on Australian bushfire research.

Note: the simulation now always uses the advanced 3D mass-consistent wind field (Sherman 1978) — there is no runtime toggle to disable it.

## Overview

This is NOT a game - it's a physics-based simulation implementing real-world fire behavior including:

- **Discrete 3D fuel element system** (not grid-based)
- **Australian-specific fire physics** (eucalyptus oil explosions, stringybark ladder fuels)
- **Extreme wind directionality** (26x faster downwind spread)
- **Critical moisture evaporation** (heat goes to evaporation first)
- **Ember generation and spotting** (up to 25km for stringybark)
- **McArthur Forest Fire Danger Index** (FFDI)

## Key Features

### Physics-Based Fire Spread
- Stefan-Boltzmann radiation with view factors
- Convection for vertical heat transfer
- Wind effects: 26x downwind boost, 0.05x upwind suppression
- Vertical fire climbing: 2.5x+ faster upward
- Slope effects: exponential uphill boost

### Critical Moisture Evaporation
Heat MUST go to moisture evaporation FIRST (2260 kJ/kg latent heat) before temperature rise. This prevents thermal runaway and creates realistic ignition delays:
- 5% moisture fuel ignites quickly
- 20% moisture fuel takes 3x longer to ignite
- Temperature rise only occurs after moisture evaporation

### Australian-Specific Behaviors

**Eucalyptus Oil Vapor Explosions:**
- Oil vaporizes at 170°C
- Autoignition at 232°C
- Explosive energy: 43 MJ/kg

**Stringybark Ladder Fuels:**
- Crown fire transition at 30% normal intensity
- 650 kW/m bark ladder intensity
- 25km ember spotting distance

**McArthur FFDI:**
- Calculates fire danger from temperature, humidity, wind, drought
- Ratings: Low → Moderate → High → Very High → Severe → Extreme → CATASTROPHIC
- Directly scales fire spread rate

### Fuel Types

1. **Eucalyptus Stringybark** - 25km spotting, extreme ladder fuel, 0.04 oil content
2. **Eucalyptus Smooth Bark** - 10km spotting, less ladder fuel, 0.02 oil content
3. **Dry Grass** - fast ignition (250°C), rapid spread, minimal spotting
4. **Shrubland/Scrub** - medium ignition (300°C), moderate spread
5. **Dead Wood/Litter** - low moisture (5%), high susceptibility
6. **Green Vegetation** - high moisture (60%+), fire resistant

## Project Structure

```
Australia-Fire-Sim/
├── Cargo.toml                  # Workspace configuration
├── FireSimFFI.h                # Auto-generated C header (for game engines)
├── crates/
│   ├── core/                   # Fire simulation core
│   │   ├── src/
│   │   │   ├── lib.rs         # Main library exports
│   │   │   ├── core_types/    # Fuel, elements, weather
│   │   │   ├── grid/          # Atmospheric grid system
│   │   │   ├── physics/       # Heat transfer, combustion
│   │   │   └── simulation/    # Main simulation loop
│   │   └── Cargo.toml
│   └── ffi/                    # C FFI for game engines
│       ├── src/
│       │   └── lib.rs         # C-compatible bindings
│       ├── build.rs           # Auto-generates FireSimFFI.h
│       ├── cbindgen.toml      # Header generation config
│       ├── README.md          # FFI documentation
│       └── Cargo.toml
└── docs/                       # Integration guides
    └── integration/
        ├── unreal-engine-integration.md  # Unreal Engine 5 guide
        └── godot-integration.md          # Godot 4.x guide
```

## Building

```bash
# Build all crates
cargo build --release

# Build FFI library for game engines (auto-generates FireSimFFI.h)
cargo build --release -p fire-sim-ffi

# Library outputs:
# - Linux: target/release/libfire_sim_ffi.so
# - Windows: target/release/fire_sim_ffi.dll  
# - macOS: target/release/libfire_sim_ffi.dylib
# - Header: FireSimFFI.h (repo root, auto-generated)

# Run tests
cargo test --release

# Run headless demo (no GUI required)
cargo run --release --bin demo-headless

# Run GUI demo (requires display system)
cargo run --release --bin demo-gui
```

## Game Engine Integration

The simulation can be integrated into **Unreal Engine 5** and **Godot 4.x** via the C FFI layer. The C header (`FireSimFFI.h`) is **automatically generated** during builds.

### Quick Start for Game Engines

1. Build the FFI library: `cargo build --release -p fire-sim-ffi`
2. Copy library files to your game project
3. Use the auto-generated `FireSimFFI.h` header
4. Follow the integration guide for your engine:
   - **Unreal Engine 5**: [docs/integration/unreal-engine-integration.md](docs/integration/unreal-engine-integration.md)
   - **Godot 4.x**: [docs/integration/godot-integration.md](docs/integration/godot-integration.md)

### FFI Example (C/C++)

```cpp
#include "FireSimFFI.h"

// Create simulation (1000x1000m, 2m grid cells, flat terrain)
uintptr_t sim_id = 0;
fire_sim_create(1000.0f, 1000.0f, 2.0f, 0, &sim_id);

// Add fuel element (dry grass at position 500, 500, 0)
uint32_t elem_id = 0;
fire_sim_add_fuel(sim_id, 500.0f, 500.0f, 0.0f, 2, 10, 0.5f, -1, &elem_id);

// Ignite element
fire_sim_ignite(sim_id, elem_id, 600.0f);

// Set weather conditions
fire_sim_set_weather_from_live(sim_id, 35.0f, 20.0f, 40.0f, 45.0f, 1013.25f, 7.0f, 0.0f);

// Update simulation (timestep in seconds)
fire_sim_update(sim_id, 0.016f); // 60 FPS

// Query burning elements
ElementFireState states[100];
uint32_t count = 0;
fire_sim_get_burning_elements(sim_id, states, 100, &count);

// Apply water suppression
fire_sim_apply_water_direct(sim_id, 500.0f, 500.0f, 0.0f, 1000.0f);

// Cleanup
fire_sim_destroy(sim_id);
```

See `crates/ffi/README.md` for complete FFI documentation.

## Demos

### Headless Demo

The `demo-headless` crate provides a command-line interface for running simulations:
- Text-based statistics output
- Configurable fire size, duration, terrain
- Weather and suppression options
- Performance metrics

### GUI Demo (NEW!)

The `demo-gui` crate provides a real-time 3D visualization using the Bevy game engine:
- **Interactive 3D view** of fire spread
- **Dynamic fire visualization** with temperature-based colors
- **Camera controls** for scene exploration
- **Live statistics** showing simulation state and weather
- **Interactive controls** for pause, speed, and suppression

See [demo-gui/README.md](demo-gui/README.md) for detailed information.

**Requirements**: GUI demo requires a display system (X11/Wayland on Linux, native on Windows/macOS) and system libraries. See demo-gui README for installation instructions.

## Performance

- **600,000 fuel elements**: Spatial indexing with Morton encoding
- **1,000+ burning simultaneously**: 60 FPS target
- **10,000 active embers**: Parallel processing with Rayon
- **Update frequency**: 10-30 Hz physics update
- **Memory**: <2 GB for full simulation

Current demo: 7,845 elements with 4,500+ burning simultaneously.

## FFI for Unreal Engine

The FFI layer provides C-compatible functions for integration:

```c
// Create simulation
FireSimulation* sim = fire_sim_create(1000.0, 1000.0, 100.0);

// Add fuel element
uint32_t id = fire_sim_add_fuel_element(sim, x, y, z, fuel_type, part_type, mass, parent_id);

// Ignite element
fire_sim_ignite_element(sim, id, 600.0);

// Set weather
fire_sim_set_weather(sim, temp, humidity, wind_speed, wind_direction, drought);

// Update simulation
fire_sim_update(sim, dt);

// Get burning elements for rendering
uint32_t count;
FireElementVisual* elements = fire_sim_get_burning_elements(sim, &count);

// Get embers for particle effects
EmberVisual* embers = fire_sim_get_embers(sim, &count);

// Cleanup
fire_sim_destroy(sim);
```

## Scientific Basis

**✅ Scientifically Validated Against Published Research**

This simulation accurately implements peer-reviewed Australian bushfire dynamics. See [docs/SCIENTIFIC_VALIDATION.md](docs/SCIENTIFIC_VALIDATION.md) for comprehensive validation report.

Based on:
- **Rothermel Fire Spread Model** (1972)
- **McArthur Forest Fire Danger Index Mark 5** - Noble et al. (1980)
- **Byram's Intensity and Flame Height** equations (1959)
- **CSIRO Bushfire Research**: fuel classification, fire behavior
- **Stefan-Boltzmann Law**: radiant heat transfer (Stefan 1879, Boltzmann 1884)
- **Bureau of Meteorology**: Australian fire weather data
- **Eucalyptus Fire Behavior**: Pausas et al. (2017), Forest Education Foundation

### Key Research Papers
- Noble et al. (1980) - McArthur's fire-danger meters as equations
- Rothermel (1972) - Mathematical model for wildland fire spread
- Byram (1959) - Combustion of forest fuels
- Pausas et al. (2017) - Stringybark ember spotting
- Dowdy (2018) - Climatological variability of Australian fire weather
- Harris & Lucas (2019) - ENSO effects on Australian fire weather

See [docs/CITATIONS.bib](docs/CITATIONS.bib) for complete bibliography.

## Critical Implementation Details

### Avoiding Thermal Runaway
Always cap temperature at fuel-specific maximum:
```rust
self.temperature = self.temperature.min(self.fuel.max_flame_temperature);
```

### Moisture Evaporation First
Heat MUST evaporate moisture before raising temperature:
```rust
let evaporation_energy = moisture_mass * 2260.0; // kJ/kg
let heat_for_evaporation = heat_kj.min(evaporation_energy);
// ... evaporate moisture ...
let remaining_heat = heat_kj - heat_for_evaporation;
// ... then raise temperature ...
```

### Extreme Wind Directionality
```rust
if alignment > 0.0 {
    // Downwind: 26x at 10 m/s
    1.0 + alignment * wind_speed_ms * 2.5
} else {
    // Upwind: 0.05x minimum
    ((-alignment.abs() * wind_speed_ms * 0.35).exp()).max(0.05)
}
```

## Testing

42 unit tests covering:
- Wind directionality (26x downwind vs 0.05x upwind)
- Moisture evaporation delays
- Vertical spread (fire climbs faster)
- Stringybark crown fire transitions
- FFDI calculations and scaling
- Ember physics (buoyancy, wind drift, cooling)
- Spatial indexing
- Fire spread simulation

All tests passing ✅

## License

MIT

## Authors

Fire Sim Team
