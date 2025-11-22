# State Transition Diagram

This document provides a visual representation of the state transitions in the refactored menu system.

## State Machine

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Application Start                            │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
                    ┌────────────────┐
                    │   GameState    │
                    │     ::Menu     │ ◄──────────────┐
                    │   (DEFAULT)    │                │
                    └────────────────┘                │
                             │                        │
                             │                        │
          ┌──────────────────┴──────────────────┐    │
          │ OnEnter(GameState::Menu)            │    │
          │   ├─ setup_menu()                   │    │
          │   └─ Spawn Camera2d + OnMenuScreen  │    │
          └─────────────────────────────────────┘    │
                             │                        │
                             │                        │
          ┌──────────────────▼──────────────────┐    │
          │ Systems Running:                    │    │
          │   └─ render_menu_ui()               │    │
          │      └─ Show configuration UI       │    │
          └─────────────────────────────────────┘    │
                             │                        │
                             │ User clicks            │
                             │ "START SIMULATION"     │
                             │                        │
          ┌──────────────────▼──────────────────┐    │
          │ OnExit(GameState::Menu)             │    │
          │   ├─ cleanup_menu()                 │    │
          │   └─ Despawn Camera2d               │    │
          └─────────────────────────────────────┘    │
                             │                        │
                             ▼                        │
                    ┌────────────────┐                │
                    │   GameState    │                │
                    │    ::InGame    │                │
                    └────────────────┘                │
                             │                        │
                             │                        │
          ┌──────────────────┴──────────────────┐    │
          │ OnEnter(GameState::InGame)          │    │
          │   ├─ setup_game()                   │    │
          │   ├─ Create SimulationState         │    │
          │   ├─ Spawn Camera3d + OnGameScreen  │    │
          │   ├─ Spawn Lights + OnGameScreen    │    │
          │   ├─ Spawn Terrain + OnGameScreen   │    │
          │   ├─ Spawn Fuel + OnGameScreen      │    │
          │   └─ Spawn UI + OnGameScreen        │    │
          └─────────────────────────────────────┘    │
                             │                        │
                             │                        │
          ┌──────────────────▼──────────────────┐    │
          │ Systems Running:                    │    │
          │   ├─ update_simulation()            │    │
          │   ├─ update_fuel_visuals()          │    │
          │   ├─ update_camera_controls()       │    │
          │   ├─ update_ui()                    │    │
          │   ├─ handle_controls()              │    │
          │   └─ update_tooltip()               │    │
          └─────────────────────────────────────┘    │
                             │                        │
                             │ User presses ESC       │
                             │                        │
          ┌──────────────────▼──────────────────┐    │
          │ OnExit(GameState::InGame)           │    │
          │   ├─ cleanup_game()                 │    │
          │   ├─ Despawn all OnGameScreen       │    │
          │   ├─ Clear fuel_entity_map          │    │
          │   └─ Remove AmbientLight            │    │
          └──────────────────┬──────────────────┘    │
                             │                        │
                             └────────────────────────┘
```

## Entity Lifecycle

### Menu State Entities

```
┌─────────────────────────────────────┐
│         OnMenuScreen Marker         │
├─────────────────────────────────────┤
│                                     │
│  ┌──────────────────────────────┐  │
│  │       Camera2d               │  │
│  │  (Required for egui to work) │  │
│  └──────────────────────────────┘  │
│                                     │
└─────────────────────────────────────┘
      │                         │
      │ OnEnter(Menu)           │ OnExit(Menu)
      │ spawn()                 │ despawn()
      ▼                         ▼
```

### InGame State Entities

```
┌─────────────────────────────────────────────────────────────┐
│                OnGameScreen Marker                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────┐  ┌──────────────────┐               │
│  │    Camera3d      │  │ DirectionalLight │               │
│  └──────────────────┘  └──────────────────┘               │
│                                                             │
│  ┌──────────────────┐  ┌──────────────────┐               │
│  │  TerrainMesh     │  │  FuelVisual (N)  │               │
│  └──────────────────┘  └──────────────────┘               │
│                                                             │
│  ┌──────────────────┐  ┌──────────────────┐               │
│  │   UI Root Node   │  │   TooltipText    │               │
│  │   (with children)│  └──────────────────┘               │
│  └──────────────────┘                                      │
│  │                    ┌──────────────────┐                │
│  ├─ Title            │   FPS Counter    │                │
│  ├─ ControlsText     └──────────────────┘                │
│  └─ StatsPanel                                             │
│     ├─ Stats Heading                                       │
│     └─ StatsText                                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
      │                                           │
      │ OnEnter(InGame)                           │ OnExit(InGame)
      │ spawn_all()                               │ despawn_all()
      ▼                                           ▼
```

## State Transition Timeline

```
Time  →
─────────────────────────────────────────────────────────────────────

Frame 0:   App Start
           ├─ Initialize GameState::Menu (default)
           └─ Run setup_menu()
              └─ Spawn Camera2d with OnMenuScreen

Frame N:   Menu UI Renders
           └─ User configures simulation
              └─ User clicks "START SIMULATION"

Frame N+1: State Transition (Menu → InGame)
           ├─ Run cleanup_menu()
           │  └─ Despawn Camera2d
           ├─ Change state to GameState::InGame
           └─ Run setup_game()
              ├─ Create SimulationState
              ├─ Spawn Camera3d with OnGameScreen
              ├─ Spawn DirectionalLight with OnGameScreen
              ├─ Spawn TerrainMesh with OnGameScreen
              ├─ Spawn N FuelVisuals with OnGameScreen
              └─ Spawn UI elements with OnGameScreen

Frame N+2: Game Systems Run
           ├─ update_simulation()
           ├─ update_fuel_visuals()
           ├─ update_camera_controls()
           ├─ update_ui()
           ├─ handle_controls()
           └─ update_tooltip()

Frame M:   User Presses ESC

Frame M+1: State Transition (InGame → Menu)
           ├─ Run cleanup_game()
           │  ├─ Despawn Camera3d
           │  ├─ Despawn DirectionalLight
           │  ├─ Despawn TerrainMesh
           │  ├─ Despawn all FuelVisuals
           │  ├─ Despawn all UI elements
           │  ├─ Clear fuel_entity_map
           │  └─ Remove AmbientLight resource
           ├─ Change state to GameState::Menu
           └─ Run setup_menu()
              └─ Spawn Camera2d with OnMenuScreen

Frame M+2: Menu UI Renders Again (NO BLANK SCREEN!)
           └─ User can configure and restart simulation
```

## Comparison: Old vs New

### Old System (BROKEN)

```
Manual State Flags
├─ in_menu: bool
├─ simulation_initialized: bool
└─ scene_setup: bool
    │
    ├─ Complex run conditions
    │  ├─ in_menu()
    │  ├─ should_start_simulation()
    │  ├─ should_setup_scene()
    │  └─ simulation_running()
    │
    └─ Manual cleanup (ESC handler)
       ├─ Query entities manually
       ├─ Despawn individually
       ├─ Reset flags manually
       └─ Spawn Camera2d manually
          │
          └─ ❌ Race conditions!
          └─ ❌ Incomplete cleanup!
          └─ ❌ Blank screen bug!
```

### New System (FIXED)

```
Bevy States
├─ GameState::Menu (default)
└─ GameState::InGame
    │
    ├─ Simple run conditions
    │  ├─ in_state(GameState::Menu)
    │  └─ in_state(GameState::InGame)
    │
    └─ Automatic cleanup (OnExit)
       ├─ Marker components
       │  ├─ OnMenuScreen
       │  └─ OnGameScreen
       │
       └─ Cleanup systems
          ├─ cleanup_menu()
          └─ cleanup_game()
             │
             └─ ✅ No race conditions!
             └─ ✅ Complete cleanup!
             └─ ✅ No blank screen!
```

## Key Benefits

1. **Atomic State Changes**: State transitions are atomic and predictable
2. **Automatic Cleanup**: Entities are automatically cleaned up on state exit
3. **Clear Separation**: Menu and game logic are completely separate
4. **Type Safety**: Compile-time verification of state transitions
5. **Maintainability**: Easy to add new states or modify existing ones

## References

- Bevy States Documentation: https://docs.rs/bevy/latest/bevy/state/
- Bevy Game Menu Example: https://github.com/bevyengine/bevy/blob/main/examples/games/game_menu.rs
