# Customization Guide

## Complete Configuration Reference

The demo-gui is **fully customizable** through the `DemoConfig` struct. All simulation parameters can be configured by editing the config in `main()`.

## Configuration Structure

```rust
pub struct DemoConfig {
    // ========== TERRAIN SETTINGS ==========
    pub map_width: f32,              // Map width in meters
    pub map_height: f32,             // Map height in meters  
    pub terrain_type: TerrainType,   // Flat, Hill, or Valley
    
    // ========== FIRE SETTINGS ==========
    pub elements_x: usize,           // Grid width (number of fuel elements)
    pub elements_y: usize,           // Grid height (number of fuel elements)
    pub fuel_mass: f32,              // Mass per element in kg
    pub fuel_type: FuelType,         // Type of fuel
    pub initial_ignitions: usize,    // How many elements to ignite at start
    pub spacing: f32,                // Distance between elements in meters
    
    // ========== WEATHER SETTINGS ==========
    pub temperature: f32,            // Temperature in °C
    pub humidity: f32,               // Humidity as fraction (0.0-1.0)
    pub wind_speed: f32,             // Wind speed in m/s
    pub wind_direction: f32,         // Wind direction in degrees (0-360)
    pub drought_factor: f32,         // Drought factor (0-12+)
}
```

## Default Configuration

```rust
DemoConfig {
    // Terrain: 200m x 200m with central hill
    map_width: 200.0,
    map_height: 200.0,
    terrain_type: TerrainType::Hill,
    
    // Fire: 10x10 grid of dry grass
    elements_x: 10,
    elements_y: 10,
    fuel_mass: 5.0,
    fuel_type: FuelType::DryGrass,
    initial_ignitions: 5,
    spacing: 8.0,
    
    // Weather: Extreme fire danger
    temperature: 35.0,      // Hot day
    humidity: 0.15,         // Very dry (15%)
    wind_speed: 20.0,       // Strong wind
    wind_direction: 45.0,   // NE wind
    drought_factor: 10.0,   // Extreme
}
```

**FFDI**: 85.3 (Extreme)  
**Total Elements**: 100  
**Coverage**: 72m x 72m

## Terrain Type Options

### TerrainType::Flat
```rust
terrain_type: TerrainType::Flat,
```
- All at same elevation (0m)
- No slope effects
- Pure horizontal fire spread
- Good for baseline testing

### TerrainType::Hill
```rust
terrain_type: TerrainType::Hill,
```
- Central hill up to 80m elevation
- Gaussian distribution
- Strong uphill fire acceleration
- **Default configuration**

### TerrainType::Valley
```rust
terrain_type: TerrainType::Valley,
```
- Two hills with valley between
- Hills up to 80m elevation
- Complex fire behavior
- Fire funnels through valley

## Fuel Type Options

### FuelType::DryGrass
```rust
fuel_type: FuelType::DryGrass,
```
- Fast ignition (250°C)
- Rapid spread
- Low intensity
- **Default configuration**

### FuelType::EucalyptusStringybark
```rust
fuel_type: FuelType::EucalyptusStringybark,
```
- **Extreme fire behavior**
- 25km ember spotting
- Oil vapor explosions at 232°C
- Ladder fuels cause crown fires
- High intensity

### FuelType::EucalyptusSmoothBark
```rust
fuel_type: FuelType::EucalyptusSmoothBark,
```
- Moderate fire behavior
- 10km ember spotting
- Less ladder fuel than stringybark
- Medium intensity

### FuelType::Shrubland
```rust
fuel_type: FuelType::Shrubland,
```
- Medium ignition (300°C)
- Moderate spread
- Medium intensity

### FuelType::DeadWood
```rust
fuel_type: FuelType::DeadWood,
```
- Low moisture (5%)
- High susceptibility
- Sustains fire well
- Medium intensity

## Example Configurations

### 1. Small Controlled Burn
```rust
let config = DemoConfig {
    map_width: 100.0,
    map_height: 100.0,
    terrain_type: TerrainType::Flat,
    
    elements_x: 5,
    elements_y: 5,
    fuel_mass: 2.0,
    fuel_type: FuelType::DryGrass,
    initial_ignitions: 1,
    spacing: 5.0,
    
    temperature: 25.0,
    humidity: 0.40,
    wind_speed: 5.0,
    wind_direction: 0.0,
    drought_factor: 4.0,
};
```
**FFDI**: ~12 (Moderate)  
**Purpose**: Training scenario, slow spread

### 2. Extreme Bushfire Scenario
```rust
let config = DemoConfig {
    map_width: 400.0,
    map_height: 400.0,
    terrain_type: TerrainType::Hill,
    
    elements_x: 20,
    elements_y: 20,
    fuel_mass: 10.0,
    fuel_type: FuelType::EucalyptusStringybark,
    initial_ignitions: 20,
    spacing: 12.0,
    
    temperature: 45.0,
    humidity: 0.08,
    wind_speed: 30.0,
    wind_direction: 90.0,
    drought_factor: 12.0,
};
```
**FFDI**: ~120+ (CATASTROPHIC)  
**Purpose**: Emergency response training, extreme conditions

### 3. Valley Fire Study
```rust
let config = DemoConfig {
    map_width: 300.0,
    map_height: 300.0,
    terrain_type: TerrainType::Valley,
    
    elements_x: 15,
    elements_y: 15,
    fuel_mass: 7.0,
    fuel_type: FuelType::Shrubland,
    initial_ignitions: 10,
    spacing: 10.0,
    
    temperature: 38.0,
    humidity: 0.12,
    wind_speed: 18.0,
    wind_direction: 0.0,  // North wind
    drought_factor: 9.0,
};
```
**FFDI**: ~75 (Extreme)  
**Purpose**: Topographic fire behavior study

### 4. Large Area Simulation
```rust
let config = DemoConfig {
    map_width: 500.0,
    map_height: 500.0,
    terrain_type: TerrainType::Hill,
    
    elements_x: 25,
    elements_y: 25,
    fuel_mass: 8.0,
    fuel_type: FuelType::EucalyptusSmoothBark,
    initial_ignitions: 15,
    spacing: 15.0,
    
    temperature: 40.0,
    humidity: 0.10,
    wind_speed: 25.0,
    wind_direction: 135.0,  // SE wind
    drought_factor: 11.0,
};
```
**FFDI**: ~105 (Catastrophic)  
**Purpose**: Large-scale fire progression study

## Wind Direction Reference

```
        0° (North)
           |
           |
270° ------+------ 90° (East)
 (West)    |
           |
        180° (South)
```

Common directions:
- 0° = North
- 45° = Northeast (default)
- 90° = East
- 135° = Southeast
- 180° = South
- 225° = Southwest
- 270° = West
- 315° = Northwest

## Fire Danger Rating (FFDI)

| FFDI Range | Rating | Color Code |
|------------|--------|------------|
| 0-5 | Low | Green |
| 5-12 | Moderate | Blue |
| 12-24 | High | Yellow |
| 24-50 | Very High | Orange |
| 50-75 | Severe | Red |
| 75-100 | Extreme | Dark Red |
| 100+ | CATASTROPHIC | Purple/Black |

## How to Apply Configuration

1. Open `demo-gui/src/main.rs`
2. Find the `main()` function
3. Modify the config:

```rust
fn main() {
    // Customize here!
    let config = DemoConfig {
        terrain_type: TerrainType::Valley,
        fuel_type: FuelType::EucalyptusStringybark,
        temperature: 42.0,
        wind_speed: 28.0,
        // ... other parameters
    };
    
    // Rest of the code stays the same
    App::new()
        .add_plugins(...)
        .insert_resource(config)
        // ...
}
```

4. Rebuild and run:
```bash
cargo build --release -p demo-gui
cargo run --release -p demo-gui
```

## Tips

- **Start small**: Begin with small grid (5x5) and low FFDI for learning
- **Increase gradually**: Add complexity as you understand fire behavior
- **Test extremes**: Try CATASTROPHIC conditions to see system limits
- **Match real conditions**: Use actual weather data for training scenarios
- **Document changes**: Keep notes on configurations that produce interesting results

## Validation

The GUI validates:
- `initial_ignitions` is capped at `elements_x * elements_y`
- All parameters are applied to both physics and visuals
- Reset (R key) recreates simulation with current config
- Configuration is immutable during simulation run
