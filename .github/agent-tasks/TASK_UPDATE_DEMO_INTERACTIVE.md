# Task: Update demo-interactive to New Field-Based Simulation

## Overview

The `demo-interactive` project needs to be updated to work with the new field-based simulation system (`FieldSimulation`) instead of the old element-based system. This involves updating the API calls, removing element-specific functionality, and adapting the visualization to work with field data.

**Migration Type:** Element-based → Field-based simulation  
**Estimated Effort:** Medium (4-6 hours)  
**Breaking Changes:** Yes - Complete API redesign

---

## Phase 0: Workspace Registration

### Objective
Re-integrate `demo-interactive` into the workspace build system.

### Changes Required

**File:** `Cargo.toml` (workspace root)

**Line 2 - Add demo-interactive to members:**
```toml
# FROM:
members = ["crates/core", "crates/ffi"]

# TO:
members = ["crates/core", "crates/ffi", "demo-interactive"]
```

### Acceptance Criteria
- [ ] `cargo metadata --format-version=1 | grep demo-interactive` shows the package
- [ ] `cargo build --package demo-interactive` recognizes the package (may fail compilation)

---

## Phase 1: Core API Migration

### Objective
Update simulation initialization and core type imports to use the new field-based API.

### Changes Required

**File:** `demo-interactive/src/main.rs`

**1.1 Update Imports (lines ~54-56)**

```rust
// REMOVE:
use fire_sim_core::{
    core_types::{Celsius, Degrees, Kilograms, KilometersPerHour, Meters, Percent},
    ClimatePattern, FireSimulation, Fuel, FuelPart, TerrainData, Vec3, WeatherPreset,
    WeatherSystem,
};

// ADD:
use fire_sim_core::{
    core_types::{Celsius, Degrees, Kilograms, KilometersPerHour, Meters, Percent},
    FieldSimulation,        // Replaces FireSimulation
    solver::QualityPreset,  // New: grid quality settings
    TerrainData,
    Vec3,
    WeatherPreset,
    WeatherSystem,
    Fuel,                   // Optional: if needed
};
```

**1.2 Update App Struct**

Find the `App` struct field for simulation and change type:
```rust
// FROM:
simulation: FireSimulation,

// TO:
simulation: FieldSimulation,
```

**1.3 Update Simulation Creation**

Find initialization code (likely in `new()` or `reset_simulation()`):

```rust
// OLD:
let simulation = FireSimulation::new(terrain, weather);

// NEW:
let quality = QualityPreset::Medium;  // or Low/High/Ultra
let simulation = FieldSimulation::new(&terrain, quality, weather);
```

### Acceptance Criteria
- [ ] All imports resolve without errors
- [ ] `App` struct compiles with `FieldSimulation` field
- [ ] Simulation creation code compiles
- [ ] `cargo check --package demo-interactive` passes for Phase 1 changes

---

## Phase 2: Remove Element-Based Commands

### Objective
Remove or refactor all commands that reference individual fuel elements by ID.

### Commands to Remove

**File:** `demo-interactive/src/main.rs` - `COMMANDS` array

| Command | Alias | Action |
|---------|-------|--------|
| `element` | `e` | **REMOVE** - No element IDs exist |
| `nearby` | `n` | **REMOVE** - No element IDs exist |
| `ignite <id>` | `i` | **REMOVE** - Use `ignite_position` instead |
| `heat <id>` | `h` | **REMOVE** - Use `heat_position` instead |

### Commands to Refactor

| Command | Alias | Changes Needed |
|---------|-------|----------------|
| `burning` | `b` | Show fire front info instead of element list |
| `embers` | `em` | Show `sim.ember_count()` instead of list |

### Implementation

**2.1 Remove Element Commands**

In `COMMANDS` array, delete these `CommandInfo` entries:
- `CommandInfo { name: "element", ... }`
- `CommandInfo { name: "nearby", ... }`
- `CommandInfo { name: "ignite", alias: "i", ... }` (ID-based version)
- `CommandInfo { name: "heat", alias: "h", ... }` (ID-based version)

**2.2 Refactor `burning` Command**

```rust
CommandInfo {
    name: "burning",
    alias: "b",
    handler: |app, _parts| {
        let fire_front = app.simulation.fire_front();
        app.add_message(format!("Fire front: {} points", fire_front.points.len()));
        
        if app.headless {
            println!("Active fire perimeter: {} points", fire_front.points.len());
        }
    },
},
```

**2.3 Refactor `embers` Command**

```rust
CommandInfo {
    name: "embers",
    alias: "em",
    handler: |app, _parts| {
        let count = app.simulation.ember_count();
        app.add_message(format!("Active embers: {}", count));
        
        if app.headless {
            println!("Ember count: {}", count);
        }
    },
},
```

**2.4 Update Help Text**

Remove references to element-based commands in the module-level documentation.

### Acceptance Criteria
- [ ] Element-based commands removed from `COMMANDS` array
- [ ] `burning` command shows fire front information
- [ ] `embers` command shows ember count
- [ ] Help text updated
- [ ] `cargo check --package demo-interactive` passes

---

## Phase 3: Update Status and Visualization

### Objective
Replace element-based status displays with field-based metrics and visualization.

### Changes Required

**3.1 Update Status Display**

Find `show_status()` or equivalent method:

```rust
fn show_status(&mut self) {
    let (width, height, cell_size) = self.simulation.grid_dimensions();
    let gpu = self.simulation.is_gpu_accelerated();
    let burned = self.simulation.burned_area();
    let fuel = self.simulation.fuel_consumed();
    let time = self.simulation.simulation_time();
    let fire_front = self.simulation.fire_front();
    let embers = self.simulation.ember_count();
    
    self.add_message(format!("=== Simulation Status ==="));
    self.add_message(format!("Grid: {}x{} cells ({:.2}m/cell)", width, height, cell_size));
    self.add_message(format!("Backend: {}", if gpu { "GPU" } else { "CPU" }));
    self.add_message(format!("Burned area: {:.2} m²", burned));
    self.add_message(format!("Fuel consumed: {:.2} kg", fuel));
    self.add_message(format!("Simulation time: {:.2} s", time));
    self.add_message(format!("Fire front points: {}", fire_front.points.len()));
    self.add_message(format!("Active embers: {}", embers));
}
```

**3.2 Update Temperature Heatmap**

Find heatmap generation code:

```rust
fn generate_heatmap(&mut self, grid_size: usize) {
    let temp_data = self.simulation.read_temperature();
    let (width, height, _cell_size) = self.simulation.grid_dimensions();
    
    // Sample temperature field at grid_size resolution
    let step_x = width / grid_size as u32;
    let step_y = height / grid_size as u32;
    
    for gy in 0..grid_size {
        for gx in 0..grid_size {
            let x = gx as u32 * step_x;
            let y = gy as u32 * step_y;
            let idx = (y * width + x) as usize;
            let temp = temp_data[idx];
            
            // Render temperature cell
            // Color based on temp value (e.g., blue=cold, red=hot)
        }
    }
}
```

**3.3 Add Fire Front Visualization**

```rust
fn render_fire_front(&self, frame: &mut Frame, area: Rect) {
    let fire_front = self.simulation.fire_front();
    
    // Render fire front polyline
    for point in &fire_front.points {
        // Convert world coordinates to screen coordinates
        // Draw point or line segment at (point.x, point.y)
    }
}
```

### Acceptance Criteria
- [ ] Status display shows grid dimensions, GPU status, burned area, fuel consumed
- [ ] Temperature heatmap reads from `read_temperature()` field
- [ ] Fire front visualization implemented (optional: can be basic)
- [ ] `cargo build --package demo-interactive` succeeds
- [ ] Interactive mode displays new status correctly

---

## Phase 4: Testing and Validation

### Objective
Verify all functionality works correctly in both headless and interactive modes.

### Testing Checklist

**4.1 Build and Format**
- [ ] `cargo build --release --bin demo-interactive` succeeds
- [ ] `cargo clippy --package demo-interactive` has zero warnings
- [ ] `cargo fmt --check --package demo-interactive` passes

**4.2 Headless Mode Testing**

```bash
./target/release/demo-interactive --headless <<'HEREDOC'
1000
1000
p perth
ip 500 500 50
s 100
st
hm 20
q
HEREDOC
```

Expected output:
- [ ] Simulation initializes with 1000x1000m terrain
- [ ] Weather preset changes to Perth
- [ ] Ignition occurs at (500, 500) with 50m radius
- [ ] 100 timesteps execute without errors
- [ ] Status shows grid dimensions and field metrics
- [ ] Heatmap displays temperature values
- [ ] Program exits cleanly

**4.3 Interactive Mode Testing**

```bash
cargo run --release --bin demo-interactive
```

Test commands:
- [ ] `p catastrophic` - Changes weather preset
- [ ] `ip 75 75 25` - Ignites fire at position
- [ ] `s 50` - Steps simulation forward
- [ ] `st` - Shows status view with field metrics
- [ ] `hm 30` - Shows temperature heatmap
- [ ] `b` - Shows fire front info
- [ ] `em` - Shows ember count
- [ ] `w` - Shows weather view
- [ ] `?` - Shows help (verify element commands removed)
- [ ] `q` - Exits cleanly

**4.4 Edge Cases**
- [ ] Very small terrain (50x50m) works
- [ ] Very large terrain (5000x5000m) initializes
- [ ] Quality preset changes (Low/Medium/High/Ultra) work
- [ ] Multiple ignition points work
- [ ] Reset command works: `r 500 500`

**4.5 Performance**
- [ ] GPU acceleration detected if available
- [ ] CPU fallback works if GPU unavailable
- [ ] Simulation runs at acceptable framerate (>10 FPS in interactive mode)
- [ ] No memory leaks during extended runs

### Acceptance Criteria
- [ ] All build checks pass (build, clippy, fmt)
- [ ] All headless mode tests pass
- [ ] All interactive mode tests pass
- [ ] All edge cases handled gracefully
- [ ] Performance is acceptable
- [ ] Ready for integration into main branch

---

## API Reference

### Field-Based Simulation API

| Old Element-Based API | New Field-Based API | Notes |
|----------------------|---------------------|-------|
| `FireSimulation::new(terrain, weather)` | `FieldSimulation::new(&terrain, quality, weather)` | Requires `QualityPreset` |
| `sim.update(dt)` | `sim.update(dt)` | Unchanged |
| `sim.get_fuel_elements()` | **REMOVED** | No elements in field system |
| `sim.ignite_element(id)` | `sim.ignite_at(position, radius)` | Position-based |
| `sim.get_element_by_id(id)` | **REMOVED** | Query fields instead |
| `sim.burning_elements_count()` | `sim.fire_front().points.len()` | Fire perimeter points |
| `sim.embers()` | `sim.ember_count()` | Count only |
| N/A | `sim.read_temperature()` | New: field access |
| N/A | `sim.read_level_set()` | New: fire distance field |
| N/A | `sim.grid_dimensions()` | New: grid info |
| N/A | `sim.is_gpu_accelerated()` | New: backend info |
| N/A | `sim.burned_area()` | New: metric |
| N/A | `sim.fuel_consumed()` | New: metric |

### Weather API (Unchanged)

```rust
// Read-only access
let weather: &WeatherSystem = sim.weather();
let temp = weather.temperature;
let wind_speed = weather.wind_speed;
let humidity = weather.humidity;

// Mutable access
let weather_mut: &mut WeatherSystem = sim.weather_mut();
weather_mut.apply_preset(WeatherPreset::Catastrophic);
```

### Position-Based Commands (Keep These)

Already implemented in demo-interactive:
- `ignite_position <x> <y> [radius] [amount] [filters]` - Works with `sim.ignite_at()`
- `heat_position <x> <y> <temp> [radius] [amount] [filters]` - May need adapter

---

## Notes

- The new system is **field-based** (continuous) rather than **element-based** (discrete)
- There are no individual fuel elements with IDs anymore
- All operations work on spatial positions (x, y, z) instead of element IDs
- Temperature, fuel, and fire state are represented as 2D grids
- The fire front is extracted automatically using marching squares algorithm
- GPU acceleration is automatic (falls back to CPU if unavailable)
Important Notes

### Architecture Changes
- **Field-based** (continuous) replaces **element-based** (discrete)
- No individual fuel elements with IDs
- All operations use spatial positions (x, y, z)
- Temperature, fuel, and fire state are 2D grids
- Fire front extracted via marching squares algorithm
- GPU acceleration automatic with CPU fallback

### Migration Impact
- **Breaking:** All element-based commands removed
- **Breaking:** Element IDs no longer exist
- **Compatible:** Position-based commands (`ignite_position`, `heat_position`) still work
- **Compatible:** Weather system unchanged
- **New:** Field queries and visualization capabilities

### Performance Considerations
- Grid size controlled by `QualityPreset` (Low/Medium/High/Ultra)
- GPU acceleration provides 10-100x speedup on supported hardware
- CPU fallback always available
- Larger grids = higher accuracy but slower performance

---

## References

### Source Files
- New simulation: `crates/core/src/simulation/field_simulation.rs`
- Field solver API: `crates/core/src/solver/mod.rs`
- Quality presets: `crates/core/src/solver/quality.rs`
- Fire front extraction: `crates/core/src/solver/marching_squares.rs`
- Public API exports: `crates/core/src/lib.rs`

### Related Documentation
- `.github/copilot-instructions.md` - Project development guidelines
- `crates/core/README.md` - Core library documentation (if exists)
- `demo-interactive/README.md` - Demo usage guide (needs update after migration)