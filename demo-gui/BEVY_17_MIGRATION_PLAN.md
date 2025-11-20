# Bevy 0.17 + bevy-egui Migration Plan

## Overview

This document outlines the complete plan to migrate demo-gui from Bevy 0.14 to Bevy 0.17 and replace the custom menu system with bevy-egui.

## Scope

**Current State:**
- Bevy 0.14.0
- Custom scrollable menu system (550+ lines)
- 1471 total lines in main.rs
- All features working: menu, FPS, 3D rendering, tooltips, stats panel

**Target State:**
- Bevy 0.17.3
- bevy-egui 0.38.0 for menu
- All existing features preserved
- Improved menu UX with native egui controls

## Estimated Effort

**Total: 12-16 hours** of focused development time

### Phase 1: Dependencies (30 min)
- Update Cargo.toml to Bevy 0.17.3
- Add bevy_egui = "0.38"
- Remove zstd feature (not in 0.17)
- Test dependency resolution

### Phase 2: Core API Migration (6-8 hours)
**Text System Changes (~2 hours):**
- 25+ occurrences of `TextBundle::from_section` → New Text API
- `TextSection` → `Text::new()` / `Text::from_section()`
- `text.sections[0].value` → Direct text manipulation

**Query Method Changes (~1 hour):**
- `query.get_single()` → `query.single()` (returns Result)
- `query.get_single_mut()` → `query.single_mut()`
- Add proper `Ok()` / error handling

**Camera/Transform Changes (~2 hours):**
- `Camera3dBundle` → `Camera3d` component
- `Camera2dBundle` → `Camera2d` component
- Update viewport_to_world calls (returns Result)
- Fix Transform access patterns

**UI System Changes (~1-2 hours):**
- `NodeBundle` → Update style properties
- `BackgroundColor` → Color API changes  
- `bevy::ui::Style` imports
- Button interaction patterns

**Time API (~30 min):**
- `time.delta_seconds()` → `time.delta_secs()`
- Update all timing code

### Phase 3: Replace Menu with bevy-egui (3-4 hours)

**Remove Custom Menu (1 hour):**
- Delete 550+ lines of custom menu code:
  - `setup_menu()` function
  - `handle_menu_interactions()` system
  - `update_config_display()` system
  - `handle_menu_scroll()` system
  - All menu UI components and button enums

**Implement egui Menu (2-3 hours):**
```rust
fn render_menu_ui(
    mut contexts: EguiContexts,
    mut config: ResMut<DemoConfig>,
    mut menu_state: ResMut<MenuState>,
) {
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.heading("Fire Simulation Configuration");
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Terrain Settings
            ui.group(|ui| {
                ui.heading("Terrain");
                ui.add(egui::Slider::new(&mut config.map_width, 50.0..=500.0));
                ui.add(egui::Slider::new(&mut config.map_height, 50.0..=500.0));
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    if ui.button(config.terrain_type.name()).clicked() {
                        config.terrain_type = config.terrain_type.cycle();
                    }
                });
            });
            
            // Fire Settings
            ui.group(|ui| {
                ui.heading("Fire");
                ui.add(egui::Slider::new(&mut config.elements_x, 5..=20));
                ui.add(egui::Slider::new(&mut config.elements_y, 5..=20));
                ui.add(egui::Slider::new(&mut config.fuel_mass, 1.0..=20.0));
                ui.horizontal(|ui| {
                    ui.label("Fuel Type:");
                    if ui.button(config.fuel_type.name()).clicked() {
                        config.fuel_type = config.fuel_type.cycle();
                    }
                });
                ui.add(egui::Slider::new(&mut config.initial_ignitions, 1..=20));
                ui.add(egui::Slider::new(&mut config.spacing, 5.0..=15.0));
            });
            
            // Weather Settings
            ui.group(|ui| {
                ui.heading("Weather");
                ui.add(egui::Slider::new(&mut config.temperature, 10.0..=50.0));
                ui.add(egui::Slider::new(&mut config.humidity, 0.05..=0.60));
                ui.add(egui::Slider::new(&mut config.wind_speed, 0.0..=40.0));
                ui.add(egui::Slider::new(&mut config.wind_direction, 0.0..=360.0));
                ui.add(egui::Slider::new(&mut config.drought_factor, 1.0..=20.0));
            });
            
            ui.add_space(20.0);
            
            if ui.add(egui::Button::new("START SIMULATION").min_size(egui::vec2(200.0, 40.0))).clicked() {
                menu_state.start_simulation = true;
            }
        });
    });
}
```

**Benefits:**
- Native egui sliders with better UX
- Automatic scrolling
- Professional styling out-of-the-box
- Less code to maintain (200 lines vs 550 lines)
- Built-in keyboard navigation
- Better responsiveness

### Phase 4: Testing & Validation (2-3 hours)

**Compilation Testing (1 hour):**
- Fix all compiler errors
- Resolve all warnings
- Ensure clean build

**Functional Testing (1-2 hours):**
- ✅ Menu displays correctly
- ✅ All config parameters adjustable
- ✅ START button works
- ✅ Simulation initializes properly
- ✅ 3D rendering works
- ✅ Camera controls responsive
- ✅ FPS counter displays
- ✅ Stats panel updates
- ✅ Tooltips show on hover
- ✅ All keyboard controls work
- ✅ Reset returns to menu
- ✅ Water suppression functions

**Performance Testing (30 min):**
- Frame rate consistency
- Menu responsiveness
- Memory usage
- Simulation step timing

### Phase 5: Documentation (1 hour)

**Update Documentation:**
- README.md - New Bevy version and egui menu
- USAGE.md - Updated menu screenshots/description
- UI_FEATURES.md - egui integration details
- BEVY_UPGRADE.md - Mark as complete

**Add New Docs:**
- EGUI_MENU.md - egui menu customization guide
- Migration notes for future reference

## Implementation Strategy

### Recommended Approach: Incremental Migration

**Step 1:** Create working branch from current state
**Step 2:** Update dependencies, test resolution
**Step 3:** Migrate core APIs one subsystem at a time:
  - Text system first
  - Query methods second
  - Camera/Transform third
  - UI components fourth
  - Time API last
**Step 4:** Replace menu with egui
**Step 5:** Test after each subsystem
**Step 6:** Final integration testing
**Step 7:** Documentation updates

### Alternative Approach: Complete Rewrite

Create new main_v17.rs with all changes:
- Copy working structure
- Apply all API changes at once
- Replace menu system
- Test as complete unit
- Replace main.rs when working

## Risks & Mitigation

**Risk 1: Breaking Changes**
- Mitigation: Keep Bevy 0.14 backup (main_v14_backup.rs)
- Test incrementally
- Document all changes

**Risk 2: egui Integration Issues**
- Mitigation: Test egui basics first
- Reference bevy-egui examples
- Keep custom menu as backup option

**Risk 3: Performance Regression**
- Mitigation: Profile before and after
- Benchmark key operations
- Monitor frame rates during testing

**Risk 4: Feature Loss**
- Mitigation: Create comprehensive test checklist
- Verify each feature during testing phase
- Compare screenshots before/after

## Dependencies

```toml
[dependencies]
bevy = { version = "0.17", default-features = false, features = [
    "animation",
    "bevy_asset",
    "bevy_scene",
    "bevy_winit",
    "bevy_core_pipeline",
    "bevy_pbr",
    "bevy_gltf",
    "bevy_render",
    "bevy_sprite",
    "bevy_text",
    "bevy_ui",
    "multi_threaded",
    "png",
    "hdr",
    "x11",
    "bevy_gizmos",
    "tonemapping_luts",
    "default_font",
] }

bevy_egui = "0.38"
```

Note: `zstd` feature removed (not available in Bevy 0.17)

## Success Criteria

- ✅ All 55 core tests pass
- ✅ Demo compiles without errors/warnings
- ✅ Menu uses egui (not custom UI)
- ✅ All 13 config parameters editable
- ✅ Simulation runs at 60 FPS
- ✅ All features from Bevy 0.14 version work
- ✅ Documentation updated
- ✅ No performance regressions

## Timeline

- **Day 1 (4-6 hours):** Dependencies + Core API migration
- **Day 2 (4-6 hours):** egui menu + Testing
- **Day 3 (2-3 hours):** Documentation + Polish

## Next Steps

1. Create feature branch `bevy-0.17-migration`
2. Update Cargo.toml dependencies
3. Begin Phase 2 (Core API Migration)
4. Commit after each working subsystem
5. Document any unexpected issues
6. Report progress regularly

## References

- [Bevy 0.17 Release Notes](https://bevyengine.org/news/bevy-0-17/)
- [bevy-egui Documentation](https://docs.rs/bevy_egui/)
- [Bevy 0.14 → 0.17 Migration Guide](https://bevyengine.org/learn/migration-guides/)
- BEVY_UPGRADE.md (this repo)
