# Unreal Engine 5 Integration Guide

## Overview

This guide explains how to integrate the **Australia Fire Simulation** Rust library into Unreal Engine 5 for realistic wildfire visualization and gameplay mechanics.

The fire simulation core is written in Rust for performance and scientific accuracy, with a C-compatible FFI (Foreign Function Interface) layer that allows seamless integration with Unreal Engine's C++ codebase.

## Architecture

```
Unreal Engine 5 (C++)
        ↓
    FFI Layer (C API)
        ↓
Fire Sim Core (Rust)
```

## Step 1: Build the Rust Library

### Prerequisites
- Rust toolchain (1.70+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- C compiler (MSVC on Windows, GCC/Clang on Linux/Mac)

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

```bash
# Install cbindgen if not already installed
cargo install cbindgen

# Generate C header file
cbindgen --config crates/ffi/cbindgen.toml --crate fire-sim-ffi --output FireSimFFI.h
```

## Step 2: Project Setup in Unreal Engine

### 1. Copy Library Files

Create a `ThirdParty/FireSim` directory in your Unreal project:

```
YourProject/
├── Source/
├── ThirdParty/
│   └── FireSim/
│       ├── Include/
│       │   └── FireSimFFI.h
│       ├── Lib/
│       │   ├── Win64/
│       │   │   └── fire_sim_ffi.dll.lib
│       │   ├── Linux/
│       │   │   └── libfire_sim_ffi.so
│       │   └── Mac/
│       │       └── libfire_sim_ffi.dylib
│       └── Binaries/
│           ├── Win64/
│           │   └── fire_sim_ffi.dll
│           ├── Linux/
│           │   └── libfire_sim_ffi.so
│           └── Mac/
│               └── libfire_sim_ffi.dylib
```

### 2. Update Build Configuration

Edit `YourProject.Build.cs`:

```csharp
// YourProject.Build.cs
using System.IO;
using UnrealBuildTool;

public class YourProject : ModuleRules
{
    public YourProject(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
        
        PublicDependencyModuleNames.AddRange(new string[] { 
            "Core", 
            "CoreUObject", 
            "Engine", 
            "InputCore" 
        });
        
        // Add Fire Simulation library
        string ThirdPartyPath = Path.Combine(ModuleDirectory, "../../ThirdParty/FireSim");
        string IncludePath = Path.Combine(ThirdPartyPath, "Include");
        string LibPath = Path.Combine(ThirdPartyPath, "Lib");
        string BinPath = Path.Combine(ThirdPartyPath, "Binaries");
        
        PublicIncludePaths.Add(IncludePath);
        
        if (Target.Platform == UnrealTargetPlatform.Win64)
        {
            PublicAdditionalLibraries.Add(Path.Combine(LibPath, "Win64", "fire_sim_ffi.dll.lib"));
            RuntimeDependencies.Add(Path.Combine(BinPath, "Win64", "fire_sim_ffi.dll"));
        }
        else if (Target.Platform == UnrealTargetPlatform.Linux)
        {
            PublicAdditionalLibraries.Add(Path.Combine(LibPath, "Linux", "libfire_sim_ffi.so"));
            RuntimeDependencies.Add(Path.Combine(BinPath, "Linux", "libfire_sim_ffi.so"));
        }
        else if (Target.Platform == UnrealTargetPlatform.Mac)
        {
            PublicAdditionalLibraries.Add(Path.Combine(LibPath, "Mac", "libfire_sim_ffi.dylib"));
            RuntimeDependencies.Add(Path.Combine(BinPath, "Mac", "libfire_sim_ffi.dylib"));
        }
    }
}
```

## Step 3: Create C++ Wrapper Class

Create `FireSimulationManager.h`:

```cpp
// FireSimulationManager.h
#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "FireSimFFI.h"
#include "FireSimulationManager.generated.h"

USTRUCT(BlueprintType)
struct FBurningElement
{
    GENERATED_BODY()
    
    UPROPERTY(BlueprintReadOnly)
    int32 ID;
    
    UPROPERTY(BlueprintReadOnly)
    FVector Position;
    
    UPROPERTY(BlueprintReadOnly)
    float Temperature;
    
    UPROPERTY(BlueprintReadOnly)
    float FlameHeight;
    
    UPROPERTY(BlueprintReadOnly)
    float Intensity;
};

USTRUCT(BlueprintType)
struct FEmberData
{
    GENERATED_BODY()
    
    UPROPERTY(BlueprintReadOnly)
    int32 ID;
    
    UPROPERTY(BlueprintReadOnly)
    FVector Position;
    
    UPROPERTY(BlueprintReadOnly)
    FVector Velocity;
    
    UPROPERTY(BlueprintReadOnly)
    float Temperature;
};

UCLASS()
class YOURPROJECT_API AFireSimulationManager : public AActor
{
    GENERATED_BODY()
    
public:
    AFireSimulationManager();
    virtual ~AFireSimulationManager();
    
    virtual void BeginPlay() override;
    virtual void Tick(float DeltaTime) override;
    virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;
    
    // Configuration
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Fire Simulation")
    float MapWidth = 1000.0f;
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Fire Simulation")
    float MapHeight = 1000.0f;
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Fire Simulation")
    float MapDepth = 100.0f;
    
    // Weather
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Weather", meta = (ClampMin = "-20", ClampMax = "50"))
    float Temperature = 30.0f;
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Weather", meta = (ClampMin = "0", ClampMax = "100"))
    float Humidity = 30.0f;
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Weather", meta = (ClampMin = "0", ClampMax = "100"))
    float WindSpeed = 30.0f;
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Weather", meta = (ClampMin = "0", ClampMax = "360"))
    float WindDirection = 0.0f;
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Weather", meta = (ClampMin = "0", ClampMax = "10"))
    float DroughtFactor = 5.0f;
    
    // Functions
    UFUNCTION(BlueprintCallable, Category = "Fire Simulation")
    void InitializeSimulation();
    
    UFUNCTION(BlueprintCallable, Category = "Fire Simulation")
    int32 AddFuelElement(FVector Position, uint8 FuelType, uint8 PartType, float Mass, int32 ParentID = -1);
    
    UFUNCTION(BlueprintCallable, Category = "Fire Simulation")
    void IgniteElement(int32 ElementID, float InitialTemp = 600.0f);
    
    UFUNCTION(BlueprintCallable, Category = "Fire Simulation")
    void UpdateWeather(float NewTemp, float NewHumidity, float NewWindSpeed, float NewWindDirection, float NewDrought);
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation")
    TArray<FBurningElement> GetBurningElements();
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation")
    TArray<FEmberData> GetEmbers();
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation")
    int32 GetBurningCount() const { return BurningCount; }
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation")
    int32 GetEmberCount() const { return EmberCount; }
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation")
    float GetFFDI() const { return CurrentFFDI; }
    
private:
    uintptr_t SimulationID = 0;
    int32 BurningCount = 0;
    int32 EmberCount = 0;
    float CurrentFFDI = 0.0f;
};
```

Create `FireSimulationManager.cpp`:

```cpp
// FireSimulationManager.cpp
#include "FireSimulationManager.h"

AFireSimulationManager::AFireSimulationManager()
{
    PrimaryActorTick.bCanEverTick = true;
}

AFireSimulationManager::~AFireSimulationManager()
{
    if (SimulationID != 0)
    {
        fire_sim_destroy(SimulationID);
        SimulationID = 0;
    }
}

void AFireSimulationManager::BeginPlay()
{
    Super::BeginPlay();
    InitializeSimulation();
}

void AFireSimulationManager::Tick(float DeltaTime)
{
    Super::Tick(DeltaTime);
    
    if (SimulationID != 0)
    {
        // Update simulation
        fire_sim_update(SimulationID, DeltaTime);
        
        // Get current stats
        BurningCount = fire_sim_burning_count(SimulationID);
        EmberCount = fire_sim_ember_count(SimulationID);
        
        // Update FFDI (TODO: Add FFI function for this)
        // CurrentFFDI = fire_sim_get_ffdi(SimulationID);
    }
}

void AFireSimulationManager::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
    if (SimulationID != 0)
    {
        fire_sim_destroy(SimulationID);
        SimulationID = 0;
    }
    
    Super::EndPlay(EndPlayReason);
}

void AFireSimulationManager::InitializeSimulation()
{
    if (SimulationID != 0)
    {
        fire_sim_destroy(SimulationID);
    }
    
    // Create simulation
    SimulationID = fire_sim_create(MapWidth, MapHeight, MapDepth);
    
    // Set initial weather
    fire_sim_update_weather(SimulationID, Temperature, Humidity, 
                           WindSpeed * FMath::Sin(FMath::DegreesToRadians(WindDirection)),
                           WindSpeed * FMath::Cos(FMath::DegreesToRadians(WindDirection)),
                           0.0f, DroughtFactor);
    
    UE_LOG(LogTemp, Log, TEXT("Fire Simulation initialized (ID: %llu)"), SimulationID);
}

int32 AFireSimulationManager::AddFuelElement(FVector Position, uint8 FuelType, uint8 PartType, float Mass, int32 ParentID)
{
    if (SimulationID == 0)
    {
        UE_LOG(LogTemp, Error, TEXT("Simulation not initialized"));
        return -1;
    }
    
    // Convert Unreal coordinates (Z-up, cm) to simulation coordinates (Z-up, meters)
    float X = Position.X / 100.0f;
    float Y = Position.Y / 100.0f;
    float Z = Position.Z / 100.0f;
    
    uint32_t ParentU32 = (ParentID >= 0) ? static_cast<uint32_t>(ParentID) : UINT32_MAX;
    
    uint32_t ElementID = fire_sim_add_fuel_element(SimulationID, X, Y, Z, FuelType, PartType, Mass, ParentU32);
    
    return static_cast<int32>(ElementID);
}

void AFireSimulationManager::IgniteElement(int32 ElementID, float InitialTemp)
{
    if (SimulationID == 0 || ElementID < 0)
    {
        return;
    }
    
    fire_sim_ignite_element(SimulationID, static_cast<uint32_t>(ElementID), InitialTemp);
}

void AFireSimulationManager::UpdateWeather(float NewTemp, float NewHumidity, float NewWindSpeed, float NewWindDirection, float NewDrought)
{
    Temperature = NewTemp;
    Humidity = NewHumidity;
    WindSpeed = NewWindSpeed;
    WindDirection = NewWindDirection;
    DroughtFactor = NewDrought;
    
    if (SimulationID != 0)
    {
        fire_sim_update_weather(SimulationID, Temperature, Humidity,
                               WindSpeed * FMath::Sin(FMath::DegreesToRadians(WindDirection)),
                               WindSpeed * FMath::Cos(FMath::DegreesToRadians(WindDirection)),
                               0.0f, DroughtFactor);
    }
}

TArray<FBurningElement> AFireSimulationManager::GetBurningElements()
{
    TArray<FBurningElement> Result;
    
    if (SimulationID == 0)
    {
        return Result;
    }
    
    uint32_t Count = 0;
    const FireElementVisual* Elements = fire_sim_get_burning_elements(SimulationID, &Count);
    
    if (Elements == nullptr || Count == 0)
    {
        return Result;
    }
    
    Result.Reserve(Count);
    
    for (uint32_t i = 0; i < Count; ++i)
    {
        FBurningElement Elem;
        Elem.ID = Elements[i].id;
        // Convert simulation coordinates (meters) back to Unreal coordinates (cm, Z-up)
        Elem.Position = FVector(Elements[i].position[0] * 100.0f,
                               Elements[i].position[1] * 100.0f,
                               Elements[i].position[2] * 100.0f);
        Elem.Temperature = Elements[i].temperature;
        Elem.FlameHeight = Elements[i].flame_height;
        Elem.Intensity = Elements[i].intensity;
        
        Result.Add(Elem);
    }
    
    return Result;
}

TArray<FEmberData> AFireSimulationManager::GetEmbers()
{
    TArray<FEmberData> Result;
    
    if (SimulationID == 0)
    {
        return Result;
    }
    
    uint32_t Count = 0;
    const EmberVisual* Embers = fire_sim_get_embers(SimulationID, &Count);
    
    if (Embers == nullptr || Count == 0)
    {
        return Result;
    }
    
    Result.Reserve(Count);
    
    for (uint32_t i = 0; i < Count; ++i)
    {
        FEmberData Ember;
        Ember.ID = Embers[i].id;
        // Convert to Unreal coordinates
        Ember.Position = FVector(Embers[i].position[0] * 100.0f,
                                Embers[i].position[1] * 100.0f,
                                Embers[i].position[2] * 100.0f);
        Ember.Velocity = FVector(Embers[i].velocity[0] * 100.0f,
                                Embers[i].velocity[1] * 100.0f,
                                Embers[i].velocity[2] * 100.0f);
        Ember.Temperature = Embers[i].temperature;
        
        Result.Add(Ember);
    }
    
    return Result;
}
```

## Step 4: Blueprint Integration

### 1. Create Blueprint Actor

1. In Unreal Editor, create a new Blueprint Class based on `AFireSimulationManager`
2. Name it `BP_FireSimulation`
3. Add it to your level

### 2. Visualization

Create particle systems and materials for:
- **Flames**: Use Niagara particle system with temperature-based color gradient
- **Smoke**: Large particles with alpha fade and turbulence
- **Embers**: Glowing particles with physics and wind response
- **PyroCb Clouds**: Volumetric clouds with lightning effects

### 3. Example Blueprint Graph

```
Event Tick
  ├─> Get Burning Elements
  │     └─> For Each Element
  │           └─> Spawn Flame Particle at Position (scaled by Intensity)
  │
  ├─> Get Embers
  │     └─> For Each Ember
  │           └─> Spawn Ember Particle at Position with Velocity
  │
  └─> Update UI with FFDI and Burning Count
```

## Step 5: Performance Optimization

### Multi-Threading

The Rust simulation is already multi-threaded (using Rayon), but ensure Unreal integration doesn't block the game thread:

```cpp
// Use Async Task for large queries
Async(EAsyncExecution::ThreadPool, [this]()
{
    TArray<FBurningElement> Elements = GetBurningElements();
    
    // Update on game thread
    AsyncTask(ENamedThreads::GameThread, [this, Elements]()
    {
        UpdateVisualization(Elements);
    });
});
```

### LOD System

Implement Level of Detail for fire visualization:
- **Close**: Full particle effects per element
- **Medium**: Clustered particle systems
- **Far**: Single billboard with animated texture

### Spatial Culling

Only update visible fire elements:

```cpp
for (const FBurningElement& Elem : BurningElements)
{
    if (IsLocationInFrustum(Elem.Position))
    {
        UpdateFireVisuals(Elem);
    }
}
```

## Step 6: Example Scenarios

### Grassfire Spread

```cpp
// Spawn grass grid
for (int X = -500; X < 500; X += 200)
{
    for (int Y = -500; Y < 500; Y += 200)
    {
        FVector Pos(X, Y, 0);
        int32 ID = FireSimManager->AddFuelElement(Pos, 2, 10, 0.5f); // Dry grass
    }
}

// Ignite center
FireSimManager->IgniteElement(CenterElementID, 600.0f);
```

### Eucalyptus Forest

```cpp
// Plant trees
for (int i = 0; i < 20; ++i)
{
    FVector TreePos = GetRandomForestLocation();
    
    // Trunk
    int32 TrunkID = FireSimManager->AddFuelElement(TreePos, 0, 1, 10.0f);
    
    // Crown
    FVector CrownPos = TreePos + FVector(0, 0, 1500); // 15m up
    FireSimManager->AddFuelElement(CrownPos, 0, 6, 5.0f, TrunkID);
}
```

## Fuel Type Reference

| ID | Fuel Type | Description |
|----|-----------|-------------|
| 0 | Eucalyptus Stringybark | Extreme fire behavior, 25km spotting |
| 1 | Eucalyptus Smooth Bark | Moderate behavior, 10km spotting |
| 2 | Dry Grass | Fast ignition, rapid spread |
| 3 | Shrubland | Medium ignition |
| 4 | Dead Wood/Litter | High susceptibility |
| 5 | Green Vegetation | Fire resistant |
| 6 | Water | Non-burnable, blocks heat |
| 7 | Rock | Non-burnable, reduces heat |

## Part Type Reference

| ID | Part Type | Description |
|----|-----------|-------------|
| 0 | Root | Underground/base |
| 1 | TrunkLower | Lower trunk |
| 2 | TrunkMiddle | Middle trunk |
| 3 | TrunkUpper | Upper trunk |
| 4 | BarkLayer | Bark on trunk |
| 5 | Branch | Branch with leaves |
| 6 | Crown | Canopy foliage |
| 10 | GroundLitter | Dead material on ground |
| 11 | GroundVegetation | Living ground cover |
| 20 | Surface | Non-flammable surface |

## Troubleshooting

### Library Not Found

**Windows**: Ensure `fire_sim_ffi.dll` is in `Binaries/Win64/`

**Linux**: Set `LD_LIBRARY_PATH`:
```bash
export LD_LIBRARY_PATH=/path/to/ThirdParty/FireSim/Binaries/Linux:$LD_LIBRARY_PATH
```

### Linker Errors

Verify Build.cs includes correct library paths and platform checks.

### Simulation Crashes

- Check that `SimulationID` is valid (non-zero)
- Ensure `fire_sim_destroy()` is called in destructor
- Verify element IDs are valid before ignition

### Performance Issues

- Reduce update frequency (e.g., 10 Hz instead of 60 Hz)
- Limit visualization to visible elements
- Use spatial culling and LOD
- Consider running simulation on separate thread

## Advanced Topics

### Custom Weather Presets

Create regional weather by calling FFI functions to set monthly temperatures, humidity, etc.

### PyroCb Lightning Events

Monitor `fire_sim_get_pyrocb_events()` to trigger lightning visual effects and new ignitions.

### Save/Load Simulation State

Serialize simulation state for checkpoints (requires additional FFI functions).

### Networked Multiplayer

Replicate fire state across clients using Unreal's replication system.

## Resources

- [Rust FFI Documentation](https://doc.rust-lang.org/nomicon/ffi.html)
- [Unreal Engine C++ Programming Guide](https://docs.unrealengine.com/5.0/en-US/programming-with-cplusplus-in-unreal-engine/)
- [Unreal Engine Third-Party Libraries](https://docs.unrealengine.com/5.0/en-US/integrating-third-party-libraries-into-unreal-engine/)
- [Australian Fire Behavior Research](https://www.csiro.au/en/research/natural-disasters/bushfires)

## Support

For issues or questions:
- GitHub Issues: https://github.com/mineubob/Australia-Fire-Sim/issues
- Documentation: https://github.com/mineubob/Australia-Fire-Sim/docs

---

**Note**: This simulation is designed for scientific accuracy and emergency response training. Fire behavior is based on real-world physics and Australian bushfire research.
