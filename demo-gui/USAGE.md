# Demo GUI Usage Examples

## Running the GUI Demo

```bash
# From repository root
cargo run --release --bin demo-gui
```

## What You'll See

When you run the demo-gui application, you'll see:

### 3D Scene
- **Fuel Elements**: 100 cubic elements (10x10 grid) arranged on terrain
  - **Green cubes**: Unburned fuel
  - **Orange/red cubes with glow**: Burning elements (color intensity based on temperature)
  - **Dark gray cubes**: Consumed/extinguished fuel
  
- **Terrain**: Brown ground plane representing the simulation area (200m x 200m)

- **Lighting**: Directional light simulating sunlight, plus ambient lighting

### On-Screen Display

Top of screen shows:
```
Australia Fire Simulation - Bevy Demo
```

Statistics display updates in real-time:
```
Time: 5.3s | Burning: 12 | Fuel Consumed: 8.5 kg | Max Temp: 847°C
Weather: 35°C, 15% RH, 20.0 m/s wind | FFDI: 85.3 (Extreme)
Status: RUNNING | Speed: 1.0x
```

Bottom shows control help:
```
Controls:
  SPACE - Pause/Resume
  [ / ] - Speed Down/Up
  R - Reset
  W - Add Water Suppression
  Arrow Keys - Camera
```

## Typical Fire Progression

### Phase 1: Ignition (0-5 seconds)
- 5 elements at center start burning (red/orange)
- Temperature rises rapidly to 600-800°C
- Fire begins spreading to adjacent elements

### Phase 2: Active Spread (5-30 seconds)
- Fire spreads outward from ignition points
- Wind pushes fire in NE direction (45°)
- More elements ignite as they receive radiant heat
- Peak burning elements: 15-20

### Phase 3: Consumption (30-60 seconds)
- Fuel depletes in initially ignited areas
- Elements turn dark gray as fuel is consumed
- Fire front continues moving as new fuel ignites
- Burning elements decrease as fuel runs out

## Interactive Controls

### Camera Movement
- **Up Arrow**: Move camera forward
- **Down Arrow**: Move camera backward
- **Left Arrow**: Rotate camera left
- **Right Arrow**: Rotate camera right

Camera starts at position (150, 120, 150) looking at center (100, 0, 100)

### Simulation Controls

**SPACE**: Pause/Resume
- Freezes simulation time
- All visuals remain visible
- Press again to resume

**[ (Left Bracket)**: Slow down
- Reduces speed by 50% (minimum 0.1x)
- Useful for detailed observation
- Example: 1.0x → 0.5x → 0.25x → 0.125x

**] (Right Bracket)**: Speed up
- Doubles speed (maximum 10x)
- Fast-forward through simulation
- Example: 1.0x → 2.0x → 4.0x → 8.0x

**R**: Reset
- Restarts simulation from beginning
- Resets to initial 5 ignited elements
- Clears all accumulated state

**W**: Water Suppression
- Drops 30 water droplets in circular pattern
- Each droplet: 10 kg at 60m altitude
- Effect: Cools burning fuel, may extinguish fire
- Can be pressed multiple times

## Expected Behavior

### Fire Dynamics
- Fire spreads **26x faster downwind** (NE direction due to 45° wind)
- **Vertical spread** is 2.5x faster than horizontal
- **Temperature varies**: 250°C (ignition) → 1200°C (peak) → 0°C (extinguished)
- **FFDI 85.3** indicates "Extreme" fire danger rating

### Performance
- Maintains **60 FPS** visual rendering
- Simulation updates at **10 FPS** (0.1s timesteps)
- Statistics update every frame
- Smooth camera movement

### Visual Feedback
- **Emissive glow** on burning elements (brighter = hotter)
- **Color transition**: Green → Orange → Red → Dark Gray
- **Real-time statistics** show fire progression

## Troubleshooting

### "Cannot create window" error
- System lacks display support (X11/Wayland/Windows/macOS native)
- Try running on local machine with GUI, not via SSH/CI

### Slow performance
- Reduce simulation speed with `[` key
- Close other applications
- Try on system with better GPU

### No visual changes
- Check if simulation is paused (Status shows "PAUSED")
- Press SPACE to resume
- Press R to reset if all fuel consumed

## Advanced Usage

To modify the initial simulation state, edit `src/main.rs`:

```rust
impl Default for SimulationState {
    fn default() -> Self {
        // Modify these values:
        let elements_x = 10;     // Grid width (default: 10)
        let elements_y = 10;     // Grid height (default: 10)
        let fuel_mass = 5.0;     // Mass per element in kg
        let spacing = 8.0;       // Distance between elements in meters
        
        // Adjust weather conditions:
        let weather = WeatherSystem::new(
            35.0,  // Temperature °C (default: 35)
            0.15,  // Humidity 0-1 (default: 0.15 = 15%)
            20.0,  // Wind speed m/s (default: 20)
            45.0,  // Wind direction degrees (default: 45 = NE)
            10.0,  // Drought factor (default: 10 = extreme)
        );
        
        // Number of initially ignited elements:
        for id in 0..5 {  // Change 5 to ignite more/fewer
            sim.ignite_element(id, 600.0);
        }
    }
}
```

Then rebuild: `cargo build --release -p demo-gui`

## Comparison to Headless Demo

| Feature | demo-gui | demo-headless |
|---------|----------|---------------|
| Visualization | 3D real-time | Text statistics |
| Interaction | Interactive | Command-line args |
| Display Required | Yes | No |
| Frame Rate | 60 FPS visual | N/A |
| Use Case | Presentation, training | Batch processing, CI |
| Output | Visual scene | Console text |

Both demos use the same underlying `fire-sim-core` simulation engine with identical physics.
