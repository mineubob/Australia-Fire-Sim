# Unified GPU Fire Simulation System

**Status:** Not Started  
**Priority:** High  
**Target:** Complete rewrite of fire simulation with unified GPU-accelerated field-based physics

---

## Phases Overview

This task is divided into **6 phases** designed for incremental implementation by GitHub Coding Agent. Each phase is self-contained with clear deliverables and validation criteria.

| Phase | Name | Complexity | Est. Days | Dependency |
|-------|------|------------|-----------|------------|
| **1** | Core Infrastructure & Field Solver | High | 5-7 | None |
| **2** | Heat Transfer & Combustion Physics | High | 6-8 | Phase 1 |
| **3** | Level Set Fire Front Tracking | High | 5-7 | Phase 2 |
| **4** | Simulation Integration & Ignition | High | 5-7 | Phase 3 |
| **5** | Ember, Weather & Wind Coupling | Medium | 4-5 | Phase 4 |
| **6** | FFI, Performance & Validation | Medium | 4-6 | Phase 5 |

**Total Estimated Time: 29-40 days**

---

## ⚠️ IMPORTANT: Build Errors During Implementation

**Expect build failures until Phase 4 is complete.**

This task replaces core simulation code. During Phases 1-3:
- The new `solver/` module will compile independently
- The old `simulation/mod.rs` will still reference deleted types
- **This is expected** - the old code is deleted in Phase 4

### Per-Phase Build Expectations

| Phase | Build Status | Notes |
|-------|--------------|-------|
| 1 | ⚠️ Partial | New `solver/` compiles, old simulation has errors |
| 2 | ⚠️ Partial | Same as Phase 1 |
| 3 | ⚠️ Partial | Same as Phase 1 |
| 4 | ✅ Full | Old code deleted, new simulation wired up |
| 5 | ✅ Full | Should compile and run |
| 6 | ✅ Full | Final validation |

### Guidance for AI Agent

During Phases 1-3:
- **IGNORE** clippy/build errors in `simulation/mod.rs` and files that reference `FuelElement`
- **FOCUS** on making the new `solver/` module compile cleanly
- **DO NOT** try to fix the old code - it will be deleted in Phase 4
- Run `cargo build -p fire-sim-core --lib` to test just the core library
- Run `cargo check -p fire-sim-core` for faster feedback

After Phase 4:
- Full `cargo clippy --all-targets --all-features` must pass
- All tests must pass

### ⚠️ GPU Feature Flag (IMPORTANT FOR CODING AGENT)

The `gpu` feature is **ON by default** but can be disabled for environments without GPU access.

**GitHub Coding Agent does NOT have GPU access.** When running tests:

```bash
# WITHOUT GPU (use this in GitHub Coding Agent / CI):
cargo test --no-default-features
cargo clippy --all-targets --no-default-features
cargo build --no-default-features

# WITH GPU (use this locally if GPU available):
cargo test --all-features
cargo clippy --all-targets --all-features
cargo build --all-features
```

**Expected Behavior:**
- `--no-default-features` → CPU-only backend, all GPU code compiled out
- `--all-features` → GPU code compiled, GPU tests may fail if no GPU available

**GPU-Specific Tests:**
Mark GPU-specific tests with `#[cfg(feature = "gpu")]` so they compile out:

```rust
#[cfg(feature = "gpu")]
#[test]
fn test_gpu_context_creation() {
    // This test only runs with `--all-features`
}
```

**For the GitHub Coding Agent:** Always use `--no-default-features` for validation. GPU tests will be validated locally by the repository maintainer.

---

## Per-Phase Validation Requirements

### For GitHub Coding Agent (No GPU)

| Phase | Validation Command | Expected Result |
|-------|-------------------|-----------------|
| 1-3 | `cargo check -p fire-sim-core --no-default-features 2>&1 \| grep -v "element\|FuelElement\|SpatialIndex"` | New solver/ compiles |
| 1-3 | `cargo test -p fire-sim-core --no-default-features solver::` | New solver tests pass |
| 4+ | `cargo clippy --all-targets --no-default-features` | Zero warnings |
| 4+ | `cargo test --no-default-features` | All CPU tests pass |
| 4+ | `cargo fmt --all --check` | Formatting OK |

### For Local Development (With GPU)

| Phase | Validation Command | Expected Result |
|-------|-------------------|-----------------|
| 1-3 | `cargo check -p fire-sim-core 2>&1 \| grep -v "element\|FuelElement\|SpatialIndex"` | New solver/ compiles |
| 1-3 | `cargo test -p fire-sim-core solver::` | New solver tests pass |
| 4+ | `cargo clippy --all-targets --all-features` | Zero warnings |
| 4+ | `cargo test --all-features` | All tests pass (incl. GPU) |
| 4+ | `cargo fmt --all --check` | Formatting OK |

---

## Table of Contents

1. [Overview](#overview) - Problem statement and solution approach
2. [Critical Architecture Rules](#critical-architecture-rules-never-violate) - NEVER VIOLATE
3. [Scientific Foundation](#scientific-foundation) - Physics models and formulas
4. [Architecture Overview](#architecture-overview) - System design
5. **Phase Implementation Details** (in order):
   - [Phase 1: Core Infrastructure](#phase-1-core-infrastructure--field-solver)
   - [Phase 2: Heat Physics](#phase-2-heat-transfer--combustion-physics)
   - [Phase 3: Level Set](#phase-3-level-set-fire-front-tracking)
   - [Phase 4: Simulation](#phase-4-simulation-integration--ignition)
   - [Phase 5: Ember & Weather](#phase-5-ember-weather--wind-coupling)
   - [Phase 6: FFI & Validation](#phase-6-ffi-performance--validation)
6. **Detailed Step Implementation** (reference during phases):
   - [Step 1: GPU Infrastructure](#step-1-gpu-infrastructure-and-field-textures-phase-1)
   - [Step 2: Heat Transfer](#step-2-gpu-heat-transfer-compute-shader-phase-2)
   - [Step 3: Combustion](#step-3-gpu-combustion-and-fuel-consumption-phase-2)
   - [Step 4: Level Set](#step-4-level-set-evolution-and-curvature-phase-3)
   - [Step 5: FireSimulation](#step-5-new-firesimulation-core-structure-phase-4)
   - [Step 6: Fire Front](#step-6-fire-front-extraction-and-visualization-phase-4)
   - [Step 7: Ember Integration](#step-7-ember-spotting-integration-phase-5)
   - [Step 8: Performance](#step-8-performance-and-optimization-phase-6)
   - [Step 9: FFI](#step-9-ffi-and-external-integration-phase-6)
   - [Step 10: Validation](#step-10-validation-and-testing-phase-6)
7. [Code to Delete](#code-to-delete-breaking-changes) - Files to remove
8. [Success Criteria](#success-criteria) - Must Have / Should Have / Nice to Have
9. [Dependencies](#dependencies) - Cargo.toml changes
10. [References](#references) - Scientific literature
11. [Follow-Up Enhancements](#follow-up-enhancements-separate-tasks) - Separate task files
12. [Completion Checklist](#completion-checklist) - Tracking per-phase completion

---

## Overview

**⚠️ THIS IS A COMPLETE REPLACEMENT - NOT AN ADDITION ⚠️**

This task **completely replaces** the current fire simulation implementation. The old element-based system will be **deleted** and replaced with a unified field-based solver. There is no compatibility layer or dual-mode operation.

**What gets deleted:**
- `crates/core/src/physics/element_heat_transfer.rs` (entire file)
- `crates/core/src/core_types/element.rs` (entire file)  
- `crates/core/src/core_types/spatial.rs` (entire file)
- All `FuelElement`, `SpatialIndex`, and element-based code in `simulation/mod.rs`

**What gets created:**
- `crates/core/src/solver/` (new module with CPU + GPU backends)
- Completely rewritten `crates/core/src/simulation/mod.rs`

---

### Problem with Current System

The current fire simulation uses discrete fuel elements with element-to-element heat transfer. This approach has fundamental limitations:

1. **O(n×k) computational complexity** - spatial queries for every burning element
2. **Artificial separation** - heat transfer and fire spread are computed by different systems
3. **Smooth fire fronts** - element positions create uniform, unrealistic perimeters
4. **CPU-bound** - even with Rayon, can't match GPU parallelism

**Real fire perimeters have:**
- Jagged, irregular boundaries with convex and concave sections
- Fingers extending ahead of the main front (wind channels, dry fuel pockets)
- Indentations where fire encounters resistant fuel (wet areas, rock outcrops)
- Complex multi-lobed shapes from terrain/wind interactions

This task implements a **unified GPU field-based simulation** where:
- **Temperature, fuel, and moisture are continuous 2D fields** (not discrete elements)
- **Heat transfer is computed on GPU** using Stefan-Boltzmann radiation + convection + diffusion
- **Level Set tracks the fire front** with curvature-dependent spread
- **Ignition emerges from physics** - cells ignite when temperature exceeds threshold
- **All systems share the same grid** - perfect data locality for GPU

---

## Critical Architecture Rules (NEVER VIOLATE)

- [ ] **NEVER SIMPLIFY PHYSICS** - Implement formulas exactly as published in fire science literature
- [ ] **NEVER HARDCODE DYNAMIC VALUES** - Use fuel properties, weather conditions, grid state appropriately
- [ ] **FIX ROOT CAUSES, NOT SYMPTOMS** - Investigate WHY invalid values occur, don't clamp/mask
- [ ] **PUBLIC CODE MUST BE COMMENTED** - All public APIs need documentation
- [ ] **NO ALLOW MACROS** - Fix ALL clippy warnings by changing code (workspace denies warnings)
- [ ] **Validate with `cargo clippy --all-targets --all-features` and `cargo fmt --all`** before marking complete

---

## Scientific Foundation

### Unified Field-Based Fire Physics

Instead of discrete elements exchanging heat, we model continuous fields on a 2D grid:

```
Fields (all 2D textures on GPU):
├── T(x,y,t)  - Temperature field (Kelvin)
├── F(x,y,t)  - Fuel load field (kg/m²)
├── M(x,y,t)  - Moisture content field (fraction 0-1)
├── φ(x,y,t)  - Level set signed distance (m)
├── R(x,y,t)  - Spread rate field (m/s)
└── O(x,y,t)  - Oxygen availability field (fraction)
```

### Heat Transfer on GPU (Stefan-Boltzmann + Convection + Diffusion)

The temperature field evolves according to the heat equation with combustion source:

```
∂T/∂t = α∇²T + Q_combustion - Q_radiation - Q_convection + Q_wind_advection

Where:
- α∇²T: Thermal diffusion (conduction through fuel bed)
- Q_combustion: Heat release from burning fuel = ḿ × H × η
- Q_radiation: Stefan-Boltzmann radiative losses = εσ(T⁴ - T_amb⁴)
- Q_convection: Convective heat transfer to atmosphere
- Q_wind_advection: Wind-driven heat transport (critical for fire spread)
```

This is computed entirely on GPU with a single compute shader pass per timestep.

### Level Set Method (Sethian 1999)

The fire front is tracked as the zero-level contour of signed distance function φ:

```
∂φ/∂t + R(x,y,t)|∇φ| = 0

Where:
- φ < 0: Inside fire (burning/burned)
- φ > 0: Outside fire (unburned)
- φ = 0: Fire front (perimeter)
- R(x,y,t): Local spread rate derived from temperature gradient
```

### Curvature-Dependent Spread (Margerit & Séro-Guillaume 2002)

This is the **key mechanism** that produces realistic jagged fire perimeters like the Boddington fire (see reference image):

```
R_effective = R_base × (1 + κ_coeff × κ)

Where:
- κ > 0 (convex): Fire finger advances faster (positive feedback)
- κ < 0 (concave): Fire slows at indentations (negative feedback)
- κ_coeff ≈ 0.25 per literature
```

**Why this creates realistic fire shapes:**

1. **Fingers form naturally**: Any small perturbation in the fire front that extends outward becomes convex (κ > 0). Convex regions spread faster, so the perturbation grows into a "finger" - exactly like the lobes in the Boddington fire.

2. **Indentations persist**: Concave regions (κ < 0) spread slower, so gaps in the fire line don't fill in quickly. This creates the irregular "bays" and indentations seen in real fire perimeters.

3. **Wind amplifies asymmetry**: The head fire (downwind) develops more pronounced fingers because higher base spread rate R_base means curvature effects are amplified.

4. **Terrain creates features**: Uphill sections spread faster (higher R_base), creating the elongated lobes visible in hilly terrain like Boddington.

5. **Fuel heterogeneity seeds perturbations**: Variable fuel load and moisture create initial irregularities that curvature effects then amplify.

The result is fire perimeters with:
- Multi-lobed shapes (like the north lobe in the Boddington image)
- Ragged edges with convex "headlands" and concave "bays"
- Fingers extending along wind/slope directions
- Complex boundaries that match real fire behavior
- κ_coeff ≈ 0.25 per literature
```

### Ignition Physics (Emergent from Temperature Field)

Unlike the current system where ignition is probabilistic, ignition **emerges naturally**:

```
A cell ignites (φ becomes negative) when:
1. T(x,y) ≥ T_ignition (fuel-specific, ~300-400°C)
2. M(x,y) < M_extinction (moisture below extinction)
3. Neighboring cell is burning (φ < 0 within 1-2 cells)

The level set φ updates to include newly ignited cells.
```

This creates realistic fire spread where:
- Heat radiates ahead of the fire front
- Fuel preheats before ignition
- Moisture must evaporate first (latent heat sink)
- Wind pushes heat downwind → faster spread in that direction

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     UNIFIED GPU FIRE SIMULATION                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                    GPU FIELD TEXTURES (2D)                          │  │
│   ├─────────────────────────────────────────────────────────────────────┤  │
│   │  Temperature  │  Fuel Load  │  Moisture  │  Level Set φ  │  Oxygen  │  │
│   │   (f32)       │   (f32)     │   (f32)    │    (f32)      │  (f32)   │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                              │                                              │
│                              ▼                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                    GPU COMPUTE PASSES                               │  │
│   ├─────────────────────────────────────────────────────────────────────┤  │
│   │  Pass 1: Heat Transfer (radiation + convection + wind advection)   │  │
│   │  Pass 2: Combustion (fuel consumption, heat release, O₂ depletion) │  │
│   │  Pass 3: Moisture (evaporation from heat, equilibrium recovery)    │  │
│   │  Pass 4: Level Set (φ evolution with curvature + spread rate)      │  │
│   │  Pass 5: Ignition Sync (T > T_ign → update φ to include cell)      │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                              │                                              │
│                              ▼                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                    OUTPUT EXTRACTION                                │  │
│   ├─────────────────────────────────────────────────────────────────────┤  │
│   │  Fire Front Vertices (marching squares on φ=0)                     │  │
│   │  Intensity per vertex (from local heat release rate)               │  │
│   │  Burned area mask (φ < 0 regions)                                  │  │
│   │  Statistics (total burned area, fuel consumed, emissions)          │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                    CPU COMPONENTS (minimal)                         │  │
│   ├─────────────────────────────────────────────────────────────────────┤  │
│   │  Ember Transport (Albini spotting - sparse, trajectory-based)      │  │
│   │  Weather Updates (time-varying conditions, FFDI calculation)       │  │
│   │  Suppression Actions (player inputs, agent deployment)             │  │
│   │  Wind Field (mass-consistent solver - could be GPU in future)      │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

1. **Fields replace elements**: No more `Vec<FuelElement>` with spatial queries. Fuel properties are stored in texture channels at each grid cell.

2. **Heat transfer drives spread**: The spread rate R is derived from the temperature gradient ∇T, not computed separately from Rothermel. Heat physically flows into unburned fuel.

3. **Single GPU dispatch per field**: Each pass updates one field using neighboring values. Perfect memory locality, maximum parallelism.

4. **Sparse CPU for non-local physics**: Embers (long-range transport) and suppression (player actions) remain on CPU as they're inherently sparse/event-driven.

5. **Fuel types as texture lookups**: A fuel type index texture maps to a lookup table of properties (heat content, ignition temp, moisture extinction, etc.).

6. **GPU/CPU abstraction**: All field operations go through a `FieldSolver` trait implemented by both `GpuFieldSolver` and `CpuFieldSolver`. At runtime, the system auto-detects GPU availability and selects the appropriate backend.

### CPU Fallback Architecture

The system MUST work on machines without GPU support. This is achieved through a backend abstraction:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         BACKEND ABSTRACTION                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                    FieldSolver Trait                                │  │
│   ├─────────────────────────────────────────────────────────────────────┤  │
│   │  fn step_heat_transfer(&mut self, dt: f32)                         │  │
│   │  fn step_combustion(&mut self, dt: f32)                            │  │
│   │  fn step_level_set(&mut self, dt: f32)                             │  │
│   │  fn step_ignition_sync(&mut self)                                  │  │
│   │  fn read_temperature(&self) -> &[f32]                              │  │
│   │  fn read_level_set(&self) -> &[f32]                                │  │
│   │  fn write_ignition(&mut self, x: u32, y: u32, radius: f32)         │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                              │                                              │
│              ┌───────────────┴───────────────┐                             │
│              ▼                               ▼                             │
│   ┌─────────────────────┐         ┌─────────────────────┐                  │
│   │   GpuFieldSolver    │         │   CpuFieldSolver    │                  │
│   ├─────────────────────┤         ├─────────────────────┤                  │
│   │ wgpu textures       │         │ Vec<f32> arrays     │                  │
│   │ Compute shaders     │         │ Rayon parallel iter │                  │
│   │ <8ms @ 2048²        │         │ <50ms @ 2048²       │                  │
│   └─────────────────────┘         └─────────────────────┘                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Backend Selection:**
```rust
use log::{info, warn};

/// Result of GPU initialization attempt
pub enum GpuInitResult {
    /// GPU initialized successfully
    Success(GpuContext),
    /// No GPU adapter found (silent fallback to CPU)
    NoGpuFound,
    /// GPU found but initialization failed (log warning, fallback to CPU)
    InitFailed { adapter_name: String, error: String },
}

pub fn create_field_solver(terrain: &TerrainData, quality: QualityPreset) -> Box<dyn FieldSolver> {
    #[cfg(feature = "gpu")]
    match GpuContext::new() {
        GpuInitResult::Success(gpu_context) => {
            if gpu_context.can_allocate(quality.grid_width(), quality.grid_height()) {
                info!("Using GPU backend: {}", gpu_context.adapter_name());
                return Box::new(GpuFieldSolver::new(gpu_context, terrain, quality));
            } else {
                warn!("GPU has insufficient memory for {}x{} grid, falling back to CPU",
                    quality.grid_width(), quality.grid_height());
            }
        }
        GpuInitResult::NoGpuFound => {
            info!("No GPU found, using CPU backend");
        }
        GpuInitResult::InitFailed { adapter_name, error } => {
            // GPU was found but failed to initialize - this is worth a warning
            warn!("GPU '{}' found but failed to initialize: {}. Falling back to CPU.", 
                adapter_name, error);
        }
    }
    
    #[cfg(not(feature = "gpu"))]
    info!("GPU feature disabled, using CPU backend");
    
    Box::new(CpuFieldSolver::new(terrain, quality))
}
```

**CPU Implementation uses Rayon for parallelism:**
```rust
impl FieldSolver for CpuFieldSolver {
    fn step_heat_transfer(&mut self, dt: f32) {
        // Parallel iteration over grid cells
        self.temperature_out.par_chunks_mut(self.width)
            .enumerate()
            .for_each(|(y, row)| {
                for x in 0..self.width {
                    // Same physics as GPU shader, just in Rust
                    let T = self.temperature[y * self.width + x];
                    let laplacian = /* compute from neighbors */;
                    let radiation = /* Stefan-Boltzmann */;
                    row[x] = T + dt * (laplacian + radiation);
                }
            });
        
        std::mem::swap(&mut self.temperature, &mut self.temperature_out);
    }
}
```

---

# DETAILED STEP IMPLEMENTATION

The following sections provide detailed code specifications for each step. These are organized by phase and should be referenced when implementing each phase.

---

## Step 1: GPU Infrastructure and Field Textures (Phase 1)

**Objective:** Create wgpu-based GPU compute infrastructure with field texture management AND CPU fallback.

**Part of:** [PHASE 1: Core Infrastructure & Field Solver](#phase-1-core-infrastructure--field-solver)

**Files to Create:**
- [ ] `crates/core/src/solver/mod.rs` - Solver module entry point and re-exports
- [ ] `crates/core/src/solver/context.rs` - GpuContext with device/queue management
- [ ] `crates/core/src/solver/fields.rs` - Field data structures (CPU Vec / GPU textures)
- [ ] `crates/core/src/solver/quality.rs` - Quality presets and auto-detection
- [ ] `crates/core/src/solver/trait.rs` - FieldSolver trait definition
- [ ] `crates/core/src/solver/cpu.rs` - CpuFieldSolver implementation (always available)
- [ ] `crates/core/src/solver/gpu.rs` - GpuFieldSolver implementation (optional "gpu" feature)

**Files to Delete (replaced by new system):**
- [ ] `crates/core/src/physics/element_heat_transfer.rs` - Replaced by field solvers
- [ ] `crates/core/src/core_types/element.rs` - FuelElement replaced by fields

**Implementation Checklist:**

### 1.1 FieldSolver Trait (Backend Abstraction)
```rust
/// Trait for field-based fire simulation backends (GPU or CPU)
pub trait FieldSolver: Send + Sync {
    /// Advance heat transfer by dt seconds
    fn step_heat_transfer(&mut self, dt: f32, wind: Vec2, ambient_temp: f32);
    
    /// Advance combustion (fuel consumption, heat release)
    fn step_combustion(&mut self, dt: f32);
    
    /// Advance moisture (evaporation, equilibrium)
    fn step_moisture(&mut self, dt: f32, humidity: f32);
    
    /// Advance level set (fire front propagation)
    fn step_level_set(&mut self, dt: f32);
    
    /// Sync ignition (T > T_ign → update φ)
    fn step_ignition_sync(&mut self);
    
    /// Read temperature field (for visualization/queries)
    fn read_temperature(&self) -> Cow<[f32]>;
    
    /// Read level set field (for fire front extraction)
    fn read_level_set(&self) -> Cow<[f32]>;
    
    /// Ignite at position with radius
    fn ignite_at(&mut self, x: f32, y: f32, radius: f32);
    
    /// Grid dimensions
    fn dimensions(&self) -> (u32, u32, f32);  // width, height, cell_size
    
    /// Is this the GPU backend?
    fn is_gpu_accelerated(&self) -> bool;
}
```

### 1.2 GpuContext (GPU-only, optional)
```rust
#[cfg(feature = "gpu")]
pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_info: wgpu::AdapterInfo,
}

#[cfg(feature = "gpu")]
impl GpuContext {
    /// Initialize GPU context
    /// 
    /// Returns:
    /// - `GpuInitResult::Success` - GPU ready to use
    /// - `GpuInitResult::NoGpuFound` - No compatible GPU adapter (silent fallback)
    /// - `GpuInitResult::InitFailed` - GPU found but init failed (log warning)
    ///
    /// The distinction matters: "no GPU" is expected on some systems,
    /// but "GPU found but failed" might indicate a driver issue worth logging.
    pub fn new() -> GpuInitResult {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        
        // Try to find a GPU adapter
        let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })) {
            Some(a) => a,
            None => return GpuInitResult::NoGpuFound,
        };
        
        let adapter_name = adapter.get_info().name.clone();
        
        // Try to create device - this can fail even with a valid adapter
        match pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("FireSim GPU"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )) {
            Ok((device, queue)) => GpuInitResult::Success(Self {
                device,
                queue,
                adapter_info: adapter.get_info(),
            }),
            Err(e) => GpuInitResult::InitFailed {
                adapter_name,
                error: e.to_string(),
            },
        }
    }
    
    /// Get adapter name for logging
    pub fn adapter_name(&self) -> &str {
        &self.adapter_info.name
    }
    
    /// Get optimal workgroup size for this GPU vendor
    pub fn optimal_workgroup_size(&self) -> (u32, u32) {
        // NVIDIA/AMD prefer 16x16, Intel prefers 8x8
        match self.adapter_info.vendor {
            0x10DE | 0x1002 => (16, 16),  // NVIDIA, AMD
            _ => (8, 8),                   // Intel, others
        }
    }
    
    /// Check if GPU has enough memory for given grid size
    pub fn can_allocate(&self, width: u32, height: u32) -> bool {
        // Estimate: ~6 float textures × 4 bytes × width × height × 2 (ping-pong)
        let estimated_bytes = 6 * 4 * (width as u64) * (height as u64) * 2;
        // Conservative: require at least 2x estimated for headroom
        estimated_bytes < 256 * 1024 * 1024  // 256MB limit for safety
    }
}
```

### 1.3 CpuFieldSolver (Always Available)
```rust
/// CPU-based field solver using Rayon for parallelism
pub struct CpuFieldSolver {
    // Field arrays (ping-pong buffers for each field)
    temperature: Vec<f32>,
    temperature_back: Vec<f32>,
    fuel_load: Vec<f32>,
    moisture: Vec<f32>,
    level_set: Vec<f32>,
    level_set_back: Vec<f32>,
    oxygen: Vec<f32>,
    fuel_type: Vec<u8>,
    terrain_height: Vec<f32>,
    
    // Fuel lookup table
    fuel_lut: Vec<FuelProperties>,
    
    // Dimensions
    width: usize,
    height: usize,
    cell_size: f32,
}
```

### 1.4 GpuFieldSolver (Optional, requires "gpu" feature)
```rust
#[cfg(feature = "gpu")]
pub struct GpuFieldSolver {
    context: GpuContext,
    
    // GPU textures
    temperature: wgpu::Texture,
    temperature_back: wgpu::Texture,
    // ... other textures
    
    // Compute pipelines
    heat_transfer_pipeline: wgpu::ComputePipeline,
    combustion_pipeline: wgpu::ComputePipeline,
    level_set_pipeline: wgpu::ComputePipeline,
    ignition_sync_pipeline: wgpu::ComputePipeline,
    
    // Staging buffers for readback
    temperature_staging: wgpu::Buffer,
    level_set_staging: wgpu::Buffer,
}
```

### 1.5 Quality Presets
```rust
pub enum QualityPreset {
    Ultra,  // 4096×4096, 2.5m cells, ~10km² coverage
    High,   // 2048×2048, 5m cells, ~10km² coverage
    Medium, // 1024×1024, 10m cells, ~10km² coverage
    Low,    // 512×512, 20m cells, ~10km² coverage
}

impl QualityPreset {
    pub fn grid_dimensions(&self, terrain: &TerrainData) -> (u32, u32, f32);
    
    /// Auto-detect based on hardware
    pub fn recommended() -> Self;
}
```

**Dependencies to add to `crates/core/Cargo.toml`:**
```toml
[features]
default = ["gpu"]
gpu = ["wgpu", "bytemuck", "pollster"]

[dependencies]
wgpu = { version = "22.0", optional = true }
bytemuck = { version = "1.14", features = ["derive"], optional = true }
pollster = { version = "0.3", optional = true }
rayon = "1.10"  # Always needed for CPU fallback
```

**Testing:**
- [ ] Unit test: CPU solver produces valid output
- [ ] Unit test: GPU context initialization (when available)
- [ ] Unit test: Backend auto-selection works correctly
- [ ] Unit test: CPU and GPU produce identical results (determinism)

---

## Step 2: GPU Heat Transfer Compute Shader (Phase 2)

**Objective:** Implement Stefan-Boltzmann radiation + convection + wind advection on GPU.

**Part of:** [PHASE 2: Heat Transfer & Combustion Physics](#phase-2-heat-transfer--combustion-physics)

**Files to Create:**
- [ ] `crates/core/src/solver/shaders/heat_transfer.wgsl` - Heat transfer compute shader
- [ ] `crates/core/src/solver/heat_transfer.rs` - CPU + GPU wrapper for heat transfer pass

**Implementation Checklist:**

### 2.1 Heat Transfer Physics (GPU Shader)
```wgsl
// heat_transfer.wgsl

struct HeatParams {
    width: u32,
    height: u32,
    cell_size: f32,        // meters
    dt: f32,               // seconds
    ambient_temp: f32,     // Kelvin
    wind_x: f32,           // m/s
    wind_y: f32,           // m/s
    stefan_boltzmann: f32, // 5.67e-8 W/(m²·K⁴)
}

@group(0) @binding(0) var<storage, read> temp_in: array<f32>;
@group(0) @binding(1) var<storage, read> fuel_type: array<u32>;
@group(0) @binding(2) var<storage, read> fuel_load: array<f32>;
@group(0) @binding(3) var<storage, read> level_set: array<f32>;
@group(0) @binding(4) var<storage, read_write> temp_out: array<f32>;
@group(0) @binding(5) var<uniform> params: HeatParams;
@group(0) @binding(6) var<storage, read> fuel_lut: array<GpuFuelProperties>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.y * params.width + id.x;
    let pos = vec2<f32>(f32(id.x), f32(id.y)) * params.cell_size;
    
    let T = temp_in[idx];
    let fuel = fuel_lut[fuel_type[idx]];
    let mass = fuel_load[idx] * params.cell_size * params.cell_size;
    let is_burning = level_set[idx] < 0.0;
    
    // 1. Thermal diffusion (Laplacian)
    let T_left  = temp_in[idx - 1];
    let T_right = temp_in[idx + 1];
    let T_up    = temp_in[idx - params.width];
    let T_down  = temp_in[idx + params.width];
    let laplacian = (T_left + T_right + T_up + T_down - 4.0 * T) / (params.cell_size * params.cell_size);
    let diffusion = fuel.thermal_diffusivity * laplacian;
    
    // 2. Stefan-Boltzmann radiation (to/from neighbors)
    // Net radiative flux from hotter neighbors
    var Q_rad = 0.0;
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) { continue; }
            let nidx = (id.y + dy) * params.width + (id.x + dx);
            let T_neighbor = temp_in[nidx];
            let dist = length(vec2<f32>(f32(dx), f32(dy))) * params.cell_size;
            let view_factor = 1.0 / (3.14159 * dist * dist);
            // σε(T_n⁴ - T⁴) with proper sign
            Q_rad += fuel.emissivity * params.stefan_boltzmann * 
                     (pow(T_neighbor, 4.0) - pow(T, 4.0)) * view_factor;
        }
    }
    
    // 3. Radiative loss to atmosphere
    let Q_rad_loss = fuel.emissivity * params.stefan_boltzmann * 
                     (pow(T, 4.0) - pow(params.ambient_temp, 4.0));
    
    // 4. Wind advection (upwind scheme)
    let wind = vec2<f32>(params.wind_x, params.wind_y);
    var T_upwind = T;
    if (params.wind_x > 0.0) { T_upwind = temp_in[idx - 1]; }
    else if (params.wind_x < 0.0) { T_upwind = temp_in[idx + 1]; }
    let advection_x = params.wind_x * (T - T_upwind) / params.cell_size;
    // Similar for y...
    
    // 5. Combustion heat source (if burning)
    var Q_combustion = 0.0;
    if (is_burning && fuel_load[idx] > 0.0) {
        // Heat release from burning - handled in combustion pass
        // Here we just read the result
    }
    
    // 6. Update temperature
    let heat_capacity = mass * fuel.specific_heat;
    let dT = params.dt * (diffusion + Q_rad - Q_rad_loss - advection_x);
    temp_out[idx] = T + dT / max(heat_capacity, 0.001);
}
```

### 2.2 Key Physics Implementation
- [ ] **Full Stefan-Boltzmann T⁴**: No linearization or simplification
- [ ] **View factor geometry**: Proper 1/(πr²) for radiative exchange
- [ ] **Wind advection**: Upwind scheme for stability, pushes heat downwind
- [ ] **Thermal diffusivity**: Fuel-specific conduction through fuel bed
- [ ] **Emissivity**: Fuel-specific (flames ~0.95, fuel bed ~0.7-0.9)

### 2.3 Boundary Conditions
- [ ] Dirichlet at edges: T = T_ambient (open boundaries)
- [ ] Handle edge cases (grid boundaries)

**Scientific Validation:**
- [ ] Stefan-Boltzmann formula exact (no approximations)
- [ ] Heat flux units correct (W/m² → temperature change via heat capacity)
- [ ] Wind advection conserves energy

**Testing:**
- [ ] Unit test: Hot spot cools via radiation in still air
- [ ] Unit test: Wind advection pushes heat in correct direction
- [ ] Unit test: Temperature gradients drive correct heat flow

---

## Step 3: GPU Combustion and Fuel Consumption (Phase 2)

**Objective:** Implement fuel combustion, heat release, moisture evaporation, and oxygen consumption on GPU.

**Part of:** [PHASE 2: Heat Transfer & Combustion Physics](#phase-2-heat-transfer--combustion-physics)

**Files to Create:**
- [ ] `crates/core/src/solver/shaders/combustion.wgsl` - Combustion compute shader
- [ ] `crates/core/src/solver/combustion.rs` - CPU + GPU wrapper for combustion pass

**Implementation Checklist:**

### 3.1 Combustion Physics (GPU Shader)
```wgsl
// combustion.wgsl

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.y * params.width + id.x;
    
    let T = temperature[idx];
    let fuel = fuel_lut[fuel_type[idx]];
    let F = fuel_load[idx];
    let M = moisture[idx];
    let O2 = oxygen[idx];
    let is_burning = level_set[idx] < 0.0;
    
    if (!is_burning || F <= 0.0) {
        return;  // No combustion if not burning or no fuel
    }
    
    // 1. Moisture evaporation (latent heat sink - 2260 kJ/kg)
    // CRITICAL: Moisture must evaporate BEFORE fuel ignites (per project rules)
    var heat_available = /* heat input from neighbors */;
    var moisture_evaporated = 0.0;
    if (M > 0.0) {
        let latent_heat = 2260.0;  // kJ/kg
        let max_evap = heat_available / latent_heat;
        moisture_evaporated = min(M * F, max_evap);
        heat_available -= moisture_evaporated * latent_heat;
        moisture_out[idx] = max(0.0, M - moisture_evaporated / F);
    }
    
    // 2. Fuel consumption rate (Rothermel-based)
    // Only burn if moisture below extinction and temperature above ignition
    var burn_rate = 0.0;
    if (M < fuel.moisture_extinction && T > fuel.ignition_temperature) {
        // Base burn rate from Rothermel reaction intensity
        let moisture_damping = 1.0 - (M / fuel.moisture_extinction);
        let temp_factor = (T - fuel.ignition_temperature) / 500.0;  // Normalized
        burn_rate = fuel.base_burn_rate * moisture_damping * temp_factor;
        
        // Oxygen limitation (stoichiometric)
        let o2_required = burn_rate * 1.33;  // kg O₂ per kg fuel
        if (O2 * /* air density */ < o2_required) {
            burn_rate *= O2 / (o2_required / /* air density */);
        }
    }
    
    // 3. Heat release from combustion
    let fuel_consumed = burn_rate * params.dt;
    let heat_released = fuel_consumed * fuel.heat_content;
    
    // 4. Update fields
    fuel_load_out[idx] = max(0.0, F - fuel_consumed);
    oxygen_out[idx] = max(0.0, O2 - fuel_consumed * 1.33 / cell_volume);
    
    // 5. Temperature increase from combustion
    let mass = F * params.cell_size * params.cell_size;
    let dT = heat_released * fuel.self_heating_fraction / (mass * fuel.specific_heat);
    temperature_out[idx] = min(T + dT, fuel.max_flame_temperature);
}
```

### 3.2 Key Physics
- [ ] **Moisture evaporation FIRST**: 2260 kJ/kg latent heat before temperature rise
- [ ] **Oxygen limitation**: Stoichiometric ratio limits burn rate in low-O₂ conditions
- [ ] **Rothermel burn rate**: Base rate modified by moisture damping and temperature
- [ ] **Self-heating**: Fuel-specific fraction of heat retained in fuel bed
- [ ] **Max flame temperature**: Fuel-specific cap (eucalyptus higher due to oils)

### 3.3 Combustion Products
- [ ] CO₂ production: proportional to fuel consumed
- [ ] H₂O vapor: from combustion + evaporated moisture
- [ ] Smoke particles: for atmospheric effects

**Testing:**
- [ ] Unit test: Wet fuel requires heat input before burning
- [ ] Unit test: Burn rate decreases with oxygen depletion
- [ ] Unit test: Temperature rises during active combustion

---

## Step 4: Level Set Evolution and Curvature (Phase 3)

**Objective:** Implement level set φ field evolution with curvature-dependent spread.

**Part of:** [PHASE 3: Level Set Fire Front Tracking](#phase-3-level-set-fire-front-tracking)

**Files to Create:**
- [ ] `crates/core/src/solver/shaders/level_set.wgsl` - Level set compute shader
- [ ] `crates/core/src/solver/level_set.rs` - CPU + GPU wrapper for level set pass

**Implementation Checklist:**

### 4.1 Level Set Evolution (GPU Shader)
```wgsl
// level_set.wgsl

struct LevelSetParams {
    width: u32,
    height: u32,
    cell_size: f32,
    dt: f32,
    curvature_coeff: f32,  // 0.25 per Margerit (2002)
    noise_amplitude: f32,
    time: f32,
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.y * params.width + id.x;
    let dx = params.cell_size;
    
    // Get φ and neighbors
    let phi = phi_in[idx];
    let phi_left  = phi_in[idx - 1];
    let phi_right = phi_in[idx + 1];
    let phi_up    = phi_in[idx - params.width];
    let phi_down  = phi_in[idx + params.width];
    
    // 1. Compute gradient using upwind scheme (Godunov)
    let Dx_minus = (phi - phi_left) / dx;
    let Dx_plus  = (phi_right - phi) / dx;
    let Dy_minus = (phi - phi_up) / dx;
    let Dy_plus  = (phi_down - phi) / dx;
    
    // Godunov Hamiltonian for |∇φ|
    let grad_mag = sqrt(
        max(max(Dx_minus, 0.0), -min(Dx_plus, 0.0)) * max(max(Dx_minus, 0.0), -min(Dx_plus, 0.0)) +
        max(max(Dy_minus, 0.0), -min(Dy_plus, 0.0)) * max(max(Dy_minus, 0.0), -min(Dy_plus, 0.0))
    );
    
    // 2. Compute curvature κ
    let phi_xx = (phi_right - 2.0 * phi + phi_left) / (dx * dx);
    let phi_yy = (phi_down - 2.0 * phi + phi_up) / (dx * dx);
    let phi_xy = (phi_in[idx + params.width + 1] - phi_in[idx + params.width - 1] 
                - phi_in[idx - params.width + 1] + phi_in[idx - params.width - 1]) / (4.0 * dx * dx);
    let phi_x = (phi_right - phi_left) / (2.0 * dx);
    let phi_y = (phi_down - phi_up) / (2.0 * dx);
    
    let denom = pow(phi_x * phi_x + phi_y * phi_y, 1.5);
    var kappa = 0.0;
    if (denom > 1e-10) {
        kappa = (phi_xx * phi_y * phi_y - 2.0 * phi_x * phi_y * phi_xy + phi_yy * phi_x * phi_x) / denom;
    }
    
    // 3. Get spread rate from temperature gradient (heat-driven spread)
    // Spread rate is proportional to heat flux into unburned fuel
    let R = spread_rate[idx];
    
    // 4. Apply curvature effect (Margerit 2002)
    // Convex (κ > 0) → faster spread (fingers)
    // Concave (κ < 0) → slower spread (indentations)
    let R_effective = R * (1.0 + params.curvature_coeff * kappa);
    
    // 5. Add stochastic noise for realistic irregularity
    let noise = simplex_noise(vec2<f32>(f32(id.x), f32(id.y)) * 0.05 + params.time * 0.1);
    let R_final = R_effective * (1.0 + params.noise_amplitude * noise);
    
    // 6. Hamilton-Jacobi update: ∂φ/∂t + R|∇φ| = 0
    let dphi = -R_final * grad_mag * params.dt;
    phi_out[idx] = phi + dphi;
}
```

### 4.2 Spread Rate from Heat Transfer
Instead of using Rothermel directly, spread rate emerges from heat physics:
```rust
// In spread_rate.wgsl
// R is proportional to heat flux into the cell
let heat_flux = /* from heat transfer pass */;
let fuel = fuel_lut[fuel_type[idx]];

// Spread rate = heat flux / (fuel load × heat to ignition)
let heat_to_ignition = fuel.specific_heat * (fuel.ignition_temp - temperature[idx]) 
                     + moisture[idx] * fuel_load[idx] * LATENT_HEAT_WATER;
let R = heat_flux / heat_to_ignition;
```

This makes spread rate **emerge from physics** rather than being computed separately.

### 4.3 Signed Distance Reinitialization
Periodically reinitialize φ to maintain signed distance property:
```wgsl
// Every 10-20 timesteps, run reinitialization pass
@compute @workgroup_size(16, 16)
fn reinitialize(@builtin(global_invocation_id) id: vec3<u32>) {
    // Solve ∂φ/∂τ = sign(φ₀)(1 - |∇φ|) to restore |∇φ| = 1
    // Use a few iterations of this PDE
}
```

### 4.4 Ignition Sync Pass
Update φ to include cells that have reached ignition temperature:
```wgsl
@compute @workgroup_size(16, 16)
fn ignition_sync(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.y * params.width + id.x;
    let T = temperature[idx];
    let M = moisture[idx];
    let fuel = fuel_lut[fuel_type[idx]];
    let phi = phi_in[idx];
    
    // If currently unburned but at ignition conditions
    if (phi > 0.0 && T >= fuel.ignition_temperature && M < fuel.moisture_extinction) {
        // Check if adjacent to burning cell
        let has_burning_neighbor = 
            phi_in[idx - 1] < 0.0 || phi_in[idx + 1] < 0.0 ||
            phi_in[idx - params.width] < 0.0 || phi_in[idx + params.width] < 0.0;
        
        if (has_burning_neighbor) {
            // Ignite this cell
            phi_out[idx] = -params.cell_size * 0.5;  // Small negative value
        }
    }
}
```

**Testing:**
- [ ] Unit test: Circular fire expands at uniform rate (no curvature effect when κ_coeff=0)
- [ ] Unit test: Curvature calculation correct (convex=positive, concave=negative)
- [ ] Unit test: Ignition sync correctly updates φ when temperature reached

---

## Step 5: New FireSimulation Core Structure (Phase 4)

**Objective:** Replace the element-based `FireSimulation` with a unified GPU field-based simulation.

**Part of:** [PHASE 4: Simulation Integration & Ignition](#phase-4-simulation-integration--ignition)

**Files to Create:**
- [ ] `crates/core/src/simulation/gpu_simulation.rs` - New GPU-based FireSimulation

**Files to Modify/Delete:**
- [ ] `crates/core/src/simulation/mod.rs` - Replace or heavily modify
- [ ] Delete element-specific code paths

**Implementation Checklist:**

### 5.1 New FireSimulation Structure
```rust
/// Unified GPU-accelerated fire simulation
pub struct FireSimulation {
    // GPU context and compute infrastructure
    gpu: GpuContext,
    
    // All simulation state as GPU textures
    fields: FireFieldTextures,
    
    // Compute pipelines for each physics pass
    heat_transfer_pipeline: wgpu::ComputePipeline,
    combustion_pipeline: wgpu::ComputePipeline,
    level_set_pipeline: wgpu::ComputePipeline,
    ignition_sync_pipeline: wgpu::ComputePipeline,
    
    // Fuel type lookup table (uniform buffer)
    fuel_lut: FuelTypeLUT,
    
    // Weather and wind (updated from CPU)
    weather: WeatherSystem,
    wind_field: WindField,
    
    // Ember system (sparse, remains on CPU)
    embers: Vec<Ember>,
    
    // Output extraction
    fire_front: FireFront,
    
    // Statistics
    total_burned_area: f32,
    total_fuel_consumed: f32,
    simulation_time: f32,
}
```

### 5.2 Main Update Loop
```rust
impl FireSimulation {
    pub fn update(&mut self, dt: f32) {
        self.simulation_time += dt;
        
        // 1. Update weather and upload wind field to GPU
        self.weather.update(dt);
        self.upload_wind_field();
        
        // 2. GPU compute passes (single command buffer submission)
        let mut encoder = self.gpu.device().create_command_encoder(&Default::default());
        
        // Pass 1: Heat transfer (radiation + convection + wind)
        self.dispatch_heat_transfer(&mut encoder, dt);
        
        // Pass 2: Combustion (fuel consumption, heat release)
        self.dispatch_combustion(&mut encoder, dt);
        
        // Pass 3: Moisture update (evaporation + equilibrium)
        self.dispatch_moisture(&mut encoder, dt);
        
        // Pass 4: Level set evolution (φ update with curvature)
        self.dispatch_level_set(&mut encoder, dt);
        
        // Pass 5: Ignition sync (T > T_ign → update φ)
        self.dispatch_ignition_sync(&mut encoder);
        
        // Submit all GPU work
        self.gpu.queue().submit(Some(encoder.finish()));
        
        // 3. CPU-side sparse updates
        self.update_embers(dt);
        self.process_suppression_actions();
        
        // 4. Extract fire front (can be async/deferred)
        self.extract_fire_front();
        
        // 5. Update statistics
        self.update_statistics();
    }
    
    fn dispatch_heat_transfer(&self, encoder: &mut wgpu::CommandEncoder, dt: f32) {
        // Upload parameters
        let params = HeatParams {
            width: self.fields.width,
            height: self.fields.height,
            cell_size: self.fields.cell_size,
            dt,
            ambient_temp: self.weather.temperature.to_kelvin(),
            wind_x: self.weather.wind_vector().x,
            wind_y: self.weather.wind_vector().y,
            stefan_boltzmann: 5.67e-8,
        };
        
        // Create bind group and dispatch
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.heat_transfer_pipeline);
        pass.set_bind_group(0, &self.heat_transfer_bind_group, &[]);
        pass.dispatch_workgroups(
            self.fields.width.div_ceil(16),
            self.fields.height.div_ceil(16),
            1,
        );
    }
}
```

### 5.3 Initialization from Terrain
```rust
impl FireSimulation {
    pub fn new(terrain: &TerrainData, quality: QualityPreset) -> Result<Self, GpuError> {
        let gpu = GpuContext::new()?;
        
        // Determine grid size from quality preset and terrain
        let (width, height, cell_size) = quality.grid_dimensions(terrain);
        
        // Create field textures
        let fields = FireFieldTextures::new(&gpu, width, height, cell_size);
        
        // Initialize fuel type grid from terrain
        fields.initialize_fuel_from_terrain(terrain);
        
        // Initialize temperature to ambient
        fields.fill_temperature(terrain.ambient_temperature);
        
        // Initialize level set to "all unburned" (positive everywhere)
        fields.fill_level_set(f32::MAX);
        
        // ... create pipelines, etc.
        
        Ok(Self { gpu, fields, ... })
    }
    
    pub fn ignite_at(&mut self, position: Vec3, radius: f32) {
        // Set φ < 0 in circular region around position
        // This is done via a small GPU compute pass or CPU upload
        self.set_level_set_circle(position, radius, -1.0);
        
        // Also set high temperature to kick-start combustion
        self.set_temperature_circle(position, radius, IGNITION_TEMP + 100.0);
    }
}
```

### 5.4 Migration Path
Since breaking changes are allowed:
1. Create new `FireSimulation` alongside old one during development
2. Update demo and FFI to use new system
3. Delete old element-based code once new system works

**Testing:**
- [ ] Integration test: Ignition spreads from initial point
- [ ] Integration test: Fire stops at fuel boundaries
- [ ] Integration test: Wind affects spread direction

---

## Step 6: Fire Front Extraction and Visualization (Phase 4)

**Objective:** Extract fire perimeter polyline from GPU φ field for rendering.

**Part of:** [PHASE 4: Simulation Integration & Ignition](#phase-4-simulation-integration--ignition)

**Files to Create:**
- [ ] `crates/core/src/solver/marching_squares.rs` - Contour extraction (CPU, optionally GPU)
- [ ] `crates/core/src/simulation/fire_front.rs` - FireFront data structure

**Implementation Checklist:**

### 6.1 FireFront Structure
```rust
/// Fire perimeter data for visualization
pub struct FireFront {
    /// Perimeter vertices in world coordinates
    pub vertices: Vec<Vec3>,
    /// Normal vectors pointing outward (toward unburned fuel)
    pub normals: Vec<Vec3>,
    /// Spread velocity at each vertex
    pub velocities: Vec<Vec3>,
    /// Byram intensity (kW/m) at each vertex
    pub intensities: Vec<f32>,
    /// Curvature at each vertex
    pub curvatures: Vec<f32>,
    /// Indices into vertices for multiple disconnected fronts
    pub front_starts: Vec<usize>,
}
```

### 6.2 Marching Squares Algorithm
```rust
impl FireSimulation {
    pub fn extract_fire_front(&mut self) {
        // Read φ field from GPU (async-friendly)
        let phi = self.fields.read_level_set();
        
        // Marching squares to find φ = 0 contour
        let mut vertices = Vec::new();
        let mut front_starts = vec![0];
        
        for y in 0..self.fields.height - 1 {
            for x in 0..self.fields.width - 1 {
                // Get 4 corners of cell
                let tl = phi[y * self.fields.width + x];
                let tr = phi[y * self.fields.width + x + 1];
                let bl = phi[(y + 1) * self.fields.width + x];
                let br = phi[(y + 1) * self.fields.width + x + 1];
                
                // Compute marching squares case (0-15)
                let case = (if tl < 0.0 { 1 } else { 0 })
                         | (if tr < 0.0 { 2 } else { 0 })
                         | (if br < 0.0 { 4 } else { 0 })
                         | (if bl < 0.0 { 8 } else { 0 });
                
                // Add edge vertices based on case
                // ... standard marching squares lookup table
            }
        }
        
        // Chain vertices into contiguous polylines
        // ... connect adjacent cells
        
        // Compute derived quantities at each vertex
        self.fire_front = FireFront {
            vertices,
            normals: self.compute_normals(&vertices, &phi),
            velocities: self.compute_velocities(&vertices),
            intensities: self.compute_intensities(&vertices),
            curvatures: self.compute_curvatures(&vertices, &phi),
            front_starts,
        };
    }
}
```

### 6.3 Derived Quantities
- [ ] **Normal**: From φ gradient at vertex: n = ∇φ/|∇φ|
- [ ] **Velocity**: v = R × (-∇φ/|∇φ|) - spread direction × speed
- [ ] **Intensity**: Byram I = H × ṁ × R (kW/m)
- [ ] **Curvature**: Already computed in level set step

### 6.4 GPU-Accelerated Extraction (Optional)
For very large grids, marching squares can run on GPU:
```wgsl
// Each thread handles one cell, outputs vertices to append buffer
@compute @workgroup_size(16, 16)
fn marching_squares(@builtin(global_invocation_id) id: vec3<u32>) {
    // ... compute case and append vertices atomically
}
```

**Testing:**
- [ ] Unit test: Single ignition produces closed contour
- [ ] Unit test: Multiple fires produce separate contours
- [ ] Unit test: Vertex interpolation smooth at cell edges

---

## Step 7: Ember Spotting Integration (Phase 5)

**Objective:** Maintain ember spotting physics with the new GPU field system.

**Part of:** [PHASE 5: Ember, Weather & Wind Coupling](#phase-5-ember-weather--wind-coupling)

**Files to Modify:**
- [ ] Keep `crates/core/src/physics/albini_spotting.rs` - Core physics unchanged
- [ ] Modify ember generation to use fire front

**Implementation Checklist:**

### 7.1 Ember Generation from Fire Front
```rust
impl FireSimulation {
    fn generate_embers(&mut self, dt: f32) {
        // Embers are generated at the fire front (φ ≈ 0)
        // Rate scales with local intensity
        
        for (i, vertex) in self.fire_front.vertices.iter().enumerate() {
            let intensity = self.fire_front.intensities[i];
            
            // Albini (1983) ember generation rate
            let ember_rate = calculate_ember_generation_rate(
                intensity,
                self.get_fuel_at_position(*vertex),
                self.weather.wind_speed(),
            );
            
            // Stochastic ember generation
            if rand::random::<f32>() < ember_rate * dt {
                let ember = Ember::new(
                    *vertex,
                    self.get_fuel_at_position(*vertex),
                    intensity,
                    self.weather.wind_vector(),
                );
                self.embers.push(ember);
            }
        }
    }
    
    fn update_embers(&mut self, dt: f32) {
        // Existing Albini trajectory physics (CPU)
        for ember in &mut self.embers {
            ember.update(dt, &self.weather, &self.fields.terrain_height);
            
            // Check for landing
            if ember.has_landed() {
                // Attempt ignition at landing position
                self.attempt_spot_fire(ember);
            }
        }
        
        // Remove inactive embers
        self.embers.retain(|e| e.is_active());
    }
    
    fn attempt_spot_fire(&mut self, ember: &Ember) {
        if !ember.can_ignite() {
            return;
        }
        
        let pos = ember.position;
        let fuel = self.get_fuel_at_position(pos);
        let moisture = self.fields.read_moisture_at(pos);
        
        // Probability based on ember temp, fuel receptivity, moisture
        let ignition_prob = ember.calculate_ignition_probability(fuel, moisture);
        
        if rand::random::<f32>() < ignition_prob {
            // Ignite spot fire
            self.ignite_at(pos, 2.0);  // 2m radius spot fire
        }
    }
}
```

### 7.2 Fuel Type Lookup from Fields
```rust
impl FireSimulation {
    fn get_fuel_at_position(&self, pos: Vec3) -> &Fuel {
        let grid_x = (pos.x / self.fields.cell_size) as usize;
        let grid_y = (pos.y / self.fields.cell_size) as usize;
        let fuel_type_idx = self.fields.read_fuel_type_at(grid_x, grid_y);
        &self.fuel_types[fuel_type_idx as usize]
    }
}
```

**Testing:**
- [ ] Integration test: Embers generated at active fire front
- [ ] Integration test: Spot fires ignite at landing positions
- [ ] Validation: Spotting distances match Albini predictions

---

## Step 8: Performance and Optimization (Phase 6)

**Objective:** Achieve 60 FPS with 10km² fire area on mid-range GPU (GTX 1660 / RX 580).

**Part of:** [PHASE 6: FFI, Performance & Validation](#phase-6-ffi-performance--validation)

**Files to Create:**
- [ ] `crates/core/src/solver/profiler.rs` - GPU/CPU timing and diagnostics

**Implementation Checklist:**

### 8.1 Performance Targets
| Pass | Target Time | Grid Size |
|------|-------------|-----------|
| Heat transfer | <2ms | 2048×2048 |
| Combustion | <1ms | 2048×2048 |
| Level set | <2ms | 2048×2048 |
| Ignition sync | <0.5ms | 2048×2048 |
| Fire front extraction | <1ms | 2048×2048 |
| **Total GPU time** | **<8ms** | - |

### 8.2 Optimization Strategies
- [ ] **Single command buffer**: Submit all passes in one `queue.submit()`
- [ ] **Ping-pong buffers**: No GPU stalls waiting for previous frame
- [ ] **Workgroup size tuning**: Auto-detect optimal size per GPU vendor
- [ ] **Memory coalescing**: 16×16 workgroups for good cache behavior
- [ ] **Narrow band optimization**: Only update cells near fire front (φ close to 0)

### 8.3 Narrow Band Method
For large fires, most cells are either fully burned (φ << 0) or far from fire (φ >> 0). Only cells near the front need updating:

```wgsl
@compute @workgroup_size(16, 16)
fn level_set_narrow_band(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.y * params.width + id.x;
    let phi = phi_in[idx];
    
    // Skip cells far from fire front
    if (abs(phi) > BAND_WIDTH * params.cell_size) {
        phi_out[idx] = phi;  // Just copy
        return;
    }
    
    // Full level set update for cells in the band
    // ...
}
```

### 8.4 GPU Profiling
```rust
pub struct GpuProfiler {
    timestamp_query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    pass_times: Vec<f64>,
}

impl GpuProfiler {
    pub fn begin_pass(&mut self, encoder: &mut wgpu::CommandEncoder, pass_name: &str);
    pub fn end_pass(&mut self, encoder: &mut wgpu::CommandEncoder);
    pub fn resolve(&mut self) -> &[f64];
}
```

### 8.5 Dynamic Quality Scaling
```rust
impl FireSimulation {
    fn check_frame_time(&mut self) {
        let frame_time = self.profiler.total_gpu_time();
        
        if frame_time > 12.0 {
            // Downgrade quality
            self.resize_grid(self.fields.width / 2, self.fields.height / 2);
        } else if frame_time < 4.0 && self.quality < QualityPreset::Ultra {
            // Upgrade quality
            self.resize_grid(self.fields.width * 2, self.fields.height * 2);
        }
    }
}
```

**Testing:**
- [ ] Benchmark: 2048² grid <8ms on GTX 1660
- [ ] Benchmark: 1024² grid <4ms on Intel UHD 770
- [ ] Validation: Narrow band produces identical results to full update

---

## Step 9: FFI and External Integration (Phase 6)

**Objective:** Update FFI to expose new GPU-based simulation.

**Part of:** [PHASE 6: FFI, Performance & Validation](#phase-6-ffi-performance--validation)

**Files to Modify:**
- [ ] `crates/ffi/src/lib.rs` - Update for new FireSimulation API
- [ ] `crates/ffi/src/queries.rs` - Add fire front and field queries
- [ ] `crates/ffi/FireSimFFI.h` - Update C header

**Implementation Checklist:**

### 9.1 Core FFI Changes
```rust
/// Create fire simulation (now GPU-based)
#[no_mangle]
pub extern "C" fn fire_sim_create(
    terrain_data: *const TerrainData,
    quality: i32,  // 0=Auto, 1=Low, 2=Medium, 3=High, 4=Ultra
) -> *mut FireSimInstance;

/// Get whether GPU is being used
#[no_mangle]
pub extern "C" fn fire_sim_is_gpu_accelerated(
    instance: *const FireSimInstance,
) -> bool;
```

### 9.2 Fire Front Export
```rust
/// Get fire front vertices for rendering
#[no_mangle]
pub extern "C" fn fire_sim_get_fire_front_vertices(
    instance: *const FireSimInstance,
    out_vertices: *mut Vec3,
    out_count: *mut usize,
    max_count: usize,
) -> i32;

/// Get fire front intensities for shader coloring
#[no_mangle]
pub extern "C" fn fire_sim_get_fire_front_intensities(
    instance: *const FireSimInstance,
    out_intensities: *mut f32,
    out_count: *mut usize,
    max_count: usize,
) -> i32;

/// Get burned area mask texture
#[no_mangle]
pub extern "C" fn fire_sim_get_burned_mask(
    instance: *const FireSimInstance,
    out_mask: *mut u8,
    width: u32,
    height: u32,
) -> i32;

/// Get simulation statistics
#[no_mangle]
pub extern "C" fn fire_sim_get_statistics(
    instance: *const FireSimInstance,
    out_stats: *mut SimulationStatistics,
) -> i32;

#[repr(C)]
pub struct SimulationStatistics {
    pub burned_area_hectares: f32,
    pub fuel_consumed_tonnes: f32,
    pub active_fire_perimeter_km: f32,
    pub max_intensity_kw_m: f32,
    pub ember_count: u32,
    pub gpu_frame_time_ms: f32,
}
```

### 9.3 Field Read-back (Debug/Visualization)
```rust
/// Read temperature field for visualization (debug builds)
#[no_mangle]
pub extern "C" fn fire_sim_read_temperature_field(
    instance: *const FireSimInstance,
    out_data: *mut f32,
    width: *mut u32,
    height: *mut u32,
) -> i32;
```

**Testing:**
- [ ] Integration test: FFI compiles and links correctly
- [ ] Memory safety: No buffer overflows
- [ ] Performance: Data export <1ms

---

## Step 10: Validation and Testing (Phase 6)

**Objective:** Ensure scientific accuracy, visual realism, and backwards compatibility where needed.

**Part of:** [PHASE 6: FFI, Performance & Validation](#phase-6-ffi-performance--validation)

**Files to Create:**
- [ ] `crates/core/tests/gpu_fire_validation.rs` - New GPU system tests
- [ ] `crates/core/tests/heat_transfer_gpu.rs` - Heat transfer physics tests
- [ ] `docs/validation/gpu_fire_system.md` - Validation documentation

**Implementation Checklist:**

### 10.1 Physics Validation
- [ ] Stefan-Boltzmann radiation matches analytical solutions
- [ ] Heat diffusion matches 1D/2D analytical solutions
- [ ] Combustion heat release matches fuel heat content
- [ ] Moisture evaporation consumes correct latent heat
- [ ] Level set curvature formula correct

### 10.2 Fire Behavior Validation
- [ ] Fire spreads faster downwind (head fire)
- [ ] Fire spreads slower upwind (back fire)
- [ ] Fire spreads faster uphill
- [ ] Fire stops at fuel boundaries
- [ ] Wet fuel requires more heat to ignite

### 10.3 Visual Validation
- [ ] Fire fronts show realistic jagged perimeters
- [ ] Curvature effect produces fingering patterns
- [ ] Fire shapes comparable to historical fire maps
- [ ] Head/flank/back ratios match Anderson (1983)

### 10.4 Performance Validation
- [ ] GPU <8ms for 2048×2048 grid on GTX 1660
- [ ] Memory usage <256MB GPU textures
- [ ] No GPU memory leaks over long runs

### 10.5 Regression Testing
- [ ] Ember spotting still works (Albini physics)
- [ ] Weather system unchanged
- [ ] Suppression actions still function
- [ ] FFDI calculations unchanged

---

## Code to Delete (Breaking Changes)

The following files/modules will be **deleted** as they're replaced by the GPU field system:

### Files to Delete
- [ ] `crates/core/src/physics/element_heat_transfer.rs` - Replaced by GPU heat transfer
- [ ] `crates/core/src/core_types/element.rs` - FuelElement replaced by fields
- [ ] `crates/core/src/core_types/spatial.rs` - SpatialIndex no longer needed

### Code to Remove from Other Files
- [ ] `simulation/mod.rs`: All element-based code (`Vec<FuelElement>`, `burning_elements`, `nearby_cache`, etc.)
- [ ] `core_types/mod.rs`: FuelElement exports
- [ ] Integration tests that use FuelElement directly

### Code to Preserve
- [ ] `physics/rothermel.rs` - Used for Rothermel LUT generation
- [ ] `physics/albini_spotting.rs` - Ember physics unchanged
- [ ] `physics/combustion_physics.rs` - Combustion product formulas
- [ ] `core_types/fuel.rs` - Fuel type definitions
- [ ] `core_types/weather.rs` - Weather system
- [ ] `grid/windfield.rs` - Mass-consistent wind solver (future GPU candidate)

---

## Success Criteria

### Must Have (MVP)
- [ ] Fire simulation runs on GPU when available, **with full CPU fallback**
- [ ] CPU fallback produces identical results to GPU (deterministic)
- [ ] Fire fronts display realistic jagged perimeters (like Boddington fire image)
- [ ] Curvature-dependent spread creates natural fingering patterns
- [ ] Heat transfer drives fire spread (not separate Rothermel calculation)
- [ ] GPU achieves <8ms total compute time on GTX 1660
- [ ] CPU fallback achieves <50ms on modern CPU (i5-12400 or equivalent)
- [ ] Ember spotting still functions
- [ ] `cargo clippy --all-targets --all-features` passes with ZERO warnings
- [ ] `cargo fmt --all --check` passes

### Should Have
- [ ] Curvature-dependent spread creates natural fingering
- [ ] Stochastic noise adds realistic run-to-run variation
- [ ] Fire front vertices exported for game engine rendering
- [ ] Quality presets auto-detected based on GPU
- [ ] Narrow band optimization for large fires

### Nice to Have
- [ ] Wind field solver also on GPU
- [ ] Real-time quality scaling
- [ ] Debug visualization of temperature/φ fields

---

## Dependencies

```toml
# crates/core/Cargo.toml
[features]
# GPU is ON by default, disable with --no-default-features for CI/coding agent
default = ["gpu"]
gpu = ["dep:wgpu", "dep:bytemuck", "dep:pollster"]

[dependencies]
# GPU dependencies (optional, disabled with --no-default-features)
wgpu = { version = "22.0", optional = true }
bytemuck = { version = "1.14", features = ["derive"], optional = true }
pollster = { version = "0.3", optional = true }

# CPU backend (always available)
rayon = "1.10"
```

**Usage:**
```bash
# Default build (GPU enabled)
cargo build

# CPU-only build (for CI/coding agent without GPU)
cargo build --no-default-features

# Explicit GPU build
cargo build --features gpu
```

---

## References

1. **Sethian, J.A. (1999)**. "Level Set Methods and Fast Marching Methods." Cambridge University Press.
2. **Margerit, J., Séro-Guillaume, O. (2002)**. "Modelling forest fires. Part II: reduction to two-dimensional models." Int. J. Heat Mass Transfer, 45, 1723-1737.
3. **Rothermel, R.C. (1972)**. "A mathematical model for predicting fire spread in wildland fuels." USDA Forest Service Research Paper INT-115.
4. **Anderson, H.E. (1983)**. "Predicting Wind-Driven Wild Land Fire Size and Shape." USDA Research Paper INT-305.
5. **Stefan-Boltzmann Law**. Stefan (1879), Boltzmann (1884).
6. **Finney, M.A. (2003)**. "Calculation of fire spread rates across random landscapes." USDA RMRS-RP-44.
7. **Byram, G.M. (1959)**. "Combustion of forest fuels." In: Forest Fire: Control and Use.
8. **Albini, F.A. (1983)**. "Transport of firebrands by line thermals." Combustion Science and Technology, 32, 277-288.
9. **Clark, T.L. et al. (1996)**. "Coupled atmosphere-fire model." Int. J. Wildland Fire, 6(2), 55-68. (Fire-atmosphere coupling)
10. **Coen, J.L. (2005)**. "Simulation of the Big Elk Fire using coupled atmosphere-fire modeling." Int. J. Wildland Fire, 14(1), 49-59.
11. **Mandel, J. et al. (2011)**. "Coupled atmosphere-wildland fire modeling with WRF 3.3 and SFIRE 2011."

---

# PHASE IMPLEMENTATION DETAILS

The following sections detail each phase for GitHub Coding Agent implementation.

---

## PHASE 1: Core Infrastructure & Field Solver

**Objective:** Create the foundational GPU/CPU abstraction layer and field data structures.

**Branch Name:** `feature/gpu-fire-phase1-infrastructure`

**Estimated Time:** 5-7 days

### Phase 1 Deliverables

- [ ] `FieldSolver` trait defining the backend-agnostic interface
- [ ] `CpuFieldSolver` implementation using `Vec<f32>` and Rayon
- [ ] `GpuFieldSolver` skeleton (wgpu context, texture allocation)
- [ ] `GpuContext` with device/queue management and capability detection
- [ ] `QualityPreset` enum with auto-detection logic
- [ ] Backend auto-selection (try GPU, fall back to CPU)
- [ ] Unit tests for field allocation, read/write, and backend selection

### Phase 1 Files to Create

```
crates/core/src/solver/
├── mod.rs              # Module entry, re-exports, feature gates
├── trait.rs            # FieldSolver trait definition
├── cpu.rs              # CpuFieldSolver implementation (always available)
├── gpu.rs              # GpuFieldSolver implementation (behind "gpu" feature)
├── context.rs          # GpuContext (device, queue, capabilities)
├── quality.rs          # Quality presets, auto-detection
└── fields.rs           # Field data structures (CPU Vec / GPU textures)
```

### Phase 1 Dependencies to Add

```toml
# crates/core/Cargo.toml
[features]
default = ["gpu"]
gpu = ["wgpu", "bytemuck", "pollster"]

[dependencies]
wgpu = { version = "22.0", optional = true }
bytemuck = { version = "1.14", features = ["derive"], optional = true }
pollster = { version = "0.3", optional = true }
rayon = "1.10"  # Always required for CPU fallback
```

### Phase 1 Validation Criteria

**⚠️ Phase 1-3: Partial build expected - old simulation code will have errors**

Validate the NEW solver/ module only:
- [ ] `cargo check -p fire-sim-core --lib 2>&1 | grep -c "solver::"` shows 0 errors in solver module
- [ ] Unit test: `CpuFieldSolver` allocates and initializes fields correctly
- [ ] Unit test: Backend selection returns CPU when GPU unavailable

**For GitHub Coding Agent (no GPU):**
- [ ] `cargo build -p fire-sim-core --no-default-features` compiles solver/ (CPU-only)
- [ ] `cargo test -p fire-sim-core --no-default-features solver::` passes all CPU solver tests

**For Local Validation (with GPU):**
- [ ] `cargo build -p fire-sim-core --features gpu` compiles solver/ (GPU build)
- [ ] `cargo test -p fire-sim-core solver::` passes all solver tests
- [ ] Unit test: `GpuContext::new()` returns appropriate `GpuInitResult` variant

**Note:** The `GpuContext::new()` test should be marked `#[cfg(feature = "gpu")]` so it compiles out in no-GPU mode.

**IGNORE errors in simulation/mod.rs and files referencing FuelElement - these are deleted in Phase 4**

### Phase 1 Code Details

See **Step 1** in the detailed implementation section below for full code specifications.

---

## PHASE 2: Heat Transfer & Combustion Physics

**Objective:** Implement core heat physics on both GPU (WGSL shaders) and CPU (Rayon parallel).

**Branch Name:** `feature/gpu-fire-phase2-heat-physics`

**Estimated Time:** 6-8 days

**Depends On:** Phase 1

### Phase 2 Deliverables

- [ ] Heat transfer compute shader (`heat_transfer.wgsl`)
- [ ] Heat transfer CPU implementation (Rayon parallel)
- [ ] Combustion compute shader (`combustion.wgsl`)
- [ ] Combustion CPU implementation
- [ ] Moisture evaporation physics (2260 kJ/kg latent heat FIRST)
- [ ] Fuel lookup table (GPU uniform buffer / CPU Vec)
- [ ] Unit tests validating physics accuracy

### Phase 2 Files to Create

```
crates/core/src/solver/
├── shaders/
│   ├── heat_transfer.wgsl   # Stefan-Boltzmann + convection + wind advection
│   └── combustion.wgsl      # Fuel consumption, heat release, O₂ depletion
├── heat_transfer.rs         # CPU + GPU wrapper, pipeline creation
└── combustion.rs            # CPU + GPU wrapper, pipeline creation
```

### Phase 2 Validation Criteria

**⚠️ Phase 1-3: Partial build expected - old simulation code will have errors**

Validate the NEW solver/ module only:
- [ ] Unit test: Hot spot cools via radiation in still air (exponential decay)
- [ ] Unit test: Wind advection pushes heat in correct direction
- [ ] Unit test: Stefan-Boltzmann formula produces correct heat flux (compare to analytical)
- [ ] Unit test: Wet fuel absorbs heat for evaporation before temperature rises
- [ ] Unit test: Combustion heat release matches fuel.heat_content × mass_consumed

**For GitHub Coding Agent (no GPU):**
- [ ] `cargo test -p fire-sim-core --no-default-features solver::heat` passes
- [ ] `cargo test -p fire-sim-core --no-default-features solver::combustion` passes

**For Local Validation (with GPU):**
- [ ] `cargo test -p fire-sim-core solver::heat` passes
- [ ] `cargo test -p fire-sim-core solver::combustion` passes
- [ ] Unit test: CPU and GPU produce identical results (within floating-point tolerance)

**Note:** Mark GPU/CPU comparison tests with `#[cfg(feature = "gpu")]` so they compile out in no-GPU mode.

**IGNORE errors in simulation/mod.rs - deleted in Phase 4**

### Phase 2 Code Details

See **Steps 2-3** in the detailed implementation section below.

---

## PHASE 3: Level Set Fire Front Tracking

**Objective:** Implement level set φ evolution with curvature-dependent spread for realistic jagged fire fronts.

**Branch Name:** `feature/gpu-fire-phase3-level-set`

**Estimated Time:** 5-7 days

**Depends On:** Phase 2

### Phase 3 Deliverables

- [ ] Level set evolution compute shader (`level_set.wgsl`)
- [ ] Level set CPU implementation with Godunov upwind scheme
- [ ] Curvature calculation (κ) with proper numerical stability
- [ ] Curvature-dependent spread rate: `R_effective = R_base × (1 + 0.25 × κ)`
- [ ] Signed distance reinitialization (periodic, every ~10 steps)
- [ ] Spread rate computation from temperature gradient
- [ ] Stochastic noise for run-to-run variation

### Phase 3 Files to Create

```
crates/core/src/solver/
├── shaders/
│   ├── level_set.wgsl        # φ evolution with curvature
│   ├── reinitialize.wgsl     # Signed distance restoration
│   └── spread_rate.wgsl      # R from temperature gradient
└── level_set.rs              # CPU + GPU wrapper
```

### Phase 3 Validation Criteria

**⚠️ Phase 1-3: Partial build expected - old simulation code will have errors**

Validate the NEW solver/ module only:
- [ ] Unit test: Circular fire expands at uniform rate when κ_coeff=0
- [ ] Unit test: Curvature κ > 0 for convex regions, κ < 0 for concave
- [ ] Unit test: Fire front develops fingers when κ_coeff=0.25
- [ ] Unit test: Reinitialization maintains |∇φ| ≈ 1
- [ ] Visual inspection: Fire front shows jagged perimeter (compare to Boddington)

**For GitHub Coding Agent (no GPU):**
- [ ] `cargo test -p fire-sim-core --no-default-features solver::level_set` passes

**For Local Validation (with GPU):**
- [ ] `cargo test -p fire-sim-core solver::level_set` passes
- [ ] Unit test: CPU and GPU produce identical level set evolution

**Note:** Mark GPU/CPU comparison tests with `#[cfg(feature = "gpu")]` so they compile out in no-GPU mode.

**IGNORE errors in simulation/mod.rs - deleted in Phase 4**

### Phase 3 Code Details

See **Step 4** in the detailed implementation section below.

---

## PHASE 4: Simulation Integration & Ignition

**Objective:** Create the new `FireSimulation` struct that orchestrates all GPU/CPU passes.

**Branch Name:** `feature/gpu-fire-phase4-simulation`

**Estimated Time:** 5-7 days

**Depends On:** Phase 3

### Phase 4 Deliverables

- [ ] New `FireSimulation` struct using `FieldSolver` trait
- [ ] Ignition sync compute shader (T > T_ign → update φ)
- [ ] Ignition sync CPU implementation
- [ ] Main update loop orchestrating all passes
- [ ] `ignite_at(position, radius)` method
- [ ] Fire front extraction via marching squares
- [ ] `FireFront` struct with vertices, normals, intensities, curvatures
- [ ] Statistics tracking (burned area, fuel consumed, etc.)

### Phase 4 Files to Create/Modify

```
crates/core/src/
├── solver/
│   ├── shaders/
│   │   └── ignition_sync.wgsl
│   └── marching_squares.rs    # Contour extraction
├── simulation/
│   └── mod.rs                 # REPLACE existing with new FireSimulation
```

### Phase 4 Files to Delete

- [ ] `crates/core/src/physics/element_heat_transfer.rs`
- [ ] `crates/core/src/core_types/element.rs`
- [ ] `crates/core/src/core_types/spatial.rs`

### Phase 4 Validation Criteria

**✅ Phase 4+: Full build must succeed - old code is deleted, new system wired up**

This is the critical phase where the old and new systems are swapped:
- [ ] Delete old files: `element_heat_transfer.rs`, `element.rs`, `spatial.rs`
- [ ] Replace `simulation/mod.rs` with new `FireSimulation` using `FieldSolver`

**For GitHub Coding Agent (no GPU):**
- [ ] `cargo build --all-targets --no-default-features` succeeds
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings
- [ ] `cargo test --no-default-features` passes

**For Local Validation (with GPU):**
- [ ] `cargo build --all-targets --all-features` succeeds
- [ ] `cargo clippy --all-targets --all-features` passes with ZERO warnings
- [ ] `cargo test --all-features` passes

**Common:**
- [ ] `cargo fmt --all --check` passes
- [ ] Integration test: Ignition at point spreads fire outward
- [ ] Integration test: Fire stops at fuel boundaries (fuel_load = 0)
- [ ] Integration test: Wind affects spread direction (head fire faster)
- [ ] Integration test: Fire front extraction produces valid polyline
- [ ] Unit test: Ignition sync only ignites cells adjacent to burning cells
- [ ] All preserved tests pass (tests not using deleted FuelElement code)

### Phase 4 Code Details

See **Steps 5-6** in the detailed implementation section below.

---

## PHASE 5: Ember, Weather & Wind Coupling

**Objective:** Integrate ember spotting, weather system, and enhance fire-atmosphere coupling.

**Branch Name:** `feature/gpu-fire-phase5-ember-weather`

**Estimated Time:** 4-5 days

**Depends On:** Phase 4

### Phase 5 Deliverables

- [ ] Ember generation from fire front (using existing Albini physics)
- [ ] Ember trajectory updates connected to new system
- [ ] Spot fire ignition from landed embers
- [ ] Weather system integration (unchanged API)
- [ ] Enhanced fire-induced wind modification (see below)
- [ ] Suppression action integration

### Phase 5 Weather/Wind Enhancements

The current weather system is comprehensive. However, **fire-atmosphere coupling** can be enhanced:

**Current State:**
- Mass-consistent wind field with plume coupling exists
- Atmospheric turbulence scales with FFDI, stability, mixing height
- Byram's convection column for plume-induced updrafts

**Enhancement - Fire-Induced Wind Feedback:**

The fire's heat release should modify the local wind field more dynamically:

```rust
// In wind field update, scale entrainment based on actual fire intensity
impl WindField {
    pub fn update_with_fire_intensity(&mut self, fire_intensity_field: &[f32], ...) {
        // For cells near active fire, add entrainment wind toward fire
        // This creates the "wind rushing toward fire" effect observed in large fires
        
        // Entrainment velocity (Beer 1991):
        // v_entrain = 0.1 × (I / (ρ × cp × ΔT))^(1/3)
        // where I is local fire intensity (kW/m)
    }
}
```

This enhancement is **optional** for Phase 5 but recommended for realism.

### Phase 5 Validation Criteria

- [ ] Integration test: Embers generated at fire front
- [ ] Integration test: Spot fires ignite at correct locations
- [ ] Integration test: Weather changes affect fire behavior
- [ ] Unit test: Suppression actions reduce fire spread in target area
- [ ] Albini spotting distances still match validation data

**For GitHub Coding Agent (no GPU):**
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings
- [ ] `cargo test --no-default-features` passes

**For Local Validation (with GPU):**
- [ ] `cargo clippy --all-targets --all-features` passes with ZERO warnings
- [ ] `cargo test --all-features` passes

### Phase 5 Code Details

See **Step 7** in the detailed implementation section below.

---

## PHASE 6: FFI, Performance & Validation

**Objective:** Update external API, optimize performance, and comprehensive validation.

**Branch Name:** `feature/gpu-fire-phase6-ffi-validation`

**Estimated Time:** 4-6 days

**Depends On:** Phase 5

### Phase 6 Deliverables

- [ ] FFI updated for new FireSimulation API
- [ ] Fire front vertex export for game engines
- [ ] Performance profiling and optimization
- [ ] Narrow band optimization (optional but recommended)
- [ ] Dynamic quality scaling (optional)
- [ ] Comprehensive validation tests
- [ ] Updated documentation

### Phase 6 Files to Modify

```
crates/ffi/
├── src/
│   ├── lib.rs       # Update for new API
│   ├── queries.rs   # Add fire front queries
│   └── simulation.rs
└── FireSimFFI.h     # Update C header

docs/
└── validation/
    └── gpu_fire_system.md   # New validation doc
```

### Phase 6 Performance Targets

| Metric | Target (GPU) | Target (CPU) |
|--------|--------------|--------------|
| Total frame time @ 2048² | <8ms | <50ms |
| Heat transfer pass | <2ms | <15ms |
| Level set pass | <2ms | <15ms |
| Fire front extraction | <1ms | <5ms |
| Memory usage | <256MB GPU | <512MB RAM |

### Phase 6 Validation Criteria

**Performance (GPU only - validated locally, not by Coding Agent):**
- [ ] GPU <8ms @ 2048² on GTX 1660

**Performance (CPU - can be validated by Coding Agent):**
- [ ] CPU <50ms @ 2048² on i5-12400

**Physics (all backends):**
- [ ] Stefan-Boltzmann matches analytical solution
- [ ] Heat diffusion matches 1D analytical solution

**Behavior (all backends):**
- [ ] Fire spreads faster downwind (ratio ~3-5×)
- [ ] Fire spreads faster uphill (ratio ~2-3×)
- [ ] Fire fronts match Boddington-style jagged perimeters

**Common:**
- [ ] Regression: All preserved tests pass
- [ ] FFI: Header compiles with C compiler
- [ ] `cargo fmt --all --check` passes

**For GitHub Coding Agent (no GPU):**
- [ ] `cargo clippy --all-targets --no-default-features` passes with ZERO warnings
- [ ] `cargo test --no-default-features` passes

**For Local Validation (with GPU):**
- [ ] `cargo clippy --all-targets --all-features` passes with ZERO warnings
- [ ] `cargo test --all-features` passes

---

# FOLLOW-UP ENHANCEMENTS (Separate Tasks)

The following enhancements are **not required** for the core GPU fire system but would improve realism. They are documented in separate task files and should be implemented after this task is complete.

## Weather & Fire-Atmosphere Coupling

**See:** [WEATHER_FIRE_ATMOSPHERE_COUPLING.md](./WEATHER_FIRE_ATMOSPHERE_COUPLING.md)

Includes:
- Fire-induced wind feedback (entrainment toward fire)
- Dynamic wind variability (gusts, direction meandering)
- Pressure system dynamics (cold fronts)
- Fire whirl detection

**Current weather system status:** ✅ Comprehensive - no changes required for core GPU fire system.

## Other Potential Enhancements (Not Yet Documented)

| Enhancement | Priority | Description |
|-------------|----------|-------------|
| Fuel Heterogeneity | Medium | Sub-grid fuel variation, noise on fuel properties |
| Crown Fire Transition | Medium | Surface → passive crown → active crown states |
| Pyroconvection Dynamics | Low | Plume height, pyroCb, downdrafts (requires 3D) |

These may be documented as separate tasks in the future.

---

## Completion Checklist

### Phase Completion

- [ ] **Phase 1:** Core Infrastructure & Field Solver
  - Branch: `feature/gpu-fire-phase1-infrastructure`
  - PR: #___
  - Merged: _______________

- [ ] **Phase 2:** Heat Transfer & Combustion Physics
  - Branch: `feature/gpu-fire-phase2-heat-physics`
  - PR: #___
  - Merged: _______________

- [ ] **Phase 3:** Level Set Fire Front Tracking
  - Branch: `feature/gpu-fire-phase3-level-set`
  - PR: #___
  - Merged: _______________

- [ ] **Phase 4:** Simulation Integration & Ignition
  - Branch: `feature/gpu-fire-phase4-simulation`
  - PR: #___
  - Merged: _______________

- [ ] **Phase 5:** Ember, Weather & Wind Coupling
  - Branch: `feature/gpu-fire-phase5-ember-weather`
  - PR: #___
  - Merged: _______________

- [ ] **Phase 6:** FFI, Performance & Validation
  - Branch: `feature/gpu-fire-phase6-ffi-validation`
  - PR: #___
  - Merged: _______________

### Final Verification

- [ ] All implementation phases complete
- [ ] All old element-based code deleted
- [ ] All code quality checks pass (`clippy`, `fmt`)
- [ ] All tests pass (new tests + preserved existing tests)
- [ ] Performance targets met on reference hardware
- [ ] Fire fronts visually match reference images (jagged, realistic)
- [ ] Documentation updated
- [ ] FFI header updated

**System Complete Date:** _______________  
**Verified By:** _______________
