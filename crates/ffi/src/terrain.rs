/// Terrain configuration for the fire simulation.
/// Defines the shape and parameters of the simulated landscape.
#[repr(C)]
pub enum Terrain {
    /// Flat terrain with specified width and height in meters.
    Flat { width: f32, height: f32 },

    /// Single hill terrain.
    ///
    /// width, height, resolution, `base_elevation`, `hill_height`, `hill_radius`
    SingleHill {
        width: f32,
        height: f32,
        resolution: f32,
        base_elevation: f32,
        hill_height: f32,
        hill_radius: f32,
    },

    /// Valley between two hills.
    ///
    /// width, height, resolution, `base_elevation`, `hill_height`
    ValleyBetweenHills {
        width: f32,
        height: f32,
        resolution: f32,
        base_elevation: f32,
        hill_height: f32,
    },

    /// Create terrain from a heightmap.
    ///
    /// The heightmap pointer should point to nx*ny f32 values in row-major order.
    FromHeightmap {
        width: f32,
        height: f32,
        heightmap_ptr: *const f32,
        nx: usize,
        ny: usize,
        elevation_scale: f32,
        base_elevation: f32,
    },
}
