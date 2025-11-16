# Quick Start Guide - Bevy Demo

## Installation & First Run

### 1. Prerequisites
- Rust 1.70+ installed (`rustup` recommended)
- Git
- Graphics drivers for your GPU

### 2. Clone and Build
```bash
git clone https://github.com/mineubob/Australia-Fire-Sim.git
cd Australia-Fire-Sim
cargo build --release -p demo-bevy
```

Build time: ~5-10 minutes (first time, downloads dependencies)

### 3. Run the Demo
```bash
cargo run --release -p demo-bevy
```

The window will open showing:
- Left side: 3D visualization viewport
- Right side: Control panel with UI

## First Steps Tutorial

### Step 1: Add Some Fuel (10 seconds)
1. Look at the **🌳 Add Fuel** section in the right panel
2. Make sure **🌾 Grass** is selected (default)
3. Click several times in the center of the viewport (left side) to place grass elements
4. Or click the **🌾 Add Grass Field (5x5)** button for instant grass

**What you'll see**: Small yellow/dim dots appearing where you clicked

### Step 2: Add a Tree (5 seconds)
1. Click the **🌲 Add Stringybark Tree** button in the **⚡ Actions** section
2. A tree structure will appear at the center

**What you'll see**: A cluster of elements representing trunk and crown

### Step 3: Start the Fire! (5 seconds)
1. Click the **🔥 Ignite Center** button
2. Watch the fire start and spread!

**What you'll see**: 
- Elements turning yellow, then orange, then red as they heat up and burn
- Orange ember particles floating and drifting
- Statistics updating in real-time

### Step 4: Control the Fire (30+ seconds)
Experiment with the controls:

**Increase wind speed**: Move the "Wind Speed" slider to 60+ km/h
- Fire spreads much faster downwind
- More embers generated

**Change wind direction**: Adjust "Wind Direction" slider
- Watch fire spread change direction

**Try Catastrophic conditions**: Click **🔥 Catastrophic** button
- Temperature: 45°C
- Humidity: 10%
- Wind: 60 km/h
- FFDI rating goes to "CATASTROPHIC" (dark red)
- Fire spreads extremely fast

**Pause and observe**: Press **Space** key
- Examine the current state
- Press Space again to resume

**Add more fuel**: Click **🌳 Smooth** and place trees around the fire
- Right-click near them to create spot fires

### Step 5: Camera Controls (2 minutes)
- **Arrow Keys**: Pan around to follow the fire spread
- **+ key**: Zoom in to see details
- **- key**: Zoom out for overview
- **R key**: Reset everything and start over

## Common Scenarios to Try

### 1. Wind-Driven Grass Fire
1. Add large grass field (click the button multiple times or click many times on viewport)
2. Set wind speed to 40-50 km/h
3. Set wind direction to 90° (East)
4. Ignite center
5. Watch fire spread rapidly eastward with ember spotting

### 2. Crown Fire with Oil Vapor Explosion
1. Add 3-4 Stringybark trees (click button multiple times)
2. Set temperature to 40°C
3. Ignite one tree
4. Watch for:
   - Fire climbing vertically (faster than horizontal)
   - Crown ignition at ~650 kW/m intensity
   - Possible oil vapor explosions at 232°C
   - Long-distance ember generation (up to 25km)

### 3. Fire Danger Rating Demo
1. Start with Perth Summer preset (☀️ button)
2. Watch FFDI rating (should be "High" or "Very High")
3. Gradually increase wind speed to 80+ km/h
4. Watch rating change to "Extreme" or "CATASTROPHIC"
5. Observe how fire behavior changes

## Understanding the Visualization

### Fire Colors:
- **Yellow** (🟡): 100-600°C, early stage
- **Orange** (🟠): 600-900°C, active burning
- **Red** (🔴): 900°C+, intense fire

### Embers:
- **Small orange dots** (✨): Active embers
- Larger = hotter
- Float and drift with wind
- Can travel kilometers
- May start spot fires

### FFDI Colors:
- **Blue**: Low (FFDI < 5)
- **Green**: Moderate (5-12)
- **Gold**: High (12-24)
- **Orange**: Very High (24-50)
- **Red-Orange**: Severe (50-75)
- **Red**: Extreme (75-100)
- **Dark Red**: CATASTROPHIC (100+)

## Troubleshooting

**Problem**: Window doesn't open or crashes immediately
- **Solution**: Check GPU drivers are installed
- On Linux: `sudo apt-get install mesa-utils` or appropriate driver package

**Problem**: Low FPS / laggy
- **Solution**: Reduce number of fuel elements, add fewer at a time

**Problem**: Fire doesn't spread
- **Solution**: Make sure fuel elements are close together (< 5m apart)
- Try clicking "Ignite Center" instead of right-clicking

**Problem**: Can't see anything
- **Solution**: Press **-** key to zoom out, might be too zoomed in

## Next Steps

Once comfortable with basics:
- Experiment with different fuel types (smooth bark, shrubland, dead wood)
- Mix fuel types to see different burning rates
- Create complex scenarios with trees and grass
- Try different regional presets (edit code to add more)
- Observe ember spotting distances
- Test moisture effects on ignition delays

## Tips for Best Experience

1. **Start small**: Add 20-50 fuel elements first, then scale up
2. **Use presets**: Quick way to see realistic conditions
3. **Watch embers**: They're key to long-distance fire spread
4. **Experiment with wind**: Dramatically changes fire behavior
5. **Pause to observe**: Use Space key to examine details
6. **Reset often**: R key to try new scenarios quickly

## Performance Notes

- Simulation updates: 10 Hz (every 0.1 seconds)
- Visual rendering: 60 FPS (smooth animation)
- Recommended fuel count: 100-1000 elements
- Maximum tested: 600,000 elements (requires powerful PC)
- Current demo default: ~500 elements

Enjoy exploring realistic fire behavior! 🔥
