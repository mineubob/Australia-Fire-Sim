# Australia Fire Simulation System

A scientifically accurate wildfire simulation system built in Rust, designed for emergency response training based on Australian bushfire research.

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
├── crates/
│   ├── core/                   # Fire simulation core
│   │   ├── src/
│   │   │   ├── lib.rs         # Main library exports
│   │   │   ├── core_types/    # Fuel, elements, weather
│   │   │   ├── grid/          # Atmospheric grid system
│   │   │   ├── physics/       # Heat transfer, combustion
│   │   │   └── simulation/    # Main simulation loop
│   │   └── Cargo.toml
│   └── ffi/                    # FFI for Unreal Engine
│       ├── src/
│       │   └── lib.rs         # C-compatible bindings
│       ├── cbindgen.toml      # Header generation config
│       └── Cargo.toml
├── demo-headless/              # Command-line demo
│   ├── src/
│   │   └── main.rs            # Text-based demo with stats
│   └── Cargo.toml
└── demo-gui/                   # Bevy-based GUI demo (NEW!)
    ├── src/
    │   └── main.rs            # 3D visualization with Bevy
    ├── README.md              # GUI demo documentation
    └── Cargo.toml
```

## Building

```bash
# Build all crates
cargo build --release

# Run tests
cargo test --release

# Run headless demo (no GUI required)
cargo run --release --bin demo-headless

# Run GUI demo (requires display system)
cargo run --release --bin demo-gui
```

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

Based on:
- **Rothermel Fire Spread Model** (1972)
- **McArthur Forest Fire Danger Index Mk5**
- **Byram's Intensity and Flame Height** equations
- **CSIRO Bushfire Research**: fuel classification, fire behavior
- **Stefan-Boltzmann Law**: radiant heat transfer
- **Bureau of Meteorology**: Australian fire weather

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

19 unit tests covering:
- Wind directionality (26x downwind vs 0.05x upwind)
- Moisture evaporation delays
- Vertical spread (fire climbs faster)
- Stringybark crown fire transitions
- FFDI calculations and scaling
- Ember physics (buoyancy, wind drift, cooling)
- Spatial indexing
- Fire spread simulation

## License

MIT

## Authors

Fire Sim Team
