# Bevy 0.17 Upgrade Guide

## Current Status
- **Current Version**: Bevy 0.14
- **Target Version**: Bevy 0.17.3
- **Status**: Blocked by 42+ breaking API changes

## Required Changes

### 1. Text System (12+ occurrences)
**0.14 API:**
```rust
TextBundle::from_section("text", TextStyle { .. })
text.sections[0].value = "new text";
```

**0.17 API:**
```rust
Text::new("text")
text.0 = "new text"; // Text is now a tuple struct
```

### 2. Query Methods (5+ occurrences)
**0.14 API:**
```rust
query.get_single()
query.iter()
```

**0.17 API:**
```rust
query.single()  // Returns Result instead of Option
query.iter()    // Trait bounds changed
```

### 3. Transform Access (10+ occurrences)
**0.14 API:**
```rust
let mut transform = camera_query.get_single_mut();
transform.translation += vec;
```

**0.17 API:**
```rust
let Ok(mut transform) = camera_query.single_mut() else { return };
transform.translation += vec;
```

### 4. Style/TextStyle Components
**0.14 API:**
```rust
use bevy::prelude::*; // Style is in prelude
NodeBundle { style: Style { .. }, .. }
```

**0.17 API:**
```rust
use bevy::ui::Style;  // Need explicit import
NodeBundle { style: Style { .. }, .. }
```

### 5. Handle<T> Component Changes
**0.14 API:**
```rust
Query<&Handle<StandardMaterial>>  // Handle is Component
```

**0.17 API:**
```rust
// Handle trait bounds changed
```

### 6. Time API
**0.14 API:**
```rust
time.delta_seconds()
```

**0.17 API:**
```rust
time.delta_secs()  // Renamed method
```

### 7. Camera Viewport
**0.14 API:**
```rust
camera.viewport_to_world(transform, pos)  // Returns Option<Ray>
```

**0.17 API:**
```rust
camera.viewport_to_world(transform, pos)  // Returns Result<Ray3d, Error>
```

### 8. Interaction/Button Changes
**0.14 API:**
```rust
Query<&Interaction, With<Button>>
```

**0.17 API:**
```rust
// Button interaction system changed
```

## Migration Strategy

### Phase 1: Core Systems (Estimated: 4-6 hours)
1. Update all Text/TextBundle usage
2. Fix Query method calls
3. Update Transform access patterns
4. Add Result handling for camera methods

### Phase 2: UI System (Estimated: 2-3 hours)
1. Fix Style imports
2. Update NodeBundle/TextBundle construction
3. Fix button interaction queries
4. Update color/background APIs

### Phase 3: Testing (Estimated: 2-3 hours)
1. Test all UI interactions
2. Verify camera controls
3. Check tooltip system
4. Validate stats display
5. Test menu functionality (if implemented)

### Phase 4: Polish (Estimated: 1-2 hours)
1. Update documentation
2. Fix any remaining warnings
3. Performance testing
4. CI/CD updates

## Total Estimated Time
**10-14 hours** for complete migration and testing

## Recommendation
Implement new features (menu system, FPS display) with Bevy 0.14, then perform Bevy 0.17 upgrade as a dedicated task with proper testing.

## Benefits of Upgrading
- Better performance in UI rendering
- Improved ECS query system
- More ergonomic APIs
- Latest features and bug fixes
- Better compile times

## Risks
- Breaking changes may introduce bugs
- Extensive testing required
- Potential performance regressions
- Documentation may be incomplete

## Decision
**Current**: Stay on Bevy 0.14 for stability
**Future**: Schedule dedicated upgrade task after feature implementation
