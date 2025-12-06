use fire_sim_core::{FireSimulation, TerrainData};

/// Terrain configuration for the fire simulation.
/// Defines the shape and parameters of the simulated landscape.
#[repr(C)]
pub enum Terrain {
    /// Flat terrain with specified width and height in meters.
    Flat { width: f32, height: f32 },

    /// Single hill terrain.
    ///
    /// width, height, resolution, base_elevation, hill_height, hill_radius
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
    /// width, height, resolution, base_elevation, hill_height
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

/// The main fire simulation context.
/// Holds the internal simulation state and manages fire behavior calculations.
pub struct FireSim {
    sim: FireSimulation,
}

impl FireSim {
    /// Creates a new FireSim instance with the specified terrain.
    pub fn new(terrain: Terrain) -> Box<Self> {
        let terrain = match terrain {
            Terrain::Flat { width, height } => TerrainData::flat(width, height, 5.0, 0.0),

            Terrain::SingleHill {
                width,
                height,
                resolution,
                base_elevation,
                hill_height,
                hill_radius,
            } => TerrainData::single_hill(
                width,
                height,
                resolution,
                base_elevation,
                hill_height,
                hill_radius,
            ),

            Terrain::ValleyBetweenHills {
                width,
                height,
                resolution,
                base_elevation,
                hill_height,
            } => TerrainData::valley_between_hills(width, height, resolution, base_elevation, hill_height),

            Terrain::FromHeightmap {
                width,
                height,
                heightmap_ptr,
                nx,
                ny,
                elevation_scale,
                base_elevation,
            } => {
                // Safety: we accept a raw pointer from the caller. Convert to a slice and copy into a Vec.
                // If the pointer is null or the expected size is zero, fall back to a flat terrain at base_elevation.
                let len = nx.checked_mul(ny).unwrap_or(0);
                if heightmap_ptr.is_null() || len == 0 {
                    TerrainData::flat(width, height, 1.0, base_elevation)
                } else {
                    let slice = unsafe { std::slice::from_raw_parts(heightmap_ptr, len) };
                    TerrainData::from_heightmap(width, height, slice.to_vec(), nx, ny, elevation_scale, base_elevation)
                }
            }
        };

        let sim = FireSimulation::new(5.0, terrain);

        Box::new(Self { sim })
    }
}

#[no_mangle]
/// Creates a new fire simulation instance and returns a pointer to it.
/// Returns a null pointer if allocation fails.
pub extern "C" fn fire_sim_new(terrain: Terrain) -> *mut FireSim {
    Box::into_raw(FireSim::new(terrain))
}
