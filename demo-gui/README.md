# Demo GUI - Bevy-Based Fire Simulation Visualization

A real-time 3D visualization of the Australia Fire Simulation using the Bevy game engine.

## Overview

This demo provides an interactive, visually rich demonstration of the fire simulation with:

- **Real-time 3D rendering** of fuel elements and fire spread
- **Dynamic fire visualization** with temperature-based colors and glow effects
- **Interactive camera controls** for exploring the scene
- **Live statistics display** showing simulation metrics and weather conditions
- **Interactive controls** for pausing, speed adjustment, and fire suppression

## Features

### Visual Elements

- **Fuel Elements**: Rendered as 3D cubes
  - Green for unburned fuel
  - Orange/red with glow for burning elements (color based on temperature)
  - Gray/black for consumed fuel
- **Terrain**: 3D ground plane representing the simulation area
- **Fire Effects**: Dynamic colors and emissive lighting based on element temperature

### Controls

- **SPACE**: Pause/Resume simulation
- **[ / ]**: Decrease/Increase simulation speed (0.1x to 10x)
- **R**: Reset simulation to initial state
- **W**: Deploy water suppression (30 droplets in circular pattern)
- **Arrow Keys**: Camera controls
  - Up/Down: Move forward/backward
  - Left/Right: Rotate camera

### On-Screen Information

- **Simulation Time**: Current elapsed simulation time
- **Burning Elements**: Number of currently burning fuel elements
- **Fuel Consumed**: Total mass of fuel consumed (kg)
- **Max Temperature**: Highest temperature in the simulation (°C)
- **Weather Conditions**: Temperature, humidity, wind speed
- **Fire Danger**: McArthur FFDI and fire danger rating
- **Status**: RUNNING or PAUSED
- **Speed**: Current simulation speed multiplier

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

The demo will start with a pre-configured fire simulation:
- 10x10 grid of dry grass fuel elements
- Extreme fire danger weather conditions (35°C, 15% humidity, 20 m/s wind)
- 5 initially ignited elements
- Terrain with a central hill

## Implementation Details

### Architecture

The demo is structured using Bevy's Entity Component System (ECS):

- **Resources**: `SimulationState` manages the fire simulation and UI state
- **Components**: `FuelVisual`, `MainCamera`, `StatsText`, `ControlsText`
- **Systems**: 
  - `update_simulation`: Steps the fire simulation forward
  - `update_fuel_visuals`: Updates visual appearance based on simulation state
  - `update_camera_controls`: Handles camera movement
  - `update_ui`: Refreshes statistics display
  - `handle_controls`: Processes user input

### Simulation Integration

The demo wraps the `fire-sim-core` library and updates it at 10 FPS (0.1 second timesteps) with configurable speed multiplier. Visual updates occur at the monitor's refresh rate (typically 60 FPS) for smooth rendering.

### Performance

- Updates 100 fuel elements with burning/visual state every frame
- Smooth 60 FPS rendering on modern hardware
- Simulation complexity scales with number of burning elements

## Customization

You can modify the simulation parameters in `src/main.rs`:

```rust
impl Default for SimulationState {
    fn default() -> Self {
        // Adjust these parameters:
        let elements_x = 10;     // Grid width
        let elements_y = 10;     // Grid height
        let fuel_mass = 5.0;     // kg per element
        let spacing = 8.0;       // meters between elements
        
        // Weather configuration
        let weather = WeatherSystem::new(
            35.0,  // temperature (°C)
            0.15,  // humidity (fraction)
            20.0,  // wind speed (m/s)
            45.0,  // wind direction (degrees)
            10.0,  // drought factor
        );
        
        // ... rest of setup
    }
}
```

## Known Limitations

- Cannot run in headless/CI environments (requires display system)
- No ember particle visualization (elements only)
- Simplified terrain rendering (flat plane, not actual elevation mesh)
- Water suppression droplets not visualized (effects visible on fuel elements)

## Future Enhancements

Potential improvements for the GUI demo:

- [ ] Particle system for ember visualization
- [ ] Smoke plume rendering
- [ ] 3D terrain mesh with elevation
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
