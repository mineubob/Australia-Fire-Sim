# TODO: Main Menu and FPS Display Implementation

## User Requirements
1. **Main menu** where configuration can be set before starting simulation
2. **Customization of everything** - all parameters should be editable
3. **FPS display** showing frame rate during simulation

## Implementation Plan

### 1. Main Menu System
**Add Resource:**
```rust
#[derive(Resource)]
struct MenuState {
    show_menu: bool,
    simulation_ready: bool,
}
```

**Menu Components:**
- Title screen with "Australia Fire Simulation" header
- Scrollable configuration panel with all DemoConfig fields
- Each config item as a row with:
  - Label (e.g., "Temperature:")
  - Value display/input
  - Increment/decrement buttons for numeric values
  - Dropdown selector for enums (TerrainType, FuelType)
- START button at bottom
- Visual styling with semi-transparent backgrounds

**Config Parameters to Include:**
- Map Width/Height (numeric sliders/buttons)
- Terrain Type (dropdown: Flat/Hill/Valley)
- Grid Width/Height (numeric)
- Fuel Mass (numeric slider)
- Fuel Type (dropdown: 5 types)
- Initial Ignitions (numeric)
- Spacing (numeric slider)
- Temperature (numeric slider)
- Humidity (numeric slider 0-1)
- Wind Speed (numeric slider)
- Wind Direction (numeric slider 0-360)
- Drought Factor (numeric slider)

### 2. FPS Display
**Add Component:**
```rust
#[derive(Component)]
struct FpsText;

#[derive(Resource)]
struct FpsCounter {
    frame_count: u32,
    timer: f32,
    fps: f32,
}
```

**System:**
```rust
fn update_fps(
    time: Res<Time>,
    mut fps_counter: ResMut<FpsCounter>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    fps_counter.frame_count += 1;
    fps_counter.timer += time.delta_seconds();
    
    if fps_counter.timer >= 1.0 {
        fps_counter.fps = fps_counter.frame_count as f32 / fps_counter.timer;
        fps_counter.frame_count = 0;
        fps_counter.timer = 0.0;
    }
    
    for mut text in query.iter_mut() {
        text.sections[0].value = format!("FPS: {:.0}", fps_counter.fps);
    }
}
```

**Display Location:**
- Top-right corner
- Small text (14-16px)
- Semi-transparent background
- Updates every second

### 3. Menu Interaction System
```rust
fn handle_menu_buttons(
    mut interaction_query: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    mut config: ResMut<DemoConfig>,
    mut menu_state: ResMut<MenuState>,
) {
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            match button {
                MenuButton::Start => {
                    menu_state.show_menu = false;
                    menu_state.simulation_ready = true;
                }
                MenuButton::Increment(field) => {
                    // Increment the config field
                }
                MenuButton::Decrement(field) => {
                    // Decrement the config field
                }
                MenuButton::Select(field, value) => {
                    // Set the config field to value
                }
            }
        }
    }
}
```

### 4. State Management
**Startup:**
1. Show menu (MenuState.show_menu = true)
2. Don't initialize simulation yet

**Menu Active:**
1. Display config options
2. Handle button interactions
3. Update config values
4. Show preview of current settings

**On START:**
1. Hide menu (despawn entities)
2. Initialize simulation with current config
3. Setup 3D scene
4. Show FPS counter
5. Enable simulation systems

### 5. Visual Design
**Menu:**
```
┌─────────────────────────────────────┐
│   Australia Fire Simulation         │
│                                     │
│   ┌─────────────────────────────┐   │
│   │ TERRAIN SETTINGS           │   │
│   │  Map Width:    [←] 200 [→] │   │
│   │  Map Height:   [←] 200 [→] │   │
│   │  Terrain:      [Hill ▼]    │   │
│   │                            │   │
│   │ FIRE SETTINGS              │   │
│   │  Grid Width:   [←] 10  [→] │   │
│   │  ...                       │   │
│   │                            │   │
│   │ WEATHER SETTINGS           │   │
│   │  Temperature:  [←] 35  [→] │   │
│   │  ...                       │   │
│   └─────────────────────────────┘   │
│                                     │
│        [  START SIMULATION  ]       │
└─────────────────────────────────────┘
```

**FPS Display:**
```
┌──────────┐
│ FPS: 60  │
└──────────┘
(top-right corner, small, semi-transparent)
```

## Files to Modify
1. `demo-gui/src/main.rs` - Add menu systems and FPS counter
2. `demo-gui/README.md` - Document menu usage
3. `demo-gui/UI_FEATURES.md` - Add menu and FPS sections

## Estimated Lines of Code
- Menu system: ~300-400 lines
- FPS counter: ~50 lines
- Button components: ~100 lines
- Total: ~450-550 new lines

## Testing Checklist
- [ ] Menu displays on startup
- [ ] All config values are editable
- [ ] Increment/decrement buttons work
- [ ] Dropdown selectors work
- [ ] START button transitions to simulation
- [ ] FPS counter displays correctly
- [ ] FPS updates every second
- [ ] Config values are applied to simulation
- [ ] Can restart and see menu again (R key)

## Notes
- Keep menu simple and functional
- Use existing UI patterns from stats panel
- Ensure all values have sensible min/max limits
- Add visual feedback for button interactions
- Make FPS counter non-intrusive
