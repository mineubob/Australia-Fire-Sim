//! FFI-exposed terrain configuration types.
//!
//! This module defines the `Terrain` enum which is part of the public FFI API.
//! It allows game engines to specify different terrain configurations when creating
//! a fire simulation instance.

/// Terrain configuration for the fire simulation.
///
/// Defines the shape and parameters of the simulated landscape. This enum is FFI-safe
/// with a stable C-compatible memory layout (`#[repr(C)]`), allowing construction and
/// use from C/C++/C# code.
///
/// # FFI Usage
///
/// When calling from C/C++, you must construct this enum carefully to match Rust's
/// tagged union representation. Each variant has a discriminant (tag) followed by its fields.
/// Use the appropriate language binding or FFI helper library to ensure correct memory layout.
///
/// Example (Conceptual):
///
/// In C using the generated bindings (for example, from `cbindgen`), you would:
/// ```c
/// Terrain terrain;
/// memset(&terrain, 0, sizeof(Terrain));
///
/// // Set the tag/discriminant field to select the flat-terrain variant.
/// // Then assign the associated fields (width, height, resolution,
/// // base elevation, etc.) using the names and types defined in
/// // the generated FireSimFFI.h header.
/// ```
///
/// The exact struct, union, and field names are defined in the generated
/// `FireSimFFI.h` (or your binding generator's output). Always consult that
/// header to match the precise C layout for this enum.
#[repr(C)]
pub enum Terrain {
    /// Flat terrain with specified width and height in meters.
    Flat {
        /// Width of the terrain in meters.
        width: f32,
        /// Height of the terrain in meters.
        height: f32,
        /// Grid resolution in meters (cell size).
        resolution: f32,
        /// Base elevation of the terrain in meters.
        base_elevation: f32,
    },

    /// Single hill terrain.
    SingleHill {
        /// Width of the terrain in meters.
        width: f32,
        /// Height of the terrain in meters.
        height: f32,
        /// Grid resolution in meters (cell size).
        resolution: f32,
        /// Base elevation of the terrain in meters.
        base_elevation: f32,
        /// Height of the hill above base elevation in meters.
        hill_height: f32,
        /// Radius of the hill in meters.
        hill_radius: f32,
    },

    /// Valley between two hills.
    ValleyBetweenHills {
        /// Width of the terrain in meters.
        width: f32,
        /// Height of the terrain in meters.
        height: f32,
        /// Grid resolution in meters (cell size).
        resolution: f32,
        /// Base elevation of the valley floor in meters.
        base_elevation: f32,
        /// Height of the hills above base elevation in meters.
        hill_height: f32,
    },

    /// Create terrain from a heightmap.
    ///
    /// The heightmap pointer should point to nx*ny f32 values in row-major order.
    FromHeightmap {
        /// Width of the terrain in meters.
        width: f32,
        /// Height of the terrain in meters.
        height: f32,
        /// Pointer to heightmap data (nx*ny f32 values in row-major order).
        heightmap_ptr: *const f32,
        /// Number of columns in the heightmap grid.
        nx: usize,
        /// Number of rows in the heightmap grid.
        ny: usize,
        /// Scale factor to convert heightmap values to meters.
        elevation_scale: f32,
        /// Base elevation to add to all heightmap values in meters.
        base_elevation: f32,
    },
}
