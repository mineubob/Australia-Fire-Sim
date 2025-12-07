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
pub struct FireSimState {
    sim: FireSimulation,
}

impl FireSimState {
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
            } => TerrainData::valley_between_hills(
                width,
                height,
                resolution,
                base_elevation,
                hill_height,
            ),

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
                    TerrainData::from_heightmap(
                        width,
                        height,
                        slice.to_vec(),
                        nx,
                        ny,
                        elevation_scale,
                        base_elevation,
                    )
                }
            }
        };

        let sim = FireSimulation::new(5.0, terrain);

        Box::new(Self { sim })
    }
}

/// Create a new FireSim instance and return a raw pointer to it for use across FFI.
///
/// This function allocates a new FireSim on the heap and transfers ownership of the
/// pointer to the caller. The returned pointer must eventually be released by calling
/// `fire_sim_destroy`.
///
/// Parameters
/// - `terrain`: A `Terrain` value describing the desired terrain configuration.
///   - For `Terrain::FromHeightmap`, if `heightmap_ptr` is non-null and `nx * ny > 0`,
///     the heightmap data is read (using `from_raw_parts`) and copied into a Rust-owned
///     Vec. After this call the caller remains free to deallocate the original heightmap
///     memory. If the pointer is null or the size is zero, a flat terrain with the provided
///     base elevation will be used as a fallback.
///
/// Returns
/// - `*mut FireSim` — a raw, heap-allocated pointer owning the newly created FireSim.
///   The pointer is non-null on success (Rust heap allocation failures will abort or panic
///   according to the global allocator behavior).
///
/// Ownership & Safety
/// - The caller takes ownership of the returned pointer and MUST call `fire_sim_destroy`
///   exactly once to avoid a memory leak or a double-free.
/// - The `terrain` value is moved into the Rust function by value; any raw pointers
///   contained in `Terrain::FromHeightmap` are only read and copied if valid. The function
///   does not retain references into caller-owned memory — it owns its data after return.
/// - This function is exposed as `extern "C"` and `#[no_mangle]` for FFI usage. The C-side
///   representation of `Terrain` must match the Rust `#[repr(C)]` layout used here — ensure
///   your foreign-language bindings use a compatible representation when constructing
///   `Terrain` values to pass to this function.
///
/// Example (C-like pseudocode)
/// ```c
/// // Construct a Terrain value in a way compatible with the Rust #[repr(C)] enum,
/// // call `fire_sim_new`, then later call `fire_sim_destroy` to free it.
/// FireSim* sim = fire_sim_new(terrain);
/// // ... use sim via the rest of the FFI API ...
/// fire_sim_destroy(sim);
/// ```
#[no_mangle]
pub extern "C" fn fire_sim_new(terrain: Terrain) -> *mut FireSimState {
    Box::into_raw(FireSimState::new(terrain))
}

/// Destroys a FireSim instance previously created by `fire_sim_new`.
///
/// This function takes a raw pointer returned by `fire_sim_new` and reclaims ownership
/// using `Box::from_raw`, which runs the `FireSim` destructor and frees the allocation.
///
/// Behavior:
/// - If `ptr` is null, this function is a no-op.
/// - If `ptr` points to a `FireSim` allocated by `fire_sim_new` and has not been freed,
///   this function will free it and drop its resources.
///
/// # Safety
/// - The pointer MUST have been created by `fire_sim_new`.
/// - The pointer MUST NOT have been freed already, moved, or otherwise invalidated.
/// - Passing an invalid pointer (e.g., not originating from `fire_sim_new`, a stale pointer,
///   or a pointer into stack memory) is undefined behavior and can cause memory corruption
///   or crashes.
/// - After calling this function, the caller must not use the pointer again (double-free or use-after-free).
///
/// FFI notes:
/// - This is an `extern "C"` (no_mangle) function intended for use across language boundaries.
/// - It is safe to call from C/C++/other languages provided the pointer contract above is respected.
#[no_mangle]
pub unsafe extern "C" fn fire_sim_destroy(ptr: *mut FireSimState) {
    if ptr.is_null() {
        return;
    }

    // SAFETY: The pointer must have been created by `Box::into_raw` in `fire_sim_new`
    // and not freed or moved elsewhere. We check for null above to avoid UB from
    // dereferencing a null pointer. Converting back with `Box::from_raw` reclaims
    // ownership and will run the destructor for `FireSim` when the Box is dropped.
    unsafe {
        // Recreate the Box and immediately drop it to free the allocation.
        drop(Box::from_raw(ptr));
    }
}

/// If `ptr` is non-null, call `f` with a `&mut FireSim` and return `Some` with the closure result.
/// Returns `None` when `ptr` is null.
///
/// Safety note: the caller must ensure the pointer originates from `fire_sim_new` and is not dangling
/// or concurrently aliased mutably elsewhere.
#[inline]
fn with_fire_sim_mut<R, F>(ptr: *mut FireSimState, f: F) -> Option<R>
where
    F: FnOnce(&mut FireSimState) -> R,
{
    if ptr.is_null() {
        return None;
    }

    // SAFETY: caller contract guarantees pointer validity.
    unsafe { Some(f(&mut *ptr)) }
}

/// If `ptr` is non-null, call `f` with a shared `&FireSim` and return `Some` with the closure result.
/// Returns `None` when `ptr` is null.
///
/// Safety note: the caller must ensure the pointer originates from `fire_sim_new` and is not dangling.
#[inline]
fn with_fire_sim<R, F>(ptr: *const FireSimState, f: F) -> Option<R>
where
    F: FnOnce(&FireSimState) -> R,
{
    if ptr.is_null() {
        return None;
    }

    // SAFETY: caller contract guarantees pointer validity.
    unsafe { Some(f(&*ptr)) }
}

/// Advance the simulation by `dt` seconds.
///
/// Safety:
/// - `ptr` must be a valid pointer returned by `fire_sim_new`.
/// - Calling with an invalid pointer is undefined behavior.
/// - If `ptr` is null or `dt` is non-finite or non-positive this function is a no-op.
#[no_mangle]
pub extern "C" fn fire_sim_update(ptr: *mut FireSimState, dt: f32) {
    if !dt.is_finite() || dt <= 0.0 {
        return;
    }

    with_fire_sim_mut(ptr, |state| {
        state.sim.update(dt);
    });
}
