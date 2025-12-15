# Fire Simulation FFI

C-compatible Foreign Function Interface (FFI) for the Bushfire Simulation engine.

## Overview

This is a WIP crate that provides a C API for integrating the Rust fire simulation core into game engines and other C/C++ applications like Unreal Engine and Godot.

## Automatic C Header Generation

The C header file (`FireSimFFI.h`) is **automatically generated** during compilation via `build.rs` using [cbindgen](https://github.com/mozilla/cbindgen).

### Building

```bash
# Build the FFI library (automatically generates FireSimFFI.h in crate directory `crates/ffi`)
cargo build --release -p fire-sim-ffi

# Output files:
# - Linux: target/release/libfire_sim_ffi.so
# - Windows: target/release/fire_sim_ffi.dll
# - macOS: target/release/libfire_sim_ffi.dylib
# - Header: FireSimFFI.h (crate root of this crate)
```

> Note: `FireSimFFI.h` is generated automatically by the build (via `cbindgen` in `build.rs`) and is ignored by git (see `.gitignore`). If you must ship the header to a consumer who cannot run cbindgen, add it explicitly with `git add -f crates/ffi/FireSimFFI.h`.

## Scientific Accuracy

This FFI exposes the full scientific fire simulation including:
- Rothermel (1972) surface fire spread
- Van Wagner (1977) crown fire transitions
- Albini (1979, 1983) ember spotting physics
- Nelson (2000) fuel moisture dynamics
- McArthur FFDI fire danger rating

See the core crate documentation for details on the physics models.
