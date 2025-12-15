# Terminal UI Demo

## Overview

The Bushfire Simulation now features a rich terminal UI powered by ratatui, providing an enhanced interactive experience with multiple panels and color-coded visualizations.

## UI Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│ Fire Simulation | Step: 0 | Time: 0.0s | Elements: 2564 | Burning: 0│
└─────────────────────────────────────────────────────────────────────┘
┌─ Messages ───────────────────────────────────┐┌─ 🔥 Burning (0) ────┐
│ Welcome to Bushfire Simulation!              ││                     │
│ Created simulation with 2564 elements...     ││ (List of burning    │
│ Type 'help' for available commands.          ││  elements with      │
│                                              ││  temperatures)      │
│ Command outputs appear here...               ││                     │
│                                              ││ Color-coded:        │
│                                              ││ - Red: > 800°C      │
│                                              ││ - Yellow: > 400°C   │
│                                              ││ - White: < 400°C    │
└──────────────────────────────────────────────┘└─────────────────────┘
┌─ Command Input (F1 for help) ───────────────────────────────────────┐
│ fire> _                                                             │
└─────────────────────────────────────────────────────────────────────┘
```

## Features

### Header Panel
- Shows current simulation state: step count, elapsed time, total elements, burning elements, ember count
- Always visible at the top

### Messages Panel (Left)
- Displays all command outputs and simulation events
- Color-coded messages:
  - **Green**: Successful operations (ignition, steps)
  - **Red**: Errors and warnings
  - **Cyan**: Section headers
  - **White**: General information
- Auto-scrolls to show most recent messages
- Retains last 100 messages

### Burning Elements Panel (Right)
- Live list of currently burning elements
- Shows ID, temperature, and position
- Temperature color coding:
  - **Red**: > 800°C (very hot)
  - **Yellow**: > 400°C (hot)
  - **White**: < 400°C (warm)
- Shows count in title: "🔥 Burning (N)"

### Command Input (Bottom)
- Interactive command prompt
- Full history navigation (Up/Down arrows)
- Supports all existing commands
- F1 for quick help access

## View Modes

The UI supports multiple view modes that can be switched between:

### 1. Dashboard (default)
- Split view with messages and burning elements
- Best for active monitoring during simulation

### 2. Status View (command: `status` or `st`)
- Detailed simulation statistics
- Element temperature ranges (min/max/avg)
- Press ESC to return to dashboard

### 3. Weather View (command: `weather` or `w`)
- Complete weather conditions
- FFDI (Fire Danger Index)
- Wind speed, direction, temperature, humidity
- Drought factor and spread multiplier
- Press ESC to return to dashboard

### 4. Help View (command: `help` or `?`, or press F1)
- Complete list of available commands
- Organized by category:
  - Simulation Control
  - View Controls
  - Element Commands
  - Position Commands
  - Visualization
- Keyboard shortcuts
- Press ESC to return to dashboard

## Keyboard Controls

- **Enter**: Execute command
- **Up/Down Arrows**: Navigate command history
- **Backspace**: Delete character
- **ESC**: Return to dashboard view
- **F1**: Show help
- **Ctrl+T**: Toggle burning list sort mode (temperature/time since ignition)
- **Ctrl+C**: 
  - During stepping: Stop the stepping process (simulation remains intact)
  - Otherwise: Quit application

### Commands During Stepping

While the simulation is stepping, you can still use these commands:
- View commands: `status`, `weather`, `dashboard`, `help`
- Query commands: `element`, `burning`, `embers`, `nearby`, `heatmap`
- Navigation: Press **Ctrl+C** to interrupt stepping at any time

Simulation-modifying commands (like `ignite`, `heat`, `step`, `reset`) are blocked during stepping to prevent conflicts.

## All Commands Preserved

All existing commands from the original REPL interface are fully supported:

### Simulation Control
- `step [n]`, `s [n]` - Advance simulation
- `reset [w] [h]`, `r` - Reset with optional dimensions
- `preset <name>`, `p` - Change weather preset

### View Controls
- `dashboard`, `d` - Dashboard view
- `status`, `st` - Status view
- `weather`, `w` - Weather view
- `help`, `?` - Help view

### Element Commands
- `element <id>`, `e` - Show element details
- `burning`, `b` - List burning elements
- `embers`, `em` - List active embers
- `nearby <id>`, `n` - Show nearby elements
- `ignite <id>`, `i` - Ignite element
- `heat <id> <temp>`, `h` - Heat element

### Position Commands
- `ignite_position <x> <y> [radius] [amount] [filters]`, `ip`
- `heat_position <x> <y> <temp> [radius] [amount] [filters]`, `hp`

### Filters (for position commands)

The position commands support optional filters to target specific elements:

- `fuel=<name>` - Filter by fuel type (e.g., `fuel=eucalyptus`, `fuel=savanna`)
- `part=<type>` - Filter by fuel part type (e.g., `part=crown`, `part=root`, `part=groundlitter`)
- `minz=<height>` - Minimum height in meters (e.g., `minz=0`)
- `maxz=<height>` - Maximum height in meters (e.g., `maxz=20`)

**Examples:**
```bash
# Ignite 5 eucalyptus elements between 0-10m height in a 50m radius
ip 100 100 50 5 fuel=eucalyptus minz=0 maxz=10

# Heat all crown parts to 600°C in a 30m radius
hp 100 100 600 30 -1 part=crown

# Heat the closest 3 elements at ground level
hp 100 100 500 20 3 maxz=1.0
```

### Visualization
- `heatmap [size]`, `hm` - Show temperature heatmap

### Control
- `quit`, `q` - Exit

## Non-Interactive Usage

The demo can be used non-interactively via heredoc (for testing):

```bash
./target/release/demo-interactive --headless <<'HEREDOC'
50
50
i 100
s 10
q
HEREDOC
```