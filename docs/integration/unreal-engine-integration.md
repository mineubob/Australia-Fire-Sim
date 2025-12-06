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

The C header is **automatically generated** during the build process via `build.rs`. Simply building the FFI crate will generate `FireSimFFI.h` in the repository root:

```bash
# Build the FFI library - this automatically generates FireSimFFI.h
cargo build --release -p fire-sim-ffi

# The header is now available at: FireSimFFI.h (repo root)
```

The generated header contains:
- **Lifecycle functions**: `fire_sim_create()`, `fire_sim_destroy()`, `fire_sim_update()`
- **Fuel element functions**: `fire_sim_add_fuel()`, `fire_sim_ignite()`, `fire_sim_get_element_state()`
- **Query functions**: `fire_sim_get_burning_elements()`, `fire_sim_get_stats()`, `fire_sim_get_snapshot()`
- **Suppression functions**: `fire_sim_apply_suppression_to_elements()`, `fire_sim_apply_water_direct()`
- **Terrain functions**: `fire_sim_terrain_elevation()`, `fire_sim_terrain_slope()`, `fire_sim_terrain_aspect()`
- **Weather functions**: `fire_sim_set_weather_from_live()`, `fire_sim_get_haines_index()`
- **Multiplayer functions**: `fire_sim_submit_player_action()`, `fire_sim_get_frame_number()`

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
    float FuelRemaining;
    
    UPROPERTY(BlueprintReadOnly)
    float MoistureFraction;
    
    UPROPERTY(BlueprintReadOnly)
    float FlameHeight;
    
    UPROPERTY(BlueprintReadOnly)
    bool bIsCrownFire;
    
    UPROPERTY(BlueprintReadOnly)
    bool bHasSuppression;
    
    UPROPERTY(BlueprintReadOnly)
    float SuppressionEffectiveness;
};

USTRUCT(BlueprintType)
struct FSimulationSnapshot
{
    GENERATED_BODY()
    
    UPROPERTY(BlueprintReadOnly)
    uint32 FrameNumber;
    
    UPROPERTY(BlueprintReadOnly)
    float SimulationTime;
    
    UPROPERTY(BlueprintReadOnly)
    uint32 BurningElementCount;
    
    UPROPERTY(BlueprintReadOnly)
    float TotalFuelConsumed;
    
    UPROPERTY(BlueprintReadOnly)
    uint32 ActiveEmberCount;
    
    UPROPERTY(BlueprintReadOnly)
    uint32 PyrocumulusCount;
    
    UPROPERTY(BlueprintReadOnly)
    uint8 HainesIndex;
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
    
    // Simulation Configuration
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Fire Simulation|Setup")
    float MapWidth = 1000.0f;
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Fire Simulation|Setup")
    float MapHeight = 1000.0f;
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Fire Simulation|Setup", meta = (ClampMin = "1", ClampMax = "10"))
    float GridCellSize = 2.0f;  // Resolution in meters (2-5m recommended)
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Fire Simulation|Setup")
    uint8 TerrainType = 0;  // 0=flat, 1=single_hill, 2=valley
    
    // Weather
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Fire Simulation|Weather", meta = (ClampMin = "-20", ClampMax = "50"))
    float Temperature = 30.0f;
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Fire Simulation|Weather", meta = (ClampMin = "0", ClampMax = "100"))
    float Humidity = 30.0f;
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Fire Simulation|Weather", meta = (ClampMin = "0", ClampMax = "100"))
    float WindSpeed = 30.0f;
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Fire Simulation|Weather", meta = (ClampMin = "0", ClampMax = "360"))
    float WindDirection = 0.0f;
    
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Fire Simulation|Weather", meta = (ClampMin = "0", ClampMax = "10"))
    float DroughtFactor = 5.0f;
    
    // Core Functions
    UFUNCTION(BlueprintCallable, Category = "Fire Simulation")
    void InitializeSimulation();
    
    UFUNCTION(BlueprintCallable, Category = "Fire Simulation|Fuel")
    int32 AddFuelElement(FVector Position, uint8 FuelType, uint8 PartType, float Mass, int32 ParentID = -1);
    
    UFUNCTION(BlueprintCallable, Category = "Fire Simulation|Fire")
    void IgniteElement(int32 ElementID, float InitialTemp = 600.0f);
    
    UFUNCTION(BlueprintCallable, Category = "Fire Simulation|Weather")
    void UpdateWeather(float NewTemp, float NewHumidity, float NewWindSpeed, float NewWindDirection, float NewDrought);
    
    // Query Functions
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation|Query")
    TArray<FBurningElement> GetBurningElements();
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation|Query")
    FSimulationSnapshot GetSnapshot();
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation|Query")
    int32 GetBurningCount() const { return BurningCount; }
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation|Query")
    int32 GetEmberCount() const { return EmberCount; }
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation|Query")
    uint8 GetHainesIndex() const { return HainesIndex; }
    
    // Suppression Functions
    UFUNCTION(BlueprintCallable, Category = "Fire Simulation|Suppression")
    int32 ApplySuppressionToElements(FVector Position, float Radius, float MassPerElement, uint8 AgentType);
    
    UFUNCTION(BlueprintCallable, Category = "Fire Simulation|Suppression")
    void ApplyWaterDirect(FVector Position, float Mass);
    
    // Terrain Query Functions
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation|Terrain")
    float GetElevation(FVector Position);
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation|Terrain")
    float GetSlope(FVector Position);
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation|Terrain")
    float GetAspect(FVector Position);
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation|Terrain")
    float GetSlopeSpreadMultiplier(FVector FromPos, FVector ToPos);
    
    // Multiplayer Functions
    UFUNCTION(BlueprintCallable, Category = "Fire Simulation|Multiplayer")
    bool SubmitPlayerAction(uint8 ActionType, uint32 PlayerID, FVector ActionPos, float Param1, uint32 Param2);
    
    UFUNCTION(BlueprintCallable, BlueprintPure, Category = "Fire Simulation|Multiplayer")
    uint32 GetFrameNumber() const;
    
private:
    uintptr_t SimulationID = 0;
    int32 BurningCount = 0;
    int32 EmberCount = 0;
    uint8 HainesIndex = 0;
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
        // Update simulation with timestep
        fire_sim_update(SimulationID, DeltaTime);
        
        // Get current stats
        uint32_t BurningOut = 0, TotalOut = 0, ActiveCellsOut = 0, TotalCellsOut = 0;
        float FuelConsumedOut = 0.0f;
        
        fire_sim_get_stats(SimulationID, &BurningOut, &TotalOut, &ActiveCellsOut, &TotalCellsOut, &FuelConsumedOut);
        BurningCount = BurningOut;
        EmberCount = 0; // Query separately if needed
        
        // Get Haines Index
        HainesIndex = fire_sim_get_haines_index(SimulationID);
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
    
    // Create simulation with new API
    int32 CreateResult = fire_sim_create(MapWidth, MapHeight, GridCellSize, TerrainType, (uintptr_t*)&SimulationID);
    
    if (CreateResult != FIRE_SIM_SUCCESS)
    {
        UE_LOG(LogTemp, Error, TEXT("Failed to create fire simulation (error: %d)"), CreateResult);
        return;
    }
    
    // Set initial weather
    fire_sim_set_weather_from_live(SimulationID, Temperature, Humidity, 
                                  WindSpeed, WindDirection, 1013.25f, DroughtFactor, 0.0f);
    
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
    
    int32_t ParentID_C = (ParentID >= 0) ? ParentID : -1;
    uint32_t ElementID = 0;
    
    int32 Result = fire_sim_add_fuel(SimulationID, X, Y, Z, FuelType, PartType, Mass, ParentID_C, &ElementID);
    
    if (Result != FIRE_SIM_SUCCESS)
    {
        UE_LOG(LogTemp, Warning, TEXT("Failed to add fuel element (error: %d)"), Result);
        return -1;
    }
    
    return (int32)ElementID;
}

void AFireSimulationManager::IgniteElement(int32 ElementID, float InitialTemp)
{
    if (SimulationID == 0 || ElementID < 0)
    {
        return;
    }
    
    int32 Result = fire_sim_ignite(SimulationID, (uint32_t)ElementID, InitialTemp);
    
    if (Result != FIRE_SIM_SUCCESS)
    {
        UE_LOG(LogTemp, Warning, TEXT("Failed to ignite element %d (error: %d)"), ElementID, Result);
    }
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
        int32 Result = fire_sim_set_weather_from_live(SimulationID, Temperature, Humidity,
                                                      WindSpeed, WindDirection, 1013.25f, DroughtFactor, 0.0f);
        
        if (Result != FIRE_SIM_SUCCESS)
        {
            UE_LOG(LogTemp, Warning, TEXT("Failed to update weather (error: %d)"), Result);
        }
    }
}

TArray<FBurningElement> AFireSimulationManager::GetBurningElements()
{
    TArray<FBurningElement> Result;
    
    if (SimulationID == 0)
    {
        return Result;
    }
    
    // Pre-allocate array for max expected elements
    const uint32 MaxElements = 1000;
    ElementFireState* States = new ElementFireState[MaxElements];
    uint32 Count = 0;
    
    int32 QueryResult = fire_sim_get_burning_elements(SimulationID, States, MaxElements, &Count);
    
    if (QueryResult != FIRE_SIM_SUCCESS)
    {
        delete[] States;
        return Result;
    }
    
    Result.Reserve(Count);
    
    for (uint32 i = 0; i < Count; ++i)
    {
        FBurningElement Elem;
        Elem.ID = States[i].element_id;
        Elem.Temperature = States[i].temperature;
        Elem.FuelRemaining = States[i].fuel_remaining;
        Elem.MoistureFraction = States[i].moisture_fraction;
        Elem.FlameHeight = States[i].flame_height;
        Elem.bIsCrownFire = States[i].is_crown_fire;
        Elem.bHasSuppression = States[i].has_suppression;
        Elem.SuppressionEffectiveness = States[i].suppression_effectiveness;
        
        Result.Add(Elem);
    }
    
    delete[] States;
    return Result;
}

FSimulationSnapshot AFireSimulationManager::GetSnapshot()
{
    FSimulationSnapshot Snapshot;
    FMemory::Memzero(&Snapshot, sizeof(FSimulationSnapshot));
    
    if (SimulationID != 0)
    {
        SimulationSnapshot SnapC;
        int32 Result = fire_sim_get_snapshot(SimulationID, &SnapC);
        
        if (Result == FIRE_SIM_SUCCESS)
        {
            Snapshot.FrameNumber = SnapC.frame_number;
            Snapshot.SimulationTime = SnapC.simulation_time;
            Snapshot.BurningElementCount = SnapC.burning_element_count;
            Snapshot.TotalFuelConsumed = SnapC.total_fuel_consumed;
            Snapshot.ActiveEmberCount = SnapC.active_ember_count;
            Snapshot.PyrocumulusCount = SnapC.pyrocumulus_count;
            Snapshot.HainesIndex = SnapC.haines_index;
        }
    }
    
    return Snapshot;
}

int32 AFireSimulationManager::ApplySuppressionToElements(FVector Position, float Radius, float MassPerElement, uint8 AgentType)
{
    if (SimulationID == 0)
    {
        return -1;
    }
    
    uint32 Count = 0;
    int32 Result = fire_sim_apply_suppression_to_elements(SimulationID, 
                                                         Position.X / 100.0f, Position.Y / 100.0f, Position.Z / 100.0f,
                                                         Radius / 100.0f, MassPerElement, AgentType, &Count);
    
    if (Result != FIRE_SIM_SUCCESS)
    {
        UE_LOG(LogTemp, Warning, TEXT("Failed to apply suppression (error: %d)"), Result);
        return -1;
    }
    
    return (int32)Count;
}

void AFireSimulationManager::ApplyWaterDirect(FVector Position, float Mass)
{
    if (SimulationID == 0)
    {
        return;
    }
    
    int32 Result = fire_sim_apply_water_direct(SimulationID, 
                                              Position.X / 100.0f, Position.Y / 100.0f, Position.Z / 100.0f, 
                                              Mass);
    
    if (Result != FIRE_SIM_SUCCESS)
    {
        UE_LOG(LogTemp, Warning, TEXT("Failed to apply water (error: %d)"), Result);
    }
}

float AFireSimulationManager::GetElevation(FVector Position)
{
    if (SimulationID == 0) return 0.0f;
    
    float Elevation = 0.0f;
    int32 Result = fire_sim_get_elevation(SimulationID, Position.X / 100.0f, Position.Y / 100.0f, &Elevation);
    
    return (Result == FIRE_SIM_SUCCESS) ? Elevation * 100.0f : 0.0f;  // Convert back to cm
}

float AFireSimulationManager::GetSlope(FVector Position)
{
    if (SimulationID == 0) return 0.0f;
    
    return fire_sim_terrain_slope(SimulationID, Position.X / 100.0f, Position.Y / 100.0f);
}

float AFireSimulationManager::GetAspect(FVector Position)
{
    if (SimulationID == 0) return 0.0f;
    
    return fire_sim_terrain_aspect(SimulationID, Position.X / 100.0f, Position.Y / 100.0f);
}

float AFireSimulationManager::GetSlopeSpreadMultiplier(FVector FromPos, FVector ToPos)
{
    if (SimulationID == 0) return 1.0f;
    
    return fire_sim_terrain_slope_multiplier(SimulationID, 
                                            FromPos.X / 100.0f, FromPos.Y / 100.0f,
                                            ToPos.X / 100.0f, ToPos.Y / 100.0f);
}

bool AFireSimulationManager::SubmitPlayerAction(uint8 ActionType, uint32 PlayerID, FVector ActionPos, float Param1, uint32 Param2)
{
    if (SimulationID == 0)
    {
        return false;
    }
    
    CPlayerAction Action;
    Action.action_type = ActionType;
    Action.player_id = PlayerID;
    Action.timestamp = GetWorld()->GetTimeSeconds();
    Action.position_x = ActionPos.X / 100.0f;
    Action.position_y = ActionPos.Y / 100.0f;
    Action.position_z = ActionPos.Z / 100.0f;
    Action.param1 = Param1;
    Action.param2 = Param2;
    
    return fire_sim_submit_player_action(SimulationID, &Action);
}

uint32 AFireSimulationManager::GetFrameNumber() const
{
    if (SimulationID == 0) return 0;
    
    return fire_sim_get_frame_number(SimulationID);
}
```

## Step 4: Error Handling

The FFI provides consistent error codes for all functions:

```cpp
// Error codes returned by FFI functions
#define FIRE_SIM_SUCCESS           0     // Operation successful
#define FIRE_SIM_INVALID_ID       -1     // Invalid simulation ID
#define FIRE_SIM_NULL_POINTER     -2     // Null pointer passed
#define FIRE_SIM_INVALID_FUEL     -3     // Invalid fuel/agent type
#define FIRE_SIM_INVALID_TERRAIN  -4     // Invalid terrain type
#define FIRE_SIM_LOCK_ERROR       -5     // Internal lock error
```

Always check return values from FFI calls:

```cpp
int32 Result = fire_sim_add_fuel(SimulationID, X, Y, Z, FuelType, PartType, Mass, ParentID, &ElementID);

if (Result != FIRE_SIM_SUCCESS)
{
    switch (Result)
    {
        case FIRE_SIM_INVALID_ID:
            UE_LOG(LogTemp, Error, TEXT("Invalid simulation ID"));
            break;
        case FIRE_SIM_INVALID_FUEL:
            UE_LOG(LogTemp, Error, TEXT("Invalid fuel type: %d"), FuelType);
            break;
        case FIRE_SIM_NULL_POINTER:
            UE_LOG(LogTemp, Error, TEXT("Null pointer error"));
            break;
        default:
            UE_LOG(LogTemp, Error, TEXT("Unknown error: %d"), Result);
    }
    return -1;
}
```

### 1. Create Blueprint Actor

1. In Unreal Editor, create a new Blueprint Class based on `AFireSimulationManager`
2. Name it `BP_FireSimulation`
3. Add it to your level

### 2. Visualization

Create particle systems and materials for:
- **Flames**: Use Niagara particle system with temperature-based color gradient (scale with FlameHeight)
- **Smoke**: Large particles with alpha fade and turbulence (scale with fuel consumed)
- **Embers**: Use high-quality particles for visible fire spots (deprecated: query burning elements for positions instead)
- **PyroCb Clouds**: Volumetric clouds when PyrocumulusCount > 0

### 3. Example Blueprint Graph

```
Event Tick
  ├─> Get Snapshot
  │     └─> Update HUD with FrameNumber, BurningCount, TotalFuelConsumed
  │
  ├─> Get Burning Elements
  │     └─> For Each Element
  │           ├─> Spawn Flame Particle at Element Position (scaled by FlameHeight * Temperature)
  │           ├─> If bIsCrownFire: Activate crown fire VFX
  │           └─> If bHasSuppression: Overlay suppression coverage effect
  │
  └─> Update UI with Haines Index
        └─> Display fire weather severity (2-6 scale)
```

### 4. Multiplayer Action Example

```cpp
// Helicopter water drop at position
bool bSuccess = FireSimManager->ApplySuppressionToElements(
    HelicopterPos,
    500.0f,  // 500cm radius (5 meters)
    50000.0f, // 50kg per element
    0        // Water agent type
);

// Submit for network replication
CPlayerAction DropAction;
DropAction.action_type = 1; // Water drop
DropAction.player_id = LocalPlayerID;
DropAction.position_x = HelicopterPos.X / 100.0f;
DropAction.position_y = HelicopterPos.Y / 100.0f;
DropAction.position_z = HelicopterPos.Z / 100.0f;
DropAction.param1 = 50000.0f; // Total mass

fire_sim_submit_player_action(SimulationID, &DropAction);
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
