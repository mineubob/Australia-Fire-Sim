# Bevy Demo UI Layout

## Window Layout

The demo window (1600x900 pixels) is divided into two main areas:

### Main Viewport (Left - ~1250px wide)
- **3D visualization area** showing the fire simulation from a top-down view
- Fire elements rendered as colored sprites:
  - Yellow/Orange: Low temperature fires (100-600°C)
  - Orange: Medium temperature fires (600-900°C)  
  - Red: High temperature fires (900°C+)
  - Dim yellow: Heated but not burning elements
- Ember particles rendered as small orange/red dots
- Camera can be panned with arrow keys and zoomed with +/-

### Control Panel (Right - 350px wide)
A vertical sidebar with egui UI containing:

#### Header Section
```
🔥 Fire Simulation Control
----------------------------------------
Time: 5.1s
Status: ▶ Running
----------------------------------------
```

#### Statistics Section  
```
📊 Statistics
Total Elements: 497
🔥 Burning: 497
✨ Embers: 242
🪵 Fuel Consumed: 57.1 kg
----------------------------------------
```

#### Fire Danger Section
```
⚠️ Fire Danger
FFDI: 13.0
Rating: High ← (color-coded based on danger level)
----------------------------------------
```

#### Weather Controls Section
```
🌤️ Weather
Temperature (°C): [====|==       ] 30.0
Humidity (%):     [====|==       ] 30.0
Wind Speed (km/h):[====|==       ] 30.0
Wind Direction (°):[            ] 0.0
Drought Factor:   [====|=        ] 5.0
----------------------------------------
```

#### Quick Presets Section
```
📍 Quick Presets
[🔥 Catastrophic]  [☀️ Perth Summer]
----------------------------------------
```

#### Fuel Spawning Section
```
🌳 Add Fuel
Click on map to place fuel:
[🌾 Grass] [🌲 Stringy]
[🌳 Smooth] [🌿 Shrub]
[🪵 Dead Wood]
----------------------------------------
```

#### Quick Actions Section
```
⚡ Actions
[🌾 Add Grass Field (5x5)]
[🌲 Add Stringybark Tree]
[🔥 Ignite Center]
----------------------------------------
```

#### Simulation Controls Section
```
[⏸ Pause]
[🔄 Reset]
----------------------------------------
```

#### Help Text Section
```
Controls:
Left Click: Add fuel
Right Click: Ignite
Arrow Keys: Pan
+/-: Zoom
Space: Pause
R: Reset
```

## Color Scheme

### Fire Danger Rating Colors:
- **Low**: Blue (#87CEEB)
- **Moderate**: Green (#32CD32)
- **High**: Gold (#FFD700)
- **Very High**: Orange (#FF8C00)
- **Severe**: Orange-Red (#FF4500)
- **Extreme**: Red (#FF0000)
- **CATASTROPHIC**: Dark Red (#8B0000)

### Fire Visualization Colors:
- **Low temp**: RGB(1.0, 0.8, 0.0) - Yellow
- **Medium temp**: RGB(1.0, 0.5, 0.0) - Orange
- **High temp**: RGB(1.0, 0.2, 0.0) - Red-Orange

## Interactive Features

1. **Sliders** - Smooth, draggable controls for all weather parameters
2. **Buttons** - Clickable buttons for presets, actions, and controls
3. **Radio buttons** - Fuel type selection with emoji icons
4. **Real-time updates** - Statistics update continuously as simulation runs
5. **Mouse interaction**:
   - Left click on viewport → Place fuel element
   - Right click on viewport → Ignite area
   - Hover over UI elements for feedback

## Example Simulation Visualization

When running with grass and trees:
- Small yellow/orange dots representing grass elements
- Larger clusters showing tree structures (trunk + crown)
- Animated flames spreading across the field
- Orange ember particles floating and drifting with wind
- Heat visualization showing elements warming up before ignition

## Window Title
```
Australia Fire Simulation - Bevy Demo
```

## Technical Details

- Window size: 1600x900 pixels
- UI panel: Fixed 350px width on right side
- Viewport: Remaining space on left
- Font: System default (egui default)
- Update rate: 10 Hz simulation, 60 FPS rendering
- Particle system: Up to 10,000 embers (sampled for display if >500)
