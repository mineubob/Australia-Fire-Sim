# 3D Terrain Visualization

## Overview

The demo-gui now features **true 3D terrain** that follows the actual elevation data used in the physics simulation.

## Before vs After

### Before: Flat Plane
- Simple flat rectangular mesh at y=0
- No elevation data
- Fuel elements positioned at correct heights, but terrain was flat

### After: 3D Terrain Mesh
- Mesh generated from actual TerrainData elevation samples
- Vertices positioned at real elevation values
- Proper vertex normals for realistic lighting
- Samples every 5 meters for smooth appearance
- Matches physics simulation exactly

## Terrain Types Supported

### 1. Flat Terrain (`TerrainType::Flat`)
- All vertices at same elevation
- Used for baseline fire spread testing
- No slope effects

### 2. Single Hill (`TerrainType::Hill`)
- Central hill up to 80m elevation
- Gaussian distribution for smooth slopes
- Fire spreads faster uphill (slope effect)
- Default configuration

### 3. Valley Between Hills (`TerrainType::Valley`)
- Two hills with valley in between
- Complex elevation changes
- Tests fire behavior in varied topography

## Implementation Details

### Mesh Generation Process

1. **Sample Elevation Grid**
   ```
   For each point on 5m grid:
     - Query elevation from TerrainData
     - Store as vertex position (x, elevation, y)
   ```

2. **Generate Triangles**
   ```
   For each grid quad:
     - Create 2 triangles
     - Index vertices for efficient rendering
   ```

3. **Calculate Normals**
   ```
   For each triangle:
     - Calculate face normal from cross product
     - Accumulate at vertices
     - Normalize for smooth shading
   ```

### Visual Result

The terrain mesh:
- Uses Bevy's Y-up coordinate system
- Maps simulation (x, y, elevation) to Bevy (x, elevation, y)
- Brown material with 90% roughness for realistic ground appearance
- Receives shadows from directional light
- Smooth shading from averaged normals

### Physics Integration

The terrain mesh is purely visual - it doesn't affect physics. However:
- Fuel elements are positioned at `elevation_at(x, y) + 0.5m`
- Fire spread uses slope from terrain data
- Uphill spread is exponentially faster
- Visual and physics use **identical elevation data**

## Configuration

To change terrain type, edit `main()`:

```rust
let config = DemoConfig {
    terrain_type: TerrainType::Hill,  // or Flat, Valley
    map_width: 200.0,
    map_height: 200.0,
    // ... other settings
};
```

## Performance

- Terrain mesh is static (generated once at startup)
- ~1,640 vertices for 200x200m area (5m resolution)
- ~3,200 triangles
- No impact on frame rate
- Efficient indexed rendering

## Future Improvements

Potential enhancements:
- [ ] Texture mapping with detail based on slope
- [ ] Dynamic LOD for large terrains
- [ ] Height-based coloring (green valleys, brown peaks)
- [ ] Terrain shadows from fire light sources
- [ ] Interactive terrain editing
