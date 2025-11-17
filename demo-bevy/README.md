# Australia Fire Simulation - Bevy Demo with UI

A visual, interactive demo of the fire simulation engine built with the Bevy game engine and egui UI.

## Features

- **Real-time 3D visualization** of fire spread
- **Interactive UI controls** for weather parameters
- **Fire element visualization** with color-coded flames based on temperature
- **Ember particle effects**
- **Statistics display** showing burning elements, embers, and fuel consumed
- **Fire danger rating** with FFDI calculation
- **Interactive camera controls** (pan and zoom)
- **Click-to-place fuel** and **click-to-ignite** functionality
- **Quick weather presets** (Catastrophic, Perth Summer, etc.)

## Requirements

- Rust 1.70 or later
- Graphics drivers (for GPU rendering)
- On Linux: X11 or Wayland display server

## Building

From the repository root:

```bash
cargo build --release -p demo-bevy
```

## Running

```bash
cargo run --release -p demo-bevy
```

Or run the built binary directly:

```bash
./target/release/demo-bevy
```

## Controls

### Keyboard
- **Arrow Keys**: Pan camera
- **+/-**: Zoom in/out
- **Space**: Pause/Resume simulation
- **R**: Reset simulation

### Mouse
- **Left Click**: Add fuel element at cursor position
- **Right Click**: Ignite fuel elements at cursor position

### UI Panel (Right Side)
- **Weather Controls**: Adjust temperature, humidity, wind speed/direction, and drought factor in real-time
- **Quick Presets**: Apply catastrophic conditions or regional weather patterns
- **Fuel Spawning**: Select fuel type to place (grass, stringybark, smooth bark, shrubland, dead wood)
- **Quick Actions**: 
  - Add 5x5m grass field at center
  - Add stringybark tree at center
  - Ignite center area
- **Statistics Display**: View real-time stats including:
  - Total fuel elements
  - Burning elements count
  - Active embers
  - Total fuel consumed
  - Fire Danger (FFDI) rating with color-coding

## Fire Danger Rating Colors

The UI color-codes fire danger ratings:
- **Blue**: Low
- **Green**: Moderate  
- **Gold**: High
- **Orange**: Very High
- **Orange-Red**: Severe
- **Red**: Extreme
- **Dark Red**: CATASTROPHIC

## Visualization

- **Yellow/Orange flames**: Lower temperature fires (100-600°C)
- **Orange flames**: Medium temperature fires (600-900°C)
- **Red flames**: High temperature fires (900°C+)
- **Dim yellow dots**: Heated but not burning elements (50-100°C)
- **Orange particles**: Active embers with varying sizes based on temperature

## Tips

1. Start with a grass field using the "Add Grass Field" button
2. Add some trees using the "Add Stringybark Tree" button
3. Click "Ignite Center" to start the fire
4. Adjust wind speed and direction to see directional fire spread
5. Try the "Catastrophic" preset to see extreme fire behavior
6. Watch ember generation and spotting in action

## Technical Notes

- Simulation runs at 10 Hz (0.1s timestep)
- Visual updates run at display refresh rate
- Embers are sampled for visualization (every 3rd ember if >500 active)
- Fire elements are scaled by 0.1x for better visualization
- All physics calculations use real-world fire science formulas

## Troubleshooting

### "Unable to find a GPU" error
Ensure you have proper graphics drivers installed. On Linux, you may need:
```bash
# For systems with integrated graphics
sudo apt-get install mesa-utils

# For NVIDIA GPUs
sudo apt-get install nvidia-driver-xxx
```

### Window doesn't appear
Check that your display server is running (X11 or Wayland).

### Poor performance
- Reduce the number of fuel elements by using smaller grass fields
- Limit the number of trees added
- The simulation is CPU-intensive due to physics calculations

## See Also

- [Main README](../README.md) - Core simulation documentation
- [Headless Demo](../demo-headless/README.md) - Command-line demo without graphics
