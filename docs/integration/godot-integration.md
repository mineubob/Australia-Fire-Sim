# Godot 4.x Integration Guide

## Overview

This guide explains how to integrate the **Australia Fire Simulation** Rust library into Godot 4.x for realistic wildfire visualization and gameplay mechanics.

The fire simulation core is written in Rust for performance and scientific accuracy, with a C-compatible FFI (Foreign Function Interface) layer that allows seamless integration with Godot's GDExtension system.

## Architecture

```
Godot 4.x (GDScript/C++)
        ↓
    GDExtension Layer
        ↓
    FFI Layer (C API)
        ↓
Fire Sim Core (Rust)
```

## Step 1: Build the Rust Library

### Prerequisites
- Rust toolchain (1.70+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- C compiler (GCC/Clang on Linux, MSVC on Windows, Clang on macOS)
- Godot 4.x source or SDK

### Build Steps

```bash
# Clone the repository
git clone https://github.com/mineubob/Australia-Fire-Sim.git
cd Australia-Fire-Sim

# Build the FFI library in release mode
cargo build --release -p fire-sim-ffi

# Library outputs:
# - Windows: target/release/fire_sim_ffi.dll
# - Linux: target/release/libfire_sim_ffi.so
# - macOS: target/release/libfire_sim_ffi.dylib
```

### Generate C Header

The C header is **automatically generated** during the build process via `build.rs`:

```bash
# Build the FFI library - this automatically generates FireSimFFI.h
cargo build --release -p fire-sim-ffi

# The header is now available at: FireSimFFI.h (repo root)
# Copy it to your Godot project's thirdparty directory
cp FireSimFFI.h YourProject/gdextension/thirdparty/fire_sim_ffi/include/
```

## Step 2: Create GDExtension Project Structure

Create the following directory structure in your Godot project:

```
YourProject/
├── gdextension/
│   ├── fire_sim.gdextension
│   ├── SConstruct
│   ├── binding_generator.py
│   ├── src/
│   │   ├── fire_sim_wrapper.cpp
│   │   ├── fire_sim_wrapper.h
│   │   ├── register_types.cpp
│   │   └── register_types.h
│   └── thirdparty/
│       └── fire_sim_ffi/
│           ├── include/
│           │   └── FireSimFFI.h
│           └── lib/
│               ├── linux/
│               │   └── libfire_sim_ffi.so
│               ├── windows/
│               │   └── fire_sim_ffi.dll
│               └── macos/
│                   └── libfire_sim_ffi.dylib
└── scenes/
    └── FireSimulationManager.gd
```

## Step 3: GDExtension C++ Wrapper

Create `fire_sim_wrapper.h`:

```cpp
// fire_sim_wrapper.h
#ifndef FIRE_SIM_WRAPPER_H
#define FIRE_SIM_WRAPPER_H

#include <godot_cpp/classes/node.hpp>
#include <godot_cpp/classes/node3d.hpp>
#include <godot_cpp/variant/typed_array.hpp>
#include <godot_cpp/variant/dictionary.hpp>
#include <godot_cpp/variant/vector3.hpp>

#include "FireSimFFI.h"

using namespace godot;

// C++ wrapper for fire simulation
class FireSimulation : public Node3D {
    GDCLASS(FireSimulation, Node3D)

public:
    FireSimulation();
    ~FireSimulation();

    // Lifecycle
    void _ready() override;
    void _process(double delta) override;
    void _exit_tree() override;

    // Configuration
    void set_map_width(float width);
    float get_map_width() const;
    
    void set_map_height(float height);
    float get_map_height() const;
    
    void set_grid_cell_size(float size);
    float get_grid_cell_size() const;
    
    void set_terrain_type(uint8_t type);
    uint8_t get_terrain_type() const;

    // Initialization
    int32_t initialize_simulation();
    void cleanup_simulation();

    // Weather
    int32_t set_weather(float temperature, float humidity, float wind_speed, float wind_direction, float drought_factor);
    Dictionary get_weather() const;

    // Fuel Management
    int32_t add_fuel_element(Vector3 position, uint8_t fuel_type, uint8_t part_type, float mass, int32_t parent_id = -1);
    
    int32_t ignite_element(uint32_t element_id, float initial_temp = 600.0f);
    
    int32_t apply_suppression_to_elements(Vector3 center, float radius, float mass_per_element, uint8_t agent_type);
    
    int32_t apply_water_direct(Vector3 position, float mass);

    // Query Functions
    TypedArray<Dictionary> get_burning_elements();
    
    Dictionary get_snapshot();
    
    int32_t get_burning_count() const;
    
    int32_t get_ember_count() const;
    
    uint8_t get_haines_index() const;

    // Terrain Queries
    float get_elevation(Vector3 position);
    
    float get_slope(Vector3 position);
    
    float get_aspect(Vector3 position);
    
    float get_slope_spread_multiplier(Vector3 from_pos, Vector3 to_pos);
    
    Dictionary get_terrain_info();

    // Multiplayer
    bool submit_player_action(uint8_t action_type, uint32_t player_id, Vector3 position, float param1, uint32_t param2);
    
    uint32_t get_frame_number() const;

protected:
    static void _bind_methods();

private:
    uintptr_t sim_id = 0;
    
    float map_width = 1000.0f;
    float map_height = 1000.0f;
    float grid_cell_size = 2.0f;
    uint8_t terrain_type = 0;
    
    float temperature = 30.0f;
    float humidity = 30.0f;
    float wind_speed = 30.0f;
    float wind_direction = 0.0f;
    float drought_factor = 5.0f;
};

#endif // FIRE_SIM_WRAPPER_H
```

Create `fire_sim_wrapper.cpp`:

```cpp
// fire_sim_wrapper.cpp
#include "fire_sim_wrapper.h"
#include <godot_cpp/core/class_db.hpp>

FireSimulation::FireSimulation() {
    // Constructor
}

FireSimulation::~FireSimulation() {
    cleanup_simulation();
}

void FireSimulation::_ready() {
    if (initialize_simulation() != FIRE_SIM_SUCCESS) {
        get_tree()->quit();
    }
}

void FireSimulation::_process(double delta) {
    if (sim_id != 0) {
        fire_sim_update(sim_id, (float)delta);
    }
}

void FireSimulation::_exit_tree() {
    cleanup_simulation();
}

void FireSimulation::_bind_methods() {
    // Properties
    ClassDB::bind_method(D_METHOD("set_map_width", "width"), &FireSimulation::set_map_width);
    ClassDB::bind_method(D_METHOD("get_map_width"), &FireSimulation::get_map_width);
    ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "map_width", PROPERTY_HINT_RANGE, "100,10000"), "set_map_width", "get_map_width");

    ClassDB::bind_method(D_METHOD("set_map_height", "height"), &FireSimulation::set_map_height);
    ClassDB::bind_method(D_METHOD("get_map_height"), &FireSimulation::get_map_height);
    ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "map_height", PROPERTY_HINT_RANGE, "100,10000"), "set_map_height", "get_map_height");

    ClassDB::bind_method(D_METHOD("set_grid_cell_size", "size"), &FireSimulation::set_grid_cell_size);
    ClassDB::bind_method(D_METHOD("get_grid_cell_size"), &FireSimulation::get_grid_cell_size);
    ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "grid_cell_size", PROPERTY_HINT_RANGE, "1,10"), "set_grid_cell_size", "get_grid_cell_size");

    ClassDB::bind_method(D_METHOD("set_terrain_type", "type"), &FireSimulation::set_terrain_type);
    ClassDB::bind_method(D_METHOD("get_terrain_type"), &FireSimulation::get_terrain_type);
    ADD_PROPERTY(PropertyInfo(Variant::INT, "terrain_type", PROPERTY_HINT_ENUM, "Flat,Single Hill,Valley"), "set_terrain_type", "get_terrain_type");

    // Core methods
    ClassDB::bind_method(D_METHOD("initialize_simulation"), &FireSimulation::initialize_simulation);
    ClassDB::bind_method(D_METHOD("cleanup_simulation"), &FireSimulation::cleanup_simulation);

    // Weather
    ClassDB::bind_method(D_METHOD("set_weather", "temperature", "humidity", "wind_speed", "wind_direction", "drought_factor"), &FireSimulation::set_weather);
    ClassDB::bind_method(D_METHOD("get_weather"), &FireSimulation::get_weather);

    // Fuel Management
    ClassDB::bind_method(D_METHOD("add_fuel_element", "position", "fuel_type", "part_type", "mass", "parent_id"), &FireSimulation::add_fuel_element, DEFVAL(-1));
    ClassDB::bind_method(D_METHOD("ignite_element", "element_id", "initial_temp"), &FireSimulation::ignite_element, DEFVAL(600.0f));
    ClassDB::bind_method(D_METHOD("apply_suppression_to_elements", "center", "radius", "mass_per_element", "agent_type"), &FireSimulation::apply_suppression_to_elements);
    ClassDB::bind_method(D_METHOD("apply_water_direct", "position", "mass"), &FireSimulation::apply_water_direct);

    // Query
    ClassDB::bind_method(D_METHOD("get_burning_elements"), &FireSimulation::get_burning_elements);
    ClassDB::bind_method(D_METHOD("get_snapshot"), &FireSimulation::get_snapshot);
    ClassDB::bind_method(D_METHOD("get_burning_count"), &FireSimulation::get_burning_count);
    ClassDB::bind_method(D_METHOD("get_ember_count"), &FireSimulation::get_ember_count);
    ClassDB::bind_method(D_METHOD("get_haines_index"), &FireSimulation::get_haines_index);

    // Terrain
    ClassDB::bind_method(D_METHOD("get_elevation", "position"), &FireSimulation::get_elevation);
    ClassDB::bind_method(D_METHOD("get_slope", "position"), &FireSimulation::get_slope);
    ClassDB::bind_method(D_METHOD("get_aspect", "position"), &FireSimulation::get_aspect);
    ClassDB::bind_method(D_METHOD("get_slope_spread_multiplier", "from_pos", "to_pos"), &FireSimulation::get_slope_spread_multiplier);
    ClassDB::bind_method(D_METHOD("get_terrain_info"), &FireSimulation::get_terrain_info);

    // Multiplayer
    ClassDB::bind_method(D_METHOD("submit_player_action", "action_type", "player_id", "position", "param1", "param2"), &FireSimulation::submit_player_action);
    ClassDB::bind_method(D_METHOD("get_frame_number"), &FireSimulation::get_frame_number);
}

// Property setters/getters
void FireSimulation::set_map_width(float width) { map_width = width; }
float FireSimulation::get_map_width() const { return map_width; }

void FireSimulation::set_map_height(float height) { map_height = height; }
float FireSimulation::get_map_height() const { return map_height; }

void FireSimulation::set_grid_cell_size(float size) { grid_cell_size = size; }
float FireSimulation::get_grid_cell_size() const { return grid_cell_size; }

void FireSimulation::set_terrain_type(uint8_t type) { terrain_type = type; }
uint8_t FireSimulation::get_terrain_type() const { return terrain_type; }

// Initialization
int32_t FireSimulation::initialize_simulation() {
    if (sim_id != 0) {
        fire_sim_destroy(sim_id);
    }

    uintptr_t new_sim_id = 0;
    int32_t result = fire_sim_create(map_width, map_height, grid_cell_size, terrain_type, &new_sim_id);

    if (result != FIRE_SIM_SUCCESS) {
        UtilityFunctions::print_error("Failed to create fire simulation (error: ", result, ")");
        return result;
    }

    sim_id = new_sim_id;

    // Set initial weather
    fire_sim_set_weather_from_live(sim_id, temperature, humidity, wind_speed, wind_direction, 1013.25f, drought_factor, 0.0f);

    UtilityFunctions::print("Fire simulation initialized (ID: ", sim_id, ")");
    return FIRE_SIM_SUCCESS;
}

void FireSimulation::cleanup_simulation() {
    if (sim_id != 0) {
        fire_sim_destroy(sim_id);
        sim_id = 0;
    }
}

// Weather
int32_t FireSimulation::set_weather(float temp, float hum, float wind_spd, float wind_dir, float drought) {
    temperature = temp;
    humidity = hum;
    wind_speed = wind_spd;
    wind_direction = wind_dir;
    drought_factor = drought;

    if (sim_id == 0) return FIRE_SIM_INVALID_ID;

    return fire_sim_set_weather_from_live(sim_id, temperature, humidity, wind_speed, wind_direction, 1013.25f, drought_factor, 0.0f);
}

Dictionary FireSimulation::get_weather() const {
    Dictionary weather;
    weather["temperature"] = temperature;
    weather["humidity"] = humidity;
    weather["wind_speed"] = wind_speed;
    weather["wind_direction"] = wind_direction;
    weather["drought_factor"] = drought_factor;
    return weather;
}

// Fuel Management
int32_t FireSimulation::add_fuel_element(Vector3 position, uint8_t fuel_type, uint8_t part_type, float mass, int32_t parent_id) {
    if (sim_id == 0) return FIRE_SIM_INVALID_ID;

    uint32_t element_id = 0;
    int32_t result = fire_sim_add_fuel(sim_id,
        position.x, position.y, position.z,
        fuel_type, part_type, mass, parent_id, &element_id);

    return (result == FIRE_SIM_SUCCESS) ? (int32_t)element_id : result;
}

int32_t FireSimulation::ignite_element(uint32_t element_id, float initial_temp) {
    if (sim_id == 0) return FIRE_SIM_INVALID_ID;

    return fire_sim_ignite(sim_id, element_id, initial_temp);
}

int32_t FireSimulation::apply_suppression_to_elements(Vector3 center, float radius, float mass_per_element, uint8_t agent_type) {
    if (sim_id == 0) return FIRE_SIM_INVALID_ID;

    uint32_t count = 0;
    int32_t result = fire_sim_apply_suppression_to_elements(sim_id,
        center.x, center.y, center.z,
        radius, mass_per_element, agent_type, &count);

    return (result == FIRE_SIM_SUCCESS) ? (int32_t)count : result;
}

int32_t FireSimulation::apply_water_direct(Vector3 position, float mass) {
    if (sim_id == 0) return FIRE_SIM_INVALID_ID;

    return fire_sim_apply_water_direct(sim_id, position.x, position.y, position.z, mass);
}

// Query Functions
TypedArray<Dictionary> FireSimulation::get_burning_elements() {
    TypedArray<Dictionary> result;

    if (sim_id == 0) return result;

    const uint32_t MaxElements = 1000;
    ElementFireState* states = new ElementFireState[MaxElements];
    uint32_t count = 0;

    int32_t query_result = fire_sim_get_burning_elements(sim_id, states, MaxElements, &count);

    if (query_result != FIRE_SIM_SUCCESS) {
        delete[] states;
        return result;
    }

    for (uint32_t i = 0; i < count; ++i) {
        Dictionary elem;
        elem["id"] = (int)states[i].element_id;
        elem["temperature"] = states[i].temperature;
        elem["fuel_remaining"] = states[i].fuel_remaining;
        elem["moisture_fraction"] = states[i].moisture_fraction;
        elem["is_ignited"] = states[i].is_ignited;
        elem["flame_height"] = states[i].flame_height;
        elem["is_crown_fire"] = states[i].is_crown_fire;
        elem["has_suppression"] = states[i].has_suppression;
        elem["suppression_effectiveness"] = states[i].suppression_effectiveness;

        result.push_back(elem);
    }

    delete[] states;
    return result;
}

Dictionary FireSimulation::get_snapshot() {
    Dictionary snapshot;

    if (sim_id == 0) return snapshot;

    SimulationSnapshot snap;
    int32_t result = fire_sim_get_snapshot(sim_id, &snap);

    if (result == FIRE_SIM_SUCCESS) {
        snapshot["frame_number"] = (int)snap.frame_number;
        snapshot["simulation_time"] = snap.simulation_time;
        snapshot["burning_element_count"] = (int)snap.burning_element_count;
        snapshot["total_fuel_consumed"] = snap.total_fuel_consumed;
        snapshot["active_ember_count"] = (int)snap.active_ember_count;
        snapshot["pyrocumulus_count"] = (int)snap.pyrocumulus_count;
        snapshot["haines_index"] = (int)snap.haines_index;
    }

    return snapshot;
}

int32_t FireSimulation::get_burning_count() const {
    if (sim_id == 0) return 0;

    uint32_t count = 0;
    fire_sim_get_burning_count(sim_id, &count);
    return (int32_t)count;
}

int32_t FireSimulation::get_ember_count() const {
    if (sim_id == 0) return 0;

    uint32_t count = 0;
    fire_sim_get_ember_count(sim_id, &count);
    return (int32_t)count;
}

uint8_t FireSimulation::get_haines_index() const {
    if (sim_id == 0) return 0;
    return fire_sim_get_haines_index(sim_id);
}

// Terrain
float FireSimulation::get_elevation(Vector3 position) {
    if (sim_id == 0) return 0.0f;

    float elevation = 0.0f;
    fire_sim_get_elevation(sim_id, position.x, position.y, &elevation);
    return elevation;
}

float FireSimulation::get_slope(Vector3 position) {
    if (sim_id == 0) return 0.0f;
    return fire_sim_terrain_slope(sim_id, position.x, position.y);
}

float FireSimulation::get_aspect(Vector3 position) {
    if (sim_id == 0) return 0.0f;
    return fire_sim_terrain_aspect(sim_id, position.x, position.y);
}

float FireSimulation::get_slope_spread_multiplier(Vector3 from_pos, Vector3 to_pos) {
    if (sim_id == 0) return 1.0f;
    return fire_sim_terrain_slope_multiplier(sim_id, from_pos.x, from_pos.y, to_pos.x, to_pos.y);
}

Dictionary FireSimulation::get_terrain_info() {
    Dictionary info;

    if (sim_id == 0) return info;

    float width = 0.0f, height = 0.0f, min_elev = 0.0f, max_elev = 0.0f;
    int32_t result = fire_sim_get_terrain_info(sim_id, &width, &height, &min_elev, &max_elev);

    if (result == FIRE_SIM_SUCCESS) {
        info["width"] = width;
        info["height"] = height;
        info["min_elevation"] = min_elev;
        info["max_elevation"] = max_elev;
    }

    return info;
}

// Multiplayer
bool FireSimulation::submit_player_action(uint8_t action_type, uint32_t player_id, Vector3 position, float param1, uint32_t param2) {
    if (sim_id == 0) return false;

    CPlayerAction action;
    action.action_type = action_type;
    action.player_id = player_id;
    action.timestamp = (float)Time::get_singleton()->get_ticks_msec() / 1000.0f;
    action.position_x = position.x;
    action.position_y = position.y;
    action.position_z = position.z;
    action.param1 = param1;
    action.param2 = param2;

    return fire_sim_submit_player_action(sim_id, &action);
}

uint32_t FireSimulation::get_frame_number() const {
    if (sim_id == 0) return 0;
    return fire_sim_get_frame_number(sim_id);
}
```

Create `register_types.h` and `register_types.cpp`:

```cpp
// register_types.h
#ifndef GDEXT_REGISTER_TYPES_H
#define GDEXT_REGISTER_TYPES_H

#include <godot_cpp/core/class_db.hpp>

using namespace godot;

void initialize_fire_simulation_module(ModuleInitializationLevel p_level);
void uninitialize_fire_simulation_module(ModuleInitializationLevel p_level);

#endif

// register_types.cpp
#include "register_types.h"
#include "fire_sim_wrapper.h"

using namespace godot;

void initialize_fire_simulation_module(ModuleInitializationLevel p_level) {
    if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
        return;
    }
    ClassDB::register_class<FireSimulation>();
}

void uninitialize_fire_simulation_module(ModuleInitializationLevel p_level) {
    if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
        return;
    }
}
```

## Step 4: Build Configuration

Create `fire_sim.gdextension`:

```ini
[configuration]
entry_symbol = "gdext_rust_fire_sim_library_init"
compatibility_minimum = 4.1

[libraries]
linux.x86_64.release = "res://gdextension/bin/libfire_sim_extension.so"
linux.x86_64.debug = "res://gdextension/bin/libfire_sim_extension.so.debug"
windows.x86_64.release = "res://gdextension/bin/fire_sim_extension.dll"
windows.x86_64.debug = "res://gdextension/bin/fire_sim_extension.dll"
macos.universal.release = "res://gdextension/bin/libfire_sim_extension.framework"
macos.universal.debug = "res://gdextension/bin/libfire_sim_extension.framework"
```

Create `SConstruct` for building:

```python
#!/usr/bin/env python3

import os
import sys

env = SConscript("godot-cpp/SConstruct")

# Add include paths
env.Append(CPPPATH=["src/"])
env.Append(CPPPATH=["thirdparty/fire_sim_ffi/include/"])

# Add library paths and link libraries
env.Append(LIBPATH=["thirdparty/fire_sim_ffi/lib/"])
if sys.platform == "linux":
    env.Append(LIBS=["fire_sim_ffi"])
    env.Append(LINKFLAGS=["-Wl,-rpath=."])
elif sys.platform == "win32":
    env.Append(LIBS=["fire_sim_ffi"])
elif sys.platform == "darwin":
    env.Append(LIBS=["fire_sim_ffi"])

# Compile sources
sources = [
    "src/register_types.cpp",
    "src/fire_sim_wrapper.cpp"
]

library = env.SharedLibrary(
    target="bin/fire_sim_extension",
    source=sources,
)

Default(library)
```

## Step 5: GDScript Wrapper (Optional)

Create `FireSimulationManager.gd` for easier scripting:

```gdscript
# FireSimulationManager.gd
extends FireSimulation

class_name FireSimulationManager

# Signals
signal fire_started
signal fire_spread
signal fire_extinguished
signal suppression_applied

func _ready():
    super._ready()
    connect_signals()

func connect_signals():
    # Connect to simulation events if implementing signal support
    pass

# High-level API for game logic
func create_grassfire(center: Vector3, radius: float) -> int:
    """Ignite a grassfire in a circular area"""
    var grass_elements = []
    var ignited_count = 0
    
    # Add grass fuel elements in a circle
    for i in range(20):
        var angle = (2.0 * PI * i) / 20.0
        var x = center.x + radius * cos(angle)
        var z = center.z + radius * sin(angle)
        
        var element_id = add_fuel_element(
            Vector3(x, 0, z),
            2,  # Dry grass
            10, # Ground litter
            0.5,
            -1
        )
        
        if element_id >= 0:
            grass_elements.append(element_id)
    
    # Ignite center
    if not grass_elements.is_empty():
        ignite_element(grass_elements[0], 600.0)
        ignited_count = 1
    
    fire_started.emit()
    return ignited_count

func create_eucalyptus_forest(start_pos: Vector3, grid_size: int, spacing: float) -> int:
    """Create a forest of eucalyptus trees"""
    var tree_count = 0
    
    for x in range(grid_size):
        for z in range(grid_size):
            var pos = start_pos + Vector3(x * spacing, 0, z * spacing)
            
            # Trunk
            var trunk_id = add_fuel_element(pos, 0, 1, 10.0, -1)
            
            if trunk_id >= 0:
                # Crown (canopy)
                var crown_pos = pos + Vector3(0, 15, 0)
                add_fuel_element(crown_pos, 0, 6, 5.0, trunk_id)
                tree_count += 1
    
    return tree_count

func apply_aerial_retardant_drop(position: Vector3, radius: float, mass: float) -> int:
    """Apply fire retardant from aircraft"""
    var treated = apply_suppression_to_elements(position, radius, mass / 10.0, 3)
    suppression_applied.emit()
    return treated

func get_fire_stats() -> Dictionary:
    """Get comprehensive fire statistics"""
    var snapshot = get_snapshot()
    var weather = get_weather()
    
    return {
        "frame": snapshot.get("frame_number", 0),
        "time": snapshot.get("simulation_time", 0.0),
        "burning": snapshot.get("burning_element_count", 0),
        "fuel_consumed": snapshot.get("total_fuel_consumed", 0.0),
        "embers": snapshot.get("active_ember_count", 0),
        "haines": snapshot.get("haines_index", 0),
        "temperature": weather["temperature"],
        "humidity": weather["humidity"],
        "wind_speed": weather["wind_speed"],
        "wind_direction": weather["wind_direction"]
    }

func update_live_weather_from_api(temp: float, humidity: float, wind: float, direction: float, drought: float) -> bool:
    """Update weather from external API (BOM, NOAA, etc.)"""
    var result = set_weather(temp, humidity, wind, direction, drought)
    return result == 0
```

## Step 6: Scene Setup

Create a scene `FireSimulationManager.tscn`:

```yaml
[gd_scene load_steps=2 format=3 uid="uid:fire_sim_manager"]

[ext_resource type="Script" path="res://scenes/FireSimulationManager.gd"]

[node name="FireSimulationManager" type="Node3D"]
script = ExtResource("res://scenes/FireSimulationManager.gd")
map_width = 1000.0
map_height = 1000.0
grid_cell_size = 2.0
terrain_type = 0

# Add visualization node
[node name="FireVisualization" type="Node3D" parent="."]
```

## Step 7: Visualization System

Create `FireVisualization.gd` for rendering fire effects:

```gdscript
# FireVisualization.gd
extends Node3D

@onready var fire_sim = get_parent()
@onready var multi_mesh = MultiMesh.new()

var flame_particles: PackedScene
var smoke_particles: PackedScene

func _ready():
    # Load particle presets
    flame_particles = preload("res://particles/FlameParticle.tscn")
    smoke_particles = preload("res://particles/SmokeParticle.tscn")

func _process(_delta):
    update_fire_visualization()

func update_fire_visualization():
    # Get burning elements
    var burning = fire_sim.get_burning_elements()
    
    for elem in burning:
        var pos = Vector3(elem["id"] % 100, 0, elem["id"] / 100) * 2.0  # Example positioning
        var intensity = elem["temperature"] / 1000.0
        var flame_height = elem["flame_height"]
        
        # Spawn or update flame particles
        var flame = flame_particles.instantiate()
        flame.position = pos
        flame.scale = Vector3(1, intensity * flame_height, 1)
        add_child(flame)

func get_haines_index_warning() -> String:
    var haines = fire_sim.get_haines_index()
    
    match haines:
        2, 3:
            return "Very low fire weather potential"
        4:
            return "Low fire weather potential"
        5:
            return "Moderate fire weather potential"
        6:
            return "HIGH FIRE WEATHER POTENTIAL"
        _:
            return "Unknown"
```

## Step 8: Error Handling

The FFI provides error codes for all operations:

```gdscript
# FireSimulationManager.gd
const FIRE_SIM_SUCCESS = 0
const FIRE_SIM_INVALID_ID = -1
const FIRE_SIM_NULL_POINTER = -2
const FIRE_SIM_INVALID_FUEL = -3
const FIRE_SIM_INVALID_TERRAIN = -4
const FIRE_SIM_LOCK_ERROR = -5

func check_result(result: int, operation: String) -> bool:
    match result:
        FIRE_SIM_SUCCESS:
            return true
        FIRE_SIM_INVALID_ID:
            push_error("Operation '%s': Invalid simulation ID" % operation)
        FIRE_SIM_NULL_POINTER:
            push_error("Operation '%s': Null pointer error" % operation)
        FIRE_SIM_INVALID_FUEL:
            push_error("Operation '%s': Invalid fuel/agent type" % operation)
        FIRE_SIM_INVALID_TERRAIN:
            push_error("Operation '%s': Invalid terrain type" % operation)
        FIRE_SIM_LOCK_ERROR:
            push_error("Operation '%s': Internal lock error" % operation)
        _:
            push_error("Operation '%s': Unknown error (%d)" % [operation, result])
    
    return false
```

## Fuel Type Reference

| ID | Fuel Type | Description | Spread Rate | Crown Fire Threshold |
|----|-----------|-------------|-------------|----------------------|
| 0 | Eucalyptus Stringybark | Extreme fire behavior, 25km spotting | Very Fast | Low |
| 1 | Eucalyptus Smooth Bark | Moderate behavior, 10km spotting | Fast | Medium |
| 2 | Dry Grass | Fast ignition, rapid spread | Very Fast | High (no crown) |
| 3 | Shrubland | Medium ignition, medium spread | Medium | Medium |
| 4 | Dead Wood/Litter | High susceptibility to ignition | Slow | High (smoldering) |
| 5 | Green Vegetation | Fire resistant | Very Slow | Very High |
| 6 | Water | Non-burnable, blocks heat | None | None |
| 7 | Rock | Non-burnable, reduces heat | None | None |

## Part Type Reference

| ID | Part Type | Description | Height Factor |
|----|-----------|-------------|----------------|
| 0 | Root | Underground/base, slowest burn | 0.0 |
| 1 | TrunkLower | Lower 0-33% of trunk | 0.15 |
| 2 | TrunkMiddle | Middle 33-67% of trunk | 0.50 |
| 3 | TrunkUpper | Upper 67-100% of trunk | 0.85 |
| 4 | BarkLayer | Bark on trunk | 0.40 |
| 5 | Branch | Branch with leaves | 0.70 |
| 6 | Crown | Canopy foliage (crown fire potential) | 1.0 |
| 7 | GroundLitter | Dead material on ground | 0.05 |

## Suppression Agent Type Reference

| ID | Agent Type | Effect | Duration | Cost |
|----|------------|--------|----------|------|
| 0 | Water | Rapid cooling, no residue | Minutes | Low |
| 1 | FoamClassA | Foam suppression, moderate cooling | Hours | Medium |
| 2 | ShortTermRetardant | Fire retardant, residual protection | Hours | Medium |
| 3 | LongTermRetardant | Enhanced retardant, stronger residue | Days | High |
| 4 | WettingAgent | Reduces ignition threshold | Hours | Medium |

## Example Game Implementation

### Multiplayer Fire Fighting Game

```gdscript
# GameController.gd
extends Node

@onready var fire_sim = $FireSimulationManager

func _ready():
    # Initialize simulation
    fire_sim.initialize_simulation()
    
    # Create initial fire
    fire_sim.create_eucalyptus_forest(Vector3(0, 0, 0), 10, 50.0)
    fire_sim.create_grassfire(Vector3(500, 0, 500), 100.0)

func _process(_delta):
    # Update HUD with stats
    var stats = fire_sim.get_fire_stats()
    update_hud(stats)
    
    # Check fire weather warning
    if stats["haines"] >= 6:
        show_danger_alert("Extreme fire weather!")

func player_drop_water(player_pos: Vector3):
    """Handle water drop from player"""
    var treated = fire_sim.apply_suppression_to_elements(
        player_pos,
        50.0,  # 50m radius
        10000.0, # 10kg per element
        0  # Water
    )
    print("Treated %d elements" % treated)

func player_drop_retardant(player_pos: Vector3):
    """Handle retardant drop from aircraft"""
    var treated = fire_sim.apply_aerial_retardant_drop(
        player_pos,
        200.0,  # 200m radius (wider spread)
        50000.0 # 50kg per element
    )
    print("Applied retardant to %d elements" % treated)

func update_live_weather():
    """Fetch and apply real weather data"""
    # Example: call external weather API
    var weather_data = await fetch_bom_data()
    
    fire_sim.update_live_weather_from_api(
        weather_data.temperature,
        weather_data.humidity,
        weather_data.wind_speed,
        weather_data.wind_direction,
        weather_data.drought_factor
    )

func update_hud(stats: Dictionary):
    %FrameLabel.text = "Frame: %d" % stats["frame"]
    %BurningLabel.text = "Burning: %d" % stats["burning"]
    %FuelLabel.text = "Fuel Consumed: %.1f kg" % stats["fuel_consumed"]
    %HainesLabel.text = "Haines Index: %d" % stats["haines"]
    %TemperatureLabel.text = "Temp: %.1f°C" % stats["temperature"]
    %WindLabel.text = "Wind: %.1f km/h @ %.0f°" % [stats["wind_speed"], stats["wind_direction"]]
```

## Performance Tips

1. **Grid Cell Size**: Use 2-5m for balance between accuracy and performance. Larger = faster but less detail.

2. **Update Frequency**: Query burning elements every 2-3 frames instead of every frame.

3. **Spatial Culling**: Only visualize fire elements within camera view.

4. **LOD System**: 
   - Close: Full detail particle effects
   - Medium: Reduced particle count
   - Far: Billboard or heatmap visualization

5. **Multi-threading**: GDExtension runs in the rendering thread; use async tasks for heavy queries.

## Troubleshooting

### Compilation Errors

```bash
# Ensure godot-cpp is cloned and built
git clone --recursive https://github.com/godotengine/godot-cpp.git
cd godot-cpp
scons platform=linux

# Then build extension
scons
```

### Library Not Found

- **Linux**: Check `LD_LIBRARY_PATH` includes `gdextension/thirdparty/fire_sim_ffi/lib/linux/`
- **Windows**: Place `fire_sim_ffi.dll` in project root or `gdextension/bin/`
- **macOS**: Sign dylib: `codesign --force --deep gdextension/thirdparty/fire_sim_ffi/lib/macos/libfire_sim_ffi.dylib`

### Runtime Crashes

- Verify all pointers are properly initialized
- Check that simulation ID is valid before queries
- Ensure arrays are large enough for element counts

## Resources

- [Godot 4.x GDExtension Documentation](https://docs.godotengine.org/en/stable/tutorials/advanced_guides/creating_cpp_modules.html)
- [godot-cpp Repository](https://github.com/godotengine/godot-cpp)
- [Rust FFI Documentation](https://doc.rust-lang.org/nomicon/ffi.html)
- [Australian Fire Behavior Research](https://www.csiro.au/en/research/natural-disasters/bushfires)

## Support

For issues or questions:
- GitHub Issues: https://github.com/mineubob/Australia-Fire-Sim/issues
- Godot Community: https://ask.godotengine.org/

---

**Note**: This simulation is designed for scientific accuracy and emergency response training. Fire behavior is based on real-world physics and Australian bushfire research.
