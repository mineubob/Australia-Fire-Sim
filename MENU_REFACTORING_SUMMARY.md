# Menu System Refactoring Summary

## Problem Statement
When returning to the menu from the game (pressing ESC), a blank screen would appear instead of showing the menu properly. This was due to improper state management and entity cleanup.

## Root Cause
The original implementation used a custom `MenuState` resource with manual boolean flags to track state. Entity cleanup and camera transitions were handled manually, which led to:
- Race conditions during state transitions
- Entities not being properly cleaned up
- Cameras not switching correctly
- Complex and error-prone state tracking logic

## Solution
Refactored the menu system to use Bevy's built-in `bevy_state` module, following the pattern demonstrated in the official Bevy `game_menu.rs` example.

## Key Changes

### 1. Replaced Custom State with Bevy States
**Before:**
```rust
#[derive(Resource)]
struct MenuState {
    in_menu: bool,
    simulation_initialized: bool,
    scene_setup: bool,
}
```

**After:**
```rust
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    Menu,
    InGame,
}
```

### 2. Added Marker Components for Automatic Cleanup
```rust
#[derive(Component)]
struct OnMenuScreen;

#[derive(Component)]
struct OnGameScreen;
```

All entities are now tagged with these markers based on which state they belong to.

### 3. Implemented State Transition Systems
- `OnEnter(GameState::Menu)` → `setup_menu()`: Spawns Camera2d
- `OnExit(GameState::Menu)` → `cleanup_menu()`: Despawns all menu entities
- `OnEnter(GameState::InGame)` → `setup_game()`: Creates simulation and 3D scene
- `OnExit(GameState::InGame)` → `cleanup_game()`: Despawns all game entities

### 4. Simplified Run Conditions
**Before:**
```rust
.add_systems(Update, update_simulation.run_if(simulation_running))

fn simulation_running(menu_state: Res<MenuState>) -> bool {
    !menu_state.in_menu && menu_state.simulation_initialized && menu_state.scene_setup
}
```

**After:**
```rust
.add_systems(Update, update_simulation.run_if(in_state(GameState::InGame)))
```

### 5. Proper State Transitions
**Before:**
```rust
menu_state.in_menu = false;  // Manual flag setting
```

**After:**
```rust
next_state.set(GameState::InGame);  // Proper state transition
```

## Code Changes Summary

### Files Modified
- `demo-gui/src/main.rs` (major refactoring)

### Lines Changed
- **Removed**: ~122 lines (complex state management logic)
- **Added**: ~94 lines (clean state system)
- **Net change**: -28 lines (simpler code!)

### Key Functions

#### New Functions
1. **`setup_menu()`** - Sets up menu camera when entering menu state
2. **`cleanup_menu()`** - Cleans up menu entities when exiting menu state  
3. **`cleanup_game()`** - Cleans up game entities when exiting game state

#### Refactored Functions
1. **`main()`** - Now uses `.init_state::<GameState>()` and state-based systems
2. **`setup_game()`** - Merged initialization and scene setup into one function
3. **`render_menu_ui()`** - Uses `NextState<GameState>` instead of manual state
4. **`handle_controls()`** - Simplified ESC handling with state transition

#### Removed Functions
1. ~~`init_simulation_system()`~~ - Merged into `setup_game()`
2. ~~`setup_scene()`~~ - Merged into `setup_game()`
3. ~~`setup()`~~ - Merged into `setup_game()`
4. ~~`in_menu()`~~ - Replaced with `in_state(GameState::Menu)`
5. ~~`should_start_simulation()`~~ - No longer needed
6. ~~`should_setup_scene()`~~ - No longer needed
7. ~~`simulation_running()`~~ - Replaced with `in_state(GameState::InGame)`
8. ~~`setup_menu_camera()`~~ - Replaced with `setup_menu()`

### Entity Tagging

All entities now have appropriate markers:

**Menu Entities:**
- Camera2d → `OnMenuScreen`

**Game Entities:**
- Camera3d → `OnGameScreen`
- DirectionalLight → `OnGameScreen`
- TerrainMesh → `OnGameScreen`
- FuelVisual (all) → `OnGameScreen`
- Root UI container → `OnGameScreen`
- Tooltip → `OnGameScreen`
- FPS counter → `OnGameScreen`

## Benefits

### 1. Automatic Cleanup
Entities are automatically despawned when exiting states, preventing:
- Memory leaks
- Orphaned entities
- UI elements persisting between states
- Multiple cameras active simultaneously

### 2. Predictable State Transitions
Using Bevy's state system ensures:
- Atomic state changes
- Proper system execution order
- No race conditions
- Clear state lifecycle

### 3. Better Code Organization
- Clear separation of menu and game logic
- Easier to understand and maintain
- Less boilerplate code
- Follows Bevy best practices

### 4. Robustness
- No manual flag tracking
- No complex run conditions
- Automatic resource cleanup
- Type-safe state transitions

## Testing Recommendations

1. **Start Application** → Verify menu displays
2. **Click START SIMULATION** → Verify game starts
3. **Press ESC** → **Verify menu returns (NO BLANK SCREEN)**
4. **Repeat steps 2-3 multiple times** → Verify stability
5. **Monitor entity count** → Verify no leaks

## References

- Bevy Official Example: [game_menu.rs](https://github.com/bevyengine/bevy/blob/latest/examples/games/game_menu.rs)
- Bevy States Documentation: [bevy_state](https://docs.rs/bevy/latest/bevy/state/index.html)

## Migration Guide

If updating existing code that uses the old menu system:

1. Replace `MenuState` resource with `GameState` enum
2. Add marker components to all entities
3. Replace manual state transitions with `NextState<GameState>`
4. Update run conditions from custom functions to `in_state()`
5. Implement `OnExit` cleanup systems for each state
6. Test all state transitions thoroughly

## Conclusion

This refactoring fixes the blank screen bug by implementing proper state management using Bevy's built-in state system. The code is now simpler, more maintainable, and follows Bevy best practices. The automatic cleanup ensures entities are properly managed across state transitions, preventing the blank screen issue.
