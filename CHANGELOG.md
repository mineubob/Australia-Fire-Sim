# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

- BREAKING: The simulation now *always* initializes and uses the advanced 3D mass-consistent wind field (Sherman 1978).
  - `FireSimulation::wind_field` is no longer optional and is initialized during `FireSimulation::new`.
  - Removed API methods: `enable_wind_field`, `enable_wind_field_default`, `has_wind_field`, `disable_wind_field`.
  - The old fallback helper `update_wind_field` (simple terrain-modulated wind) has been removed.
  - The demo and docs updated to remove runtime/startup toggle for the wind field.

Reason: the wind field is a core component of the simulation phases and is now required for correct behavior; the crate is not public yet so this breaking change is acceptable.

Files / areas touched:
- crates/core/src/simulation/mod.rs — WindField made non-optional and construction/init updated
- crates/core/src/core_types/atmospheric.rs — removed legacy update helper
- demo-interactive/src/main.rs — removed user-facing wind toggle calls and help text
- .github/copilot-instructions.md — updated documentation/examples

## 0.1.0 - Previous

- Initial project layout and earlier features.
