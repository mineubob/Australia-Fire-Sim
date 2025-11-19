# Demo GUI - Bevy-Based Fire Simulation Visualization

A real-time 3D visualization of the Australia Fire Simulation using the Bevy game engine.

## Overview

This demo provides an interactive, visually rich demonstration of the fire simulation with:

- **Main Menu System** with full configuration before starting
- **Real-time 3D rendering** of fuel elements and fire spread
- **3D terrain mesh** with actual elevation data (hills, valleys, or flat terrain)
- **Fully customizable** simulation parameters via interactive menu
- **Dynamic fire visualization** with temperature-based colors and glow effects
- **Interactive camera controls** for exploring the scene
- **Right-side statistics panel** with organized simulation metrics
- **FPS counter** showing real-time frame rate
- **Hover tooltips** showing detailed fuel element information
- **Interactive controls** for pausing, speed adjustment, and fire suppression

## Features

### Main Menu System ✨ NEW!

When you start the demo, you'll see a configuration menu where you can customize:

- **Terrain Settings**: Map width/height, terrain type (Flat/Hill/Valley)
- **Fire Settings**: Grid size, fuel mass, fuel type (5 options), ignitions, spacing
- **Weather Settings**: Temperature, humidity, wind speed/direction, drought factor

Use **+/-** buttons to adjust numeric values and click cycle buttons to change terrain and fuel types. Click **START SIMULATION** when ready!

### Visual Elements

- **Fuel Elements**: Rendered as 3D cubes
  - Green for unburned fuel
  - Orange/red with glow for burning elements (color based on temperature)
  - Gray/black for consumed fuel
  - **Hover to see details**: Position, temperature, fuel remaining, moisture, status, flame height
- **3D Terrain**: Mesh generated from actual elevation data with proper normals
  - Follows terrain type (flat, single hill, or valley between hills)
  - Visual representation matches simulation physics
- **Fire Effects**: Dynamic colors and emissive lighting based on element temperature
- **Statistics Panel**: Semi-transparent box on right side with organized stats
  - Simulation status (time, speed, paused/running)
  - Fire status (burning elements, fuel consumed, max temperature)
  - Weather conditions (temperature, humidity, wind)
  - Fire danger rating (FFDI and category)
- **FPS Counter** ✨ NEW!: Top-right corner shows real-time frame rate

### Customization

The simulation is **fully customizable** via the main menu at startup. You can also edit `src/main.rs` for advanced configuration:

**Terrain Settings:**
- Map dimensions (width, height)
- Terrain type (Flat, Hill, Valley)

**Fire Settings:**
- Grid size (elements_x, elements_y)
- Fuel type (DryGrass, EucalyptusStringybark, EucalyptusSmoothBark, Shrubland, DeadWood)
- Fuel mass per element
- Element spacing
- Number of initial ignitions

**Weather Settings:**
- Temperature (°C)
- Humidity (0-1 fraction)
- Wind speed (m/s)
- Wind direction (degrees)
- Drought factor

Example customization in `main()`:
```rust
let config = DemoConfig {
    map_width: 300.0,
    map_height: 300.0,
    terrain_type: TerrainType::Valley,
    elements_x: 15,
    elements_y: 15,
    fuel_type: FuelType::EucalyptusStringybark,
    fuel_mass: 8.0,
    spacing: 10.0,
    initial_ignitions: 10,
    temperature: 40.0,
    humidity: 0.10,
    wind_speed: 25.0,
    wind_direction: 90.0,  // East wind
    drought_factor: 12.0,   // Catastrophic
};
```

### Controls

**In Menu:**
- **+/-** buttons: Adjust numeric values
- **Cycle buttons**: Change terrain type and fuel type
- **START SIMULATION**: Begin the simulation with current config

**During Simulation:**
- **SPACE**: Pause/Resume simulation
- **[ / ]**: Decrease/Increase simulation speed (0.1x to 10x)
- **R**: Reset simulation (returns to menu)
- **W**: Deploy water suppression (30 droplets in circular pattern)
- **Arrow Keys**: Camera controls
  - Up/Down: Move forward/backward
  - Left/Right: Rotate camera
- **Mouse Hover**: Show detailed information about fuel elements

### User Interface

**Main Menu Screen:**
- Configuration panel with all simulation parameters
- Increment/decrement buttons for all values
- START button to begin simulation

**Simulation Screen:**

**Top-Right Corner:**
- **FPS Counter** ✨ NEW!: Real-time frame rate display

**Left Side:**
- Title and control instructions

**Right Side - Statistics Panel:**
Organized display with semi-transparent background showing:
- **Simulation Status**: Time, status (RUNNING/PAUSED), speed multiplier
- **Fire Status**: Number of burning elements, total elements, fuel consumed, max temperature
- **Weather Conditions**: Temperature, humidity, wind speed/direction, drought factor
- **Fire Danger**: FFDI value and danger rating

**Hover Tooltips:**
When you hover the mouse over a fuel element, a tooltip appears showing:
- Element ID and position coordinates
- Current temperature
- Fuel remaining (kg)
- Moisture percentage
- Status (BURNING/CONSUMED/Unburned)
- Flame height

## Requirements

### System Requirements

- **Display System**: Requires a window system (X11/Wayland on Linux, native on Windows/macOS)
- **Graphics**: OpenGL 3.3+ or Vulkan-capable GPU
- **RAM**: At least 2GB available
- **OS**: Windows, macOS, or Linux with GUI support

### Dependencies

The demo uses Bevy 0.14, which requires system libraries:

**Linux:**
```bash
# Ubuntu/Debian
sudo apt-get install libasound2-dev libudev-dev pkg-config

# For X11
sudo apt-get install libx11-dev libxcursor-dev libxi-dev libxrandr-dev

# For Wayland (optional)
sudo apt-get install libwayland-dev libxkbcommon-dev
```

**macOS:**
No additional dependencies required.

**Windows:**
No additional dependencies required.

## Building

```bash
# From the repository root
cargo build --release -p demo-gui
```

## Running

```bash
# From the repository root
cargo run --release -p demo-gui
```

The demo will start with default configuration:
- 10x10 grid of dry grass fuel elements
- Extreme fire danger weather conditions (35°C, 15% humidity, 20 m/s wind)
- 5 initially ignited elements
- Terrain with a central hill (80m elevation)
- 3D terrain mesh following actual elevation data

## Implementation Details

### Architecture

The demo is structured using Bevy's Entity Component System (ECS):

- **Resources**: 
  - `DemoConfig`: Configuration for simulation parameters
  - `SimulationState`: Manages the fire simulation and UI state
- **Components**: `FuelVisual`, `MainCamera`, `StatsText`, `ControlsText`
- **Systems**: 
  - `update_simulation`: Steps the fire simulation forward
  - `update_fuel_visuals`: Updates visual appearance based on simulation state
  - `update_camera_controls`: Handles camera movement
  - `update_ui`: Refreshes statistics display
  - `handle_controls`: Processes user input

### 3D Terrain Rendering

The terrain is rendered as a 3D mesh with actual elevation data:
- Samples elevation every 5 meters from the terrain model
- Generates proper vertex normals for realistic lighting
- Matches the physics simulation's terrain exactly
- Supports flat, hill, and valley terrain types

### Simulation Integration

The demo wraps the `fire-sim-core` library and updates it at 10 FPS (0.1 second timesteps) with configurable speed multiplier. Visual updates occur at the monitor's refresh rate (typically 60 FPS) for smooth rendering.

### Performance

- Updates fuel elements with burning/visual state every frame
- Smooth 60 FPS rendering on modern hardware
- Simulation complexity scales with number of burning elements
- Terrain mesh optimized with efficient indexing

## Advanced Customization

See the **Customization** section at the top of this README for details on configuring simulation parameters.

For more complex customizations, you can edit `src/main.rs`:
- Add new terrain types
- Implement custom fuel types
- Modify visual effects
- Add additional UI elements

## Known Limitations

- Cannot run in headless/CI environments (requires display system)
- No ember particle visualization (elements only)
- Water suppression droplets not visualized (effects visible on fuel elements)

## Future Enhancements

Potential improvements for the GUI demo:

- [ ] Particle system for ember visualization
- [ ] Smoke plume rendering
- [x] 3D terrain mesh with elevation ✅ **IMPLEMENTED**
- [x] Customizable simulation parameters ✅ **IMPLEMENTED**
- [ ] More detailed fire effects (flames, heat distortion)
- [ ] Save/load simulation state
- [ ] Recording/playback functionality
- [ ] Interactive fuel placement and ignition
- [ ] Multiple camera views
- [ ] Performance profiling overlay

## Screenshots

When running on a system with a display, you'll see:
- 3D view of the fire simulation from an elevated perspective
- Fuel elements colored by state (green→orange→red→black)
- Glowing fire effects on burning elements
- Real-time statistics overlay
- Control help text

## License

MIT - See repository root LICENSE file
