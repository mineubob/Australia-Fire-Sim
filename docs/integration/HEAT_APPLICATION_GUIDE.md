# Heat Application Guide

This guide explains how to apply heat to fuel elements for realistic fire scenarios such as backburns, controlled burns, and radiant heating from external sources.

## Table of Contents

- [Overview](#overview)
- [Heat Energy Calculation](#heat-energy-calculation)
  - [Radiant Heat from Fire Sources](#radiant-heat-from-fire-sources)
  - [Controlled Burn / Drip Torch](#controlled-burn--drip-torch)
  - [External Heat Source](#external-heat-source-eg-burning-structure)
  - [From Fire Intensity](#from-fire-intensity-byrams-method)
- [API Usage](#api-usage)
  - [Rust](#rust-api)
  - [C++ (via FFI)](#c-api-via-ffi)
  - [C# (via FFI)](#c-api-via-ffi-1)
- [Complete Examples](#complete-examples)
  - [Rust: Backburn Scenario](#rust-backburn-scenario)
  - [C++: Drip Torch Implementation](#c-drip-torch-implementation)
  - [C#: Structure Fire Radiant Heat](#c-structure-fire-radiant-heat)

---

## Overview

The simulation provides three distinct ignition/heating pathways:

1. **`ignite_element`** - Instant ignition (bypasses moisture physics)
   - Use for: Lightning strikes, explosions, testing
   - Realistic for: High-energy instantaneous ignition sources

2. **`apply_heat_to_element`** - Physics-based heating (respects moisture)
   - Use for: Backburns, controlled burns, radiant pre-heating
   - Realistic for: Gradual heating scenarios

3. **Internal heat transfer** - Automatic during simulation
   - Use for: Natural fire spread element-to-element
   - Realistic for: Fire spreading through fuel bed

---

## Heat Energy Calculation

Heat energy (`heat_kj`) depends on the physical heat source. Here are the standard methods:

### Radiant Heat from Fire Sources

Based on **Stefan-Boltzmann law** for thermal radiation:

```
Radiant Power = σ × ε × (T_source⁴ - T_target⁴)

Where:
  σ = Stefan-Boltzmann constant = 5.67×10⁻⁸ W/(m²·K⁴)
  ε = Emissivity (0.95 for flames)
  T = Temperature in Kelvin
```

**View factor** accounts for geometry (how much of source the target "sees"):

```
View Factor = source_area / (4π × distance²)
```

**Final heat flux:**

```
Flux (W/m²) = Radiant Power × View Factor
Heat (kJ) = Flux × target_area × dt × 0.001
```

**Example values:**
- Fire at 800°C, 10m away: ~15-25 kJ/s per m²
- Fire at 1200°C, 5m away: ~80-120 kJ/s per m²
- Campfire 2m away: ~5-10 kJ/s per m²

### Controlled Burn / Drip Torch

For liquid fuel delivery systems (drip torches, flame throwers):

```
Heat (kJ) = fuel_flow_rate × fuel_density × heat_content × dt

Where:
  fuel_flow_rate = L/s (typical drip torch: 0.5 L/min = 0.0083 L/s)
  fuel_density = kg/L (diesel: 0.85 kg/L)
  heat_content = kJ/kg (diesel: 43,000 kJ/kg)
  dt = time step (seconds)
```

**Example:**
- Drip torch (0.5 L/min diesel): ~304 kJ/s sustained
- Flame thrower (2 L/min): ~1,217 kJ/s sustained

### External Heat Source (e.g., Burning Structure)

For large external fire sources like buildings:

```
Radiant Power = total_fire_power × radiant_fraction
Flux = Radiant Power / (4π × distance²)
Heat (kJ) = Flux × target_area × dt × 0.001

Where:
  total_fire_power = MW (house fire: 10-20 MW)
  radiant_fraction = 0.3-0.4 (30-40% of total radiates)
```

**Example values:**
- House fire 20m away: ~30-50 kJ/s per m²
- Vehicle fire 10m away: ~50-80 kJ/s per m²
- Industrial fire 50m away: ~100-200 kJ/s per m²

### From Fire Intensity (Byram's Method)

Using **Byram's fireline intensity** (kW/m):

```
Flame Height (m) = 0.0775 × Intensity^0.46
View Factor = height / √(height² + distance²)
Radiant Flux = Intensity × 1000 × radiant_fraction × View Factor
Heat (kJ) = Radiant Flux × dt × 0.001

Where:
  Intensity = kW/m (fireline intensity)
  radiant_fraction = ~0.3 (30% radiates)
```

**Intensity reference:**
- Low intensity fire: 500-2,000 kW/m
- Moderate intensity: 2,000-10,000 kW/m
- High intensity: 10,000-50,000 kW/m
- Extreme (Black Saturday): 50,000-100,000+ kW/m

---

## API Usage

### Rust API

```rust
use fire_sim_core::simulation::FireSimulation;
use fire_sim_core::core_types::element::Vec3;

// Apply heat to a fuel element
pub fn apply_heat_to_element(
    simulation: &mut FireSimulation,
    element_id: u32,
    heat_kj: f32,
    dt: f32
) {
    simulation.apply_heat_to_element(element_id, heat_kj, dt);
}

// Check if element ignited after heating
pub fn is_element_burning(simulation: &FireSimulation, element_id: u32) -> bool {
    if let Some(element) = simulation.get_element(element_id) {
        element.ignited
    } else {
        false
    }
}
```

### C++ API (via FFI)

```cpp
#include "fire_sim.h"
#include <cmath>

// Apply radiant heat from fire source
void applyRadiantHeat(
    uintptr_t simId,
    uint32_t elementId,
    float sourceTemp,      // °C
    float targetTemp,      // °C
    float distance,        // meters
    float sourceArea,      // m²
    float targetArea,      // m²
    float dt               // seconds
) {
    const float STEFAN_BOLTZMANN = 5.67e-8f; // W/(m²·K⁴)
    const float EMISSIVITY = 0.95f;
    const float PI = 3.14159265f;
    
    // Convert to Kelvin
    float tempSourceK = sourceTemp + 273.15f;
    float tempTargetK = targetTemp + 273.15f;
    
    // Stefan-Boltzmann radiation
    float radiantPower = STEFAN_BOLTZMANN * EMISSIVITY 
                       * (pow(tempSourceK, 4) - pow(tempTargetK, 4));
    
    // View factor
    float viewFactor = sourceArea / (4.0f * PI * distance * distance);
    
    // Heat flux
    float flux = radiantPower * viewFactor;
    
    // Total heat energy
    float heatKj = flux * targetArea * dt * 0.001f;
    
    // Apply to element
    fire_sim_apply_heat_to_element(simId, elementId, heatKj, dt);
}

// Apply drip torch heat
void applyDripTorchHeat(
    uintptr_t simId,
    uint32_t elementId,
    float flowRateLPerMin,  // L/min
    float dt                // seconds
) {
    const float FUEL_DENSITY = 0.85f;     // kg/L (diesel)
    const float HEAT_CONTENT = 43000.0f;  // kJ/kg
    
    float flowRateLPerSec = flowRateLPerMin / 60.0f;
    float heatKj = flowRateLPerSec * FUEL_DENSITY * HEAT_CONTENT * dt;
    
    fire_sim_apply_heat_to_element(simId, elementId, heatKj, dt);
}
```

### C# API (via FFI)

```csharp
using System;
using System.Runtime.InteropServices;

public class FireSimulation
{
    [DllImport("fire_sim_ffi")]
    private static extern int fire_sim_apply_heat_to_element(
        UIntPtr simId,
        uint elementId,
        float heatKj,
        float dt
    );
    
    // Apply radiant heat from fire source
    public static void ApplyRadiantHeat(
        UIntPtr simId,
        uint elementId,
        float sourceTemp,      // °C
        float targetTemp,      // °C
        float distance,        // meters
        float sourceArea,      // m²
        float targetArea,      // m²
        float dt               // seconds
    ) {
        const float STEFAN_BOLTZMANN = 5.67e-8f; // W/(m²·K⁴)
        const float EMISSIVITY = 0.95f;
        const float PI = 3.14159265f;
        
        // Convert to Kelvin
        float tempSourceK = sourceTemp + 273.15f;
        float tempTargetK = targetTemp + 273.15f;
        
        // Stefan-Boltzmann radiation
        float radiantPower = STEFAN_BOLTZMANN * EMISSIVITY 
                           * (MathF.Pow(tempSourceK, 4) - MathF.Pow(tempTargetK, 4));
        
        // View factor
        float viewFactor = sourceArea / (4.0f * PI * distance * distance);
        
        // Heat flux
        float flux = radiantPower * viewFactor;
        
        // Total heat energy
        float heatKj = flux * targetArea * dt * 0.001f;
        
        // Apply to element
        fire_sim_apply_heat_to_element(simId, elementId, heatKj, dt);
    }
    
    // Apply drip torch heat
    public static void ApplyDripTorchHeat(
        UIntPtr simId,
        uint elementId,
        float flowRateLPerMin,  // L/min
        float dt                // seconds
    ) {
        const float FUEL_DENSITY = 0.85f;     // kg/L (diesel)
        const float HEAT_CONTENT = 43000.0f;  // kJ/kg
        
        float flowRateLPerSec = flowRateLPerMin / 60.0f;
        float heatKj = flowRateLPerSec * FUEL_DENSITY * HEAT_CONTENT * dt;
        
        fire_sim_apply_heat_to_element(simId, elementId, heatKj, dt);
    }
    
    // Apply heat from fire intensity (Byram's method)
    public static void ApplyFireIntensityHeat(
        UIntPtr simId,
        uint elementId,
        float intensityKwPerM,  // kW/m (fireline intensity)
        float distance,         // meters
        float dt                // seconds
    ) {
        const float RADIANT_FRACTION = 0.3f; // 30% radiates
        
        // Byram's flame height formula
        float flameHeight = 0.0775f * MathF.Pow(intensityKwPerM, 0.46f);
        
        // View factor (Thomas 1963)
        float viewFactor = flameHeight / MathF.Sqrt(flameHeight * flameHeight + distance * distance);
        
        // Radiant flux
        float flux = intensityKwPerM * 1000.0f * RADIANT_FRACTION * viewFactor;
        float heatKj = flux * dt * 0.001f;
        
        fire_sim_apply_heat_to_element(simId, elementId, heatKj, dt);
    }
}
```

---

## Complete Examples

### Rust: Backburn Scenario

```rust
use fire_sim_core::simulation::FireSimulation;
use fire_sim_core::core_types::element::Vec3;
use fire_sim_core::core_types::fuel::Fuel;
use fire_sim_core::core_types::element::FuelPart;
use fire_sim_core::grid::TerrainData;

fn simulate_backburn() {
    // Create simulation
    let terrain = TerrainData::flat(200.0, 200.0, 5.0, 0.0);
    let mut sim = FireSimulation::new(5.0, terrain);
    
    // Create fuel line to protect (e.g., near structure)
    let mut fuel_line = Vec::new();
    for i in 0..20 {
        let x = 50.0 + i as f32 * 2.0;
        let fuel = Fuel::dry_grass();
        let id = sim.add_fuel_element(
            Vec3::new(x, 100.0, 0.5),
            fuel,
            3.0,
            FuelPart::GroundVegetation,
            None,
        );
        fuel_line.push(id);
    }
    
    // Main fire approaching from south (higher y values)
    let main_fire_intensity = 8000.0; // kW/m (moderate-high)
    let main_fire_distance = 150.0;   // meters away initially
    
    // Start backburn from control line (lower y values)
    // Light backburn with drip torch
    for &fuel_id in &fuel_line {
        // Drip torch: 0.5 L/min diesel
        let flow_rate = 0.5 / 60.0;        // L/s
        let fuel_density = 0.85;            // kg/L
        let heat_content = 43000.0;         // kJ/kg
        let dt = 0.1;                       // 100ms timestep
        
        let heat_kj = flow_rate * fuel_density * heat_content * dt;
        
        // Apply sustained heat for 5 seconds
        for _ in 0..50 {
            sim.apply_heat_to_element(fuel_id, heat_kj, dt);
            sim.update(dt);
        }
    }
    
    // Simulate main fire approaching
    let mut distance = main_fire_distance;
    for _ in 0..1000 {
        // Main fire moves closer
        distance -= 0.5; // Moves 0.5m per timestep
        
        // Apply radiant heat from main fire
        for &fuel_id in &fuel_line {
            if let Some(element) = sim.get_element(fuel_id) {
                // Skip if already burned
                if element.fuel_remaining < 0.1 {
                    continue;
                }
                
                // Calculate radiant heat using Byram's method
                let flame_height = 0.0775 * main_fire_intensity.powf(0.46);
                let view_factor = flame_height / (flame_height.powi(2) + distance.powi(2)).sqrt();
                let radiant_fraction = 0.3;
                
                let flux = main_fire_intensity * 1000.0 * radiant_fraction * view_factor;
                let heat_kj = flux * 0.1 * 0.001; // 100ms timestep
                
                sim.apply_heat_to_element(fuel_id, heat_kj, 0.1);
            }
        }
        
        sim.update(0.1);
        
        // Check if backburn consumed fuel before main fire arrived
        if distance < 20.0 {
            println!("Main fire reached control line!");
            break;
        }
    }
    
    // Check effectiveness
    let burned_by_backburn = fuel_line.iter()
        .filter(|&&id| {
            if let Some(e) = sim.get_element(id) {
                e.fuel_remaining < 0.1
            } else {
                false
            }
        })
        .count();
    
    println!("Backburn consumed {}/{} fuel elements", burned_by_backburn, fuel_line.len());
}
```

### C++: Drip Torch Implementation

```cpp
#include "fire_sim.h"
#include <vector>
#include <cmath>
#include <iostream>

class DripTorch {
private:
    float flowRate;        // L/min
    float fuelDensity;     // kg/L
    float heatContent;     // kJ/kg
    bool active;
    
public:
    DripTorch(float flowRateLPerMin = 0.5f) 
        : flowRate(flowRateLPerMin)
        , fuelDensity(0.85f)    // Diesel
        , heatContent(43000.0f)
        , active(false) 
    {}
    
    void activate() { active = true; }
    void deactivate() { active = false; }
    
    // Apply heat to fuel element
    void applyHeat(uintptr_t simId, uint32_t elementId, float dt) {
        if (!active) return;
        
        float flowRateLPerSec = flowRate / 60.0f;
        float heatKj = flowRateLPerSec * fuelDensity * heatContent * dt;
        
        fire_sim_apply_heat_to_element(simId, elementId, heatKj, dt);
    }
};

void simulateControlledBurn() {
    // Create simulation
    uintptr_t simId;
    if (fire_sim_create(200.0f, 200.0f, 5.0f, 0, &simId) != FIRE_SIM_SUCCESS) {
        std::cerr << "Failed to create simulation" << std::endl;
        return;
    }
    
    // Create fuel elements
    std::vector<uint32_t> fuelIds;
    for (int i = 0; i < 50; i++) {
        for (int j = 0; j < 50; j++) {
            float x = 10.0f + i * 3.0f;
            float y = 10.0f + j * 3.0f;
            
            uint32_t elementId;
            if (fire_sim_add_fuel_element(
                simId,
                x, y, 0.5f,      // position
                2,               // dry_grass fuel type
                0,               // ground vegetation
                3.0f,            // mass
                -1,              // no parent
                &elementId
            ) == FIRE_SIM_SUCCESS) {
                fuelIds.push_back(elementId);
            }
        }
    }
    
    // Create drip torch operator
    DripTorch torch(0.5f); // 0.5 L/min
    torch.activate();
    
    // Walk through area lighting fires
    float dt = 0.1f; // 100ms timesteps
    int currentFuel = 0;
    
    for (int step = 0; step < 1000; step++) {
        // Apply drip torch to current fuel element
        if (currentFuel < fuelIds.size()) {
            torch.applyHeat(simId, fuelIds[currentFuel], dt);
            
            // Check if ignited
            FireElement* element = nullptr;
            if (fire_sim_get_element(simId, fuelIds[currentFuel], &element) == FIRE_SIM_SUCCESS) {
                if (element->ignited) {
                    currentFuel++; // Move to next element
                    std::cout << "Ignited element " << currentFuel << "/" << fuelIds.size() << std::endl;
                }
            }
        }
        
        // Update simulation
        fire_sim_update(simId, dt);
    }
    
    // Get final stats
    SimulationStats stats;
    if (fire_sim_get_stats(simId, &stats) == FIRE_SIM_SUCCESS) {
        std::cout << "Burned " << stats.burning_elements << " elements" << std::endl;
        std::cout << "Total fuel consumed: " << stats.total_fuel_consumed << " kg" << std::endl;
    }
    
    fire_sim_destroy(simId);
}

int main() {
    simulateControlledBurn();
    return 0;
}
```

### C#: Structure Fire Radiant Heat

```csharp
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public class StructureFire
{
    private float firePowerMW;
    private Vector3 position;
    private float radiantFraction;
    
    public StructureFire(Vector3 pos, float powerMW = 15.0f)
    {
        position = pos;
        firePowerMW = powerMW;
        radiantFraction = 0.35f; // 35% radiates
    }
    
    public void ApplyHeatToNearbyFuel(
        UIntPtr simId,
        List<uint> nearbyFuelIds,
        Dictionary<uint, Vector3> fuelPositions,
        float dt
    ) {
        const float PI = 3.14159265f;
        
        foreach (uint fuelId in nearbyFuelIds)
        {
            if (!fuelPositions.ContainsKey(fuelId)) continue;
            
            Vector3 fuelPos = fuelPositions[fuelId];
            float distance = Vector3.Distance(position, fuelPos);
            
            // Skip if too far
            if (distance > 100.0f) continue;
            
            // Calculate radiant power
            float radiantPowerW = firePowerMW * 1e6f * radiantFraction;
            
            // Geometric attenuation (inverse square law)
            float flux = radiantPowerW / (4.0f * PI * distance * distance);
            
            // Assume 0.5 m² fuel element surface area
            float targetArea = 0.5f;
            float heatKj = flux * targetArea * dt * 0.001f;
            
            // Apply heat
            FireSimulation.fire_sim_apply_heat_to_element(simId, fuelId, heatKj, dt);
        }
    }
}

public class WildfireSimulation
{
    [DllImport("fire_sim_ffi")]
    private static extern int fire_sim_apply_heat_to_element(
        UIntPtr simId,
        uint elementId,
        float heatKj,
        float dt
    );
    
    public static void SimulateStructureFire()
    {
        // Create simulation
        UIntPtr simId;
        if (FireSimulation.fire_sim_create(200.0f, 200.0f, 5.0f, 0, out simId) != 0)
        {
            Console.WriteLine("Failed to create simulation");
            return;
        }
        
        // Create fuel elements around structure
        var fuelIds = new List<uint>();
        var fuelPositions = new Dictionary<uint, Vector3>();
        
        for (int i = 0; i < 40; i++)
        {
            for (int j = 0; j < 40; j++)
            {
                float x = 10.0f + i * 4.0f;
                float y = 10.0f + j * 4.0f;
                
                uint elementId;
                if (FireSimulation.fire_sim_add_fuel_element(
                    simId,
                    x, y, 0.5f,
                    2,      // dry_grass
                    0,      // ground vegetation
                    3.0f,   // mass
                    uint.MaxValue, // no parent
                    out elementId
                ) == 0)
                {
                    fuelIds.Add(elementId);
                    fuelPositions[elementId] = new Vector3(x, y, 0.5f);
                }
            }
        }
        
        // Structure fire at center
        var structureFire = new StructureFire(
            new Vector3(100.0f, 100.0f, 5.0f),
            15.0f  // 15 MW house fire
        );
        
        // Simulate for 300 seconds (5 minutes)
        float dt = 0.1f;
        for (int step = 0; step < 3000; step++)
        {
            // Apply radiant heat from structure fire
            structureFire.ApplyHeatToNearbyFuel(simId, fuelIds, fuelPositions, dt);
            
            // Update simulation
            FireSimulation.fire_sim_update(simId, dt);
            
            // Log progress every 10 seconds
            if (step % 100 == 0)
            {
                SimulationStats stats;
                if (FireSimulation.fire_sim_get_stats(simId, out stats) == 0)
                {
                    float timeSeconds = step * dt;
                    Console.WriteLine($"Time: {timeSeconds:F1}s - Burning: {stats.burning_elements} elements");
                }
            }
        }
        
        // Final stats
        SimulationStats finalStats;
        if (FireSimulation.fire_sim_get_stats(simId, out finalStats) == 0)
        {
            Console.WriteLine($"Final fuel consumed: {finalStats.total_fuel_consumed:F2} kg");
            Console.WriteLine($"Burned elements: {finalStats.burning_elements}");
        }
        
        FireSimulation.fire_sim_destroy(simId);
    }
}

public struct Vector3
{
    public float X, Y, Z;
    
    public Vector3(float x, float y, float z)
    {
        X = x; Y = y; Z = z;
    }
    
    public static float Distance(Vector3 a, Vector3 b)
    {
        float dx = a.X - b.X;
        float dy = a.Y - b.Y;
        float dz = a.Z - b.Z;
        return MathF.Sqrt(dx * dx + dy * dy + dz * dz);
    }
}
```

---

## Key Takeaways

1. **Use `apply_heat_to_element` for gradual heating** - Respects moisture physics, realistic for backburns and radiant pre-heating

2. **Calculate heat from physical sources** - Use Stefan-Boltzmann radiation, Byram's intensity, or direct fuel combustion formulas

3. **Consider distance and geometry** - Inverse square law for radiant heat, view factors for geometric relationships

4. **Moisture matters** - Wet fuel (>30% moisture) won't ignite even with sustained heating

5. **Sustained application required** - Unlike `ignite_element`, heating builds up gradually over time

6. **Monitor element state** - Check `element.ignited` and `element.temperature` to track heating progress

---

## References

- **Stefan-Boltzmann Law**: Thermal radiation physics
- **Byram (1959)**: Fireline intensity and flame height equations
- **Thomas (1963)**: Radiant heat transfer from fires
- **Rothermel (1972)**: Heat of pre-ignition calculations
- **Nelson (2000)**: Fuel moisture dynamics
- **Koo et al. (2010)**: Ember ignition probabilities

For more information, see:
- [Unreal Engine Integration Guide](./unreal-engine-integration.md)
- [Scientific Validation Documentation](../validation/SCIENTIFIC_VALIDATION.md)
- [Australian Bushfire Validation](../validation/AUSTRALIAN_BUSHFIRE_VALIDATION_FINDINGS.md)
