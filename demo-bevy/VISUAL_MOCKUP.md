# Bevy Demo Visual Mockup

```
┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  Australia Fire Simulation - Bevy Demo                                           ⊡  ⊗  ⊠   │
├────────────────────────────────────────────────┬───────────────────────────────────────────┤
│                                                │  🔥 Fire Simulation Control              │
│                                                │  ─────────────────────────────────────── │
│         🟠🔴                                    │  Time: 5.1s                              │
│       🟠🔴🟡                                    │  Status: ▶ Running                       │
│     🟡🟠🔴🟡🟠                  ✨              │  ─────────────────────────────────────── │
│   🟠🟡🟠🔴🔴🟠🟡                               │  📊 Statistics                           │
│     🟡🟠🔴🟡🟠      ✨                         │  Total Elements: 497                     │
│       🟠🔴🟡              ✨                   │  🔥 Burning: 497                         │
│         🟠🔴                                    │  ✨ Embers: 242                          │
│            ✨                                  │  🪵 Fuel Consumed: 57.1 kg               │
│                    ✨                          │  ─────────────────────────────────────── │
│                          ✨                    │  ⚠️ Fire Danger                          │
│    . . . . . . . . . . .                       │  FFDI: 13.0                              │
│    . . . . . . . . . . .                       │  Rating: High [in yellow/gold]           │
│    . . . . . . . . . . .                       │  ─────────────────────────────────────── │
│    . . . . . . . . . . .      🌲               │  🌤️ Weather                              │
│    . . . . . . . . . . .                       │  Temperature (°C): [====|=====      ] 30 │
│    . . . . . . . . . . .                       │  Humidity (%):     [====|=====      ] 30 │
│    . . . . . . . . . . .      🌲               │  Wind Speed (km/h):[====|=====      ] 30 │
│    . . . . . . . . . . .                       │  Wind Direction:   [===============  ] 0 │
│    . . . . . . . . . . .                       │  Drought Factor:   [====|==========  ] 5 │
│                                                │  ─────────────────────────────────────── │
│                                                │  📍 Quick Presets                        │
│                                                │  ┌──────────────┐  ┌──────────────┐     │
│                                                │  │🔥Catastrophic│  │☀️Perth Summer │     │
│                                                │  └──────────────┘  └──────────────┘     │
│                                                │  ─────────────────────────────────────── │
│  Camera: Arrow Keys to Pan, +/- to Zoom       │  🌳 Add Fuel                             │
│                                                │  Click on map to place fuel:             │
│                                                │  [🌾Grass]  [🌲Stringy]                  │
│                                                │  [🌳Smooth]  [🌿Shrub]                   │
│                                                │  [🪵Dead Wood]                           │
│                                                │  ─────────────────────────────────────── │
│                                                │  ⚡ Actions                               │
│                                                │  ┌──────────────────────────────┐        │
│                                                │  │🌾 Add Grass Field (5x5)      │        │
│                                                │  └──────────────────────────────┘        │
│                                                │  ┌──────────────────────────────┐        │
│                                                │  │🌲 Add Stringybark Tree       │        │
│                                                │  └──────────────────────────────┘        │
│                                                │  ┌──────────────────────────────┐        │
│                                                │  │🔥 Ignite Center              │        │
│                                                │  └──────────────────────────────┘        │
│                                                │  ─────────────────────────────────────── │
│                                                │  ┌──────────────┐  ┌──────────────┐     │
│                                                │  │   ⏸ Pause   │  │   🔄 Reset   │     │
│                                                │  └──────────────┘  └──────────────┘     │
│                                                │  ─────────────────────────────────────── │
│                                                │  Controls:                               │
│                                                │  Left Click: Add fuel                    │
│                                                │  Right Click: Ignite                     │
│                                                │  Arrow Keys: Pan                         │
│                                                │  +/-: Zoom                               │
│                                                │  Space: Pause                            │
│                                                │  R: Reset                                │
└────────────────────────────────────────────────┴───────────────────────────────────────────┘

Legend:
🔴 = High temperature fire (900°C+, red)
🟠 = Medium temperature fire (600-900°C, orange)  
🟡 = Low temperature fire (100-600°C, yellow)
✨ = Active embers drifting with wind
. = Unburned grass fuel elements
🌲 = Eucalyptus tree (trunk + crown structure)
```

## Fire Visualization Details

### Color Coding by Temperature:
- **🟡 Yellow (FIRE_COLOR_LOW)**: 100-600°C - Early stage fires, low intensity
- **🟠 Orange (FIRE_COLOR_MEDIUM)**: 600-900°C - Active burning, medium intensity
- **🔴 Red (FIRE_COLOR_HIGH)**: 900°C+ - Intense fires, high temperatures

### Particle Effects:
- **✨ Embers**: Small orange/red particles that drift with wind
  - Size varies with temperature
  - Fade as they cool below 200°C
  - Can travel long distances (up to 25km for stringybark)

### Fire Spread Animation:
The visualization shows fire spreading from the ignition point:
1. Initial ignition creates hot spots (red)
2. Heat radiates to nearby fuel (yellow warming)
3. Fuel ignites and burns (orange/red)
4. Embers are generated and carried by wind
5. Embers can create spot fires downwind

### Interactive Elements:
- **Left click** on the viewport places fuel at cursor position
- **Right click** ignites all fuel within a radius of the cursor
- **Arrow keys** pan the camera around the fire scene
- **+/- keys** zoom in/out to see details or overview

### UI Panel Features:
1. **Real-time Statistics**: Update every frame showing current state
2. **Interactive Sliders**: Drag to adjust weather parameters in real-time
3. **Color-Coded FFDI**: Changes color from blue (Low) to dark red (CATASTROPHIC)
4. **Emoji Icons**: Visual indicators for fuel types and actions
5. **Responsive Buttons**: Clickable with hover effects

## Example Scenarios

### Scenario 1: Grass Fire with Wind
```
Wind: 40 km/h from West (270°)
Temperature: 35°C
Humidity: 20%
FFDI: 25 (Very High)

Fire spreads rapidly eastward due to wind:
    🟡🟡       →  
  🟡🟠🔴🟠🟡      ✨✨✨
    🟡🟡
```

### Scenario 2: Crown Fire in Eucalyptus
```
Multiple stringybark trees with oil vapor explosions:

      🔴         Intense crown fire
    🔴🔴🔴        Oil vapor autoignition
  🔴🔴🌲🔴🔴       
    🔴🔴         Trunk engulfed
     🟠          Lower trunk heating
```

### Scenario 3: Spot Fire from Embers
```
Main fire generates embers that travel downwind:

🔴🟠🟡 Main fire ----✨---✨---✨--> New ignition 🟡
                     Wind carries embers
                     up to 25km distance
```

## Performance Notes

The demo maintains smooth 60 FPS rendering while the physics simulation runs at 10 Hz (0.1s timesteps). This separation allows:
- Smooth visual updates regardless of simulation complexity
- Consistent physics calculations
- Responsive UI even during intense fire scenarios

With 500+ burning elements and 200+ embers, the visualization remains fluid and interactive.
