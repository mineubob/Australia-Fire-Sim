# Testing the Menu System Fix

## Overview
This document describes how to test the new `bevy_state`-based menu system that fixes the blank screen issue when returning to the menu from the game.

## What Was Changed

### Before
- Used a custom `MenuState` resource with manual boolean flags (`in_menu`, `simulation_initialized`, `scene_setup`)
- Manually spawned and despawned cameras between states
- Manually cleaned up entities when returning to menu
- Complex run conditions checking multiple flags

### After
- Uses Bevy's built-in `GameState` enum with proper `States` derive
- Automatic state transitions using `NextState<GameState>`
- Marker components (`OnMenuScreen`, `OnGameScreen`) for automatic cleanup
- Clean `OnEnter` and `OnExit` systems for each state
- Simple `in_state(GameState::...)` run conditions

## Key Changes

### 1. GameState Enum
```rust
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    Menu,
    InGame,
}
```

### 2. State Transition Flow
1. **App starts** → `GameState::Menu` (default)
2. **User clicks "START SIMULATION"** → Transitions to `GameState::InGame`
3. **User presses ESC** → Transitions back to `GameState::Menu`

### 3. Automatic Cleanup
- **OnExit(GameState::Menu)**: Despawns all entities with `OnMenuScreen` marker
- **OnExit(GameState::InGame)**: Despawns all entities with `OnGameScreen` marker, clears simulation state

## How to Test

### 1. Start the Application
```bash
cargo run --package demo-gui
```

### 2. Test Menu Display
- Verify the main menu is displayed with:
  - Title: "Australia Fire Simulation"
  - Configuration options (terrain, fire, weather settings)
  - "START SIMULATION" button

### 3. Test Transition to Game
1. Configure simulation parameters
2. Click "START SIMULATION" button
3. Verify:
   - Menu disappears
   - 3D scene appears with camera, terrain, fuel elements
   - UI shows simulation statistics
   - Controls are responsive

### 4. Test Return to Menu (The Fix!)
1. While in the game, press **ESC** key
2. Verify:
   - 3D scene disappears
   - Menu reappears with **no blank screen**
   - Menu is fully functional
   - Previous configuration is preserved

### 5. Test Multiple Transitions
1. Start simulation again (should work)
2. Press ESC to return to menu (should work)
3. Repeat 3-4 times to ensure stability
4. Verify no memory leaks or leftover entities

## Expected Behavior

### Menu State
- Camera2d is active
- Only menu UI visible
- Configuration can be changed
- START SIMULATION button works

### InGame State  
- Camera3d is active
- 3D scene with terrain and fuel visible
- Simulation UI overlay visible
- Controls work (SPACE, I, W, R, arrows, ESC)

## Technical Details

### Entity Lifecycle

**Menu Entities:**
- Camera2d with `OnMenuScreen` marker
- Cleaned up on `OnExit(GameState::Menu)`

**Game Entities:**
- Camera3d with `OnGameScreen` marker
- DirectionalLight with `OnGameScreen` marker
- Terrain mesh with `OnGameScreen` marker
- Fuel visuals with `OnGameScreen` marker
- All UI elements with `OnGameScreen` marker
- All cleaned up on `OnExit(GameState::InGame)`

### Resources
- `DemoConfig`: Persists across states (configuration)
- `SimulationState`: Created on game start, removed on game exit
- `AmbientLight`: Created on game start, removed on game exit

## Troubleshooting

### If blank screen appears when returning to menu:
1. Check that `Camera2d` is spawned with `OnMenuScreen` marker in `setup_menu()`
2. Verify `cleanup_game()` is properly despawning all entities
3. Ensure `OnExit(GameState::InGame)` system is registered

### If game doesn't start:
1. Check that `setup_game()` is called on `OnEnter(GameState::InGame)`
2. Verify `Camera3d` is spawned with `OnGameScreen` marker
3. Check `SimulationState` is properly initialized

### If entities persist between states:
1. Verify all game entities have `OnGameScreen` marker
2. Check that `cleanup_game()` is iterating all entities
3. Ensure marker components are properly added during spawn

## Code References

- Main state definition: Line 180-186 in `demo-gui/src/main.rs`
- State setup: Line 204-227 in `demo-gui/src/main.rs`
- Menu setup: Line 238-242
- Menu cleanup: Line 244-249
- Game setup: Line 582-665
- Game cleanup: Line 251-267
- State transition (START): Line 455
- State transition (ESC): Line 1460-1463
