//! FFI-exposed terrain configuration types.
//!
//! This module defines the `Terrain` enum which is part of the public FFI API.
//! It allows game engines to specify different terrain configurations when creating
//! a fire simulation instance.

/// Terrain configuration for the fire simulation.
/// Defines the shape and parameters of the simulated landscape.
#[repr(C)]
pub enum Terrain {
    /// Flat terrain with specified width and height in meters.
    Flat {
        /// Width of the terrain in meters.
        width: f32,
        /// Height of the terrain in meters.
        height: f32,
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
