# UI Features Guide

## Overview

The demo-gui features a modern, informative user interface designed for effective fire simulation visualization and monitoring.

## Layout

### Screen Organization

```
┌─────────────────────────────────────────────────────────────────┐
│ Australia Fire Simulation              [Statistics Panel]       │
│                                        │                    │    │
│ Controls:                              │ SIMULATION         │    │
│   SPACE - Pause/Resume                 │ Time: 25.3s        │    │
│   [ / ] - Speed                        │ Status: RUNNING    │    │
│   R - Reset                            │ Speed: 1.0x        │    │
│   W - Water Suppression                │                    │    │
│   Arrow Keys - Camera                  │ FIRE STATUS        │    │
│   Hover - Element Details              │ Burning: 12        │    │
│                                        │ Total: 100         │    │
│                                        │ Consumed: 8.5 kg   │    │
│         [3D Scene Area]                │ Max Temp: 847°C    │    │
│                                        │                    │    │
│                                        │ WEATHER            │    │
│                                        │ Temp: 35°C         │    │
│                                        │ Humidity: 15%      │    │
│         [Tooltip]                      │ Wind: 20.0 m/s     │    │
│         Element #45                    │ Direction: 45°     │    │
│         Position: (98, 102, 0.5)       │ Drought: 10.0      │    │
│         Temperature: 623°C             │                    │    │
│         Fuel: 3.2 kg                   │ FIRE DANGER        │    │
│         Status: BURNING                │ FFDI: 85.3         │    │
│                                        │ Rating: Extreme    │    │
└─────────────────────────────────────────────────────────────────┘
```

## Statistics Panel (Right Side)

### Location & Styling
- Fixed position on right side of screen
- Width: 400px
- Semi-transparent dark background (RGBA: 0.1, 0.1, 0.1, 0.85)
- 15px padding for content spacing
- 10px margin from edge
- Max height: 90% of screen

### Content Sections

#### 1. Header
- Text: "SIMULATION STATISTICS"
- Color: Gold/Yellow (1.0, 0.8, 0.2)
- Font size: 20px
- Stands out from other text

#### 2. Simulation Status
```
Simulation Time: 25.3s
Status: RUNNING (or PAUSED)
Speed: 1.0x
```

#### 3. Fire Status
```
FIRE STATUS
Burning Elements: 12
Total Elements: 100
Fuel Consumed: 8.5 kg
Max Temperature: 847°C
```
- Shows current active fire
- Tracks consumption
- Peak temperature across all elements

#### 4. Weather Conditions
```
WEATHER CONDITIONS
Temperature: 35°C
Humidity: 15%
Wind Speed: 20.0 m/s
Wind Direction: 45°
Drought Factor: 10.0
```
- All parameters that affect fire behavior
- Real-time values from weather system

#### 5. Fire Danger Rating
```
FIRE DANGER
FFDI: 85.3
Rating: Extreme
```
- McArthur FFDI calculation
- Danger category (Low/Moderate/High/Very High/Severe/Extreme/CATASTROPHIC)

## Hover Tooltips

### Activation
- Move mouse cursor over any fuel element (3D cube)
- Tooltip appears automatically
- Updates in real-time as simulation runs

### Detection Method
- Raycasting from camera through cursor position
- Sphere collision detection (2m radius)
- Selects closest element if multiple in ray path

### Displayed Information

```
Fuel Element #45
Position: (98.0, 102.0, 0.5)
Temperature: 623°C
Fuel Remaining: 3.24 kg
Moisture: 12.5%
Status: BURNING
Flame Height: 2.85 m
```

**Field Descriptions:**
- **Element #**: Unique identifier for the fuel element
- **Position**: (X, Y, Z) coordinates in meters
- **Temperature**: Current temperature in Celsius
- **Fuel Remaining**: Mass of unburned fuel in kg
- **Moisture**: Water content as percentage
- **Status**: BURNING, CONSUMED, or Unburned
- **Flame Height**: Current flame height in meters (Byram's formula)

### Tooltip Styling
- Black background with 90% opacity
- White text, 14px font
- 8px padding
- Positioned 15px offset from cursor (right and down)
- Automatically hidden when not hovering

## Color Coding

### Fuel Element Colors
- **Green**: Unburned fuel (ready to ignite)
- **Orange/Red**: Burning (intensity based on temperature)
- **Dark Gray/Black**: Consumed (no fuel remaining)

### Text Colors
- **White**: Primary text (title)
- **Gold/Yellow**: Section headers
- **Light Gray**: Statistics and data
- **Dark Gray**: Control instructions

## Interactive Controls

All controls are documented in the left panel:

### Keyboard Controls
- **SPACE**: Toggle pause/resume
- **[**: Decrease speed (×0.5)
- **]**: Increase speed (×2.0)
- **R**: Reset simulation
- **W**: Deploy water suppression
- **Arrow Keys**: Camera navigation
  - Up: Move forward
  - Down: Move backward
  - Left: Rotate left
  - Right: Rotate right

### Mouse Controls
- **Hover**: Show element details
- **Movement**: Explore scene (camera follows arrow keys)

## Usage Tips

### Monitoring Fire Progression
1. Watch "Burning Elements" in stats panel
2. Track "Max Temperature" for fire intensity
3. Check "FFDI" for danger assessment

### Exploring Elements
1. Navigate camera with arrow keys
2. Position element in center of view
3. Hover mouse over element
4. Read detailed stats in tooltip

### Analyzing Behavior
1. Note temperature differences between elements
2. Track moisture depletion over time
3. Observe flame height correlation with intensity
4. Monitor fuel consumption rate

### Performance Monitoring
- "Speed" shows simulation rate multiplier
- Adjust with [ ] keys if simulation is slow
- Status shows PAUSED when simulation is frozen

## Technical Details

### Update Frequency
- Statistics: Every frame (~60 FPS)
- Tooltip: Every frame when hovering
- Simulation: 10 FPS (0.1s timesteps)

### Raycasting Performance
- O(n) check per frame where n = visible fuel elements
- Optimized with early exit on closest element
- Minimal performance impact (<1ms per frame)

### UI Rendering
- Bevy's UI system (retained mode)
- Text updates via reactive queries
- No UI rebuild, only text content updates

## Future Enhancements

Potential improvements:
- [ ] Customizable stat panel position
- [ ] Graphical fire danger indicator
- [ ] Historical graphs (temp, fuel over time)
- [ ] Element highlighting on hover
- [ ] Click to pin tooltip
- [ ] Multi-element comparison mode
- [ ] Export statistics to file
- [ ] Custom color schemes
- [ ] Adjustable tooltip offset
