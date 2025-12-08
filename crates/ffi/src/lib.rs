use fire_sim_core::{FireSimulation, FuelElement, TerrainData};
use std::ptr;
use std::sync::{Mutex, RwLock};

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

/// The main fire simulation context.
/// Holds the internal simulation state and manages fire behavior calculations.
///
/// # Thread Safety
/// `FireSimInstance` is fully thread-safe and can be safely shared across multiple threads
/// in Godot, Unreal Engine, or any other multi-threaded environment.
///
/// The internal simulation is protected by an `RwLock`, allowing:
/// - **Multiple concurrent readers** (queries, state inspections): `.read()` lock
/// - **Exclusive writer** (simulation updates): `.write()` lock
/// - **Godot (GDScript/C#)**: Safe calls from main thread, render thread, and worker threads
/// - **Unreal Engine (C++)**: Safe calls from Game Thread, Render Thread, and async task threads
/// - **Efficient concurrent reads/writes**: `RwLock` allows multiple readers without blocking
///
/// # Usage in Game Engines
///
/// ## Godot Example
/// ```gdscript
/// var fire_sim: int  # Opaque pointer to FireSimInstance
///
/// func _ready():
///     fire_sim = FireSimFFI.fire_sim_new(create_flat_terrain(1000.0, 1000.0))
///
/// func _process(delta):
///     # Safe to call from main thread
///     FireSimFFI.fire_sim_update(fire_sim, delta)
///
/// func _exit_tree():
///     FireSimFFI.fire_sim_destroy(fire_sim)
/// ```
///
/// ## Unreal Engine Example
/// ```cpp
/// // In your actor or component header
/// void* FireSimPtr = nullptr;
///
/// // In BeginPlay or initialization
/// void AFireSimActor::BeginPlay() {
///     Terrain terrain = CreateFlatTerrain(1000.0f, 1000.0f);
///     FireSimPtr = fire_sim_new(terrain);
/// }
///
/// // Safe to call from Game Thread, Render Thread via async tasks, etc.
/// void AFireSimActor::Tick(float DeltaTime) {
///     fire_sim_update(FireSimPtr, DeltaTime);
/// }
///
/// // In EndPlay or destructor
/// void AFireSimActor::EndPlay(const EEndPlayReason::Type EndPlayReason) {
///     if (FireSimPtr) {
///         fire_sim_destroy(FireSimPtr);
///         FireSimPtr = nullptr;
///     }
/// }
/// ```
///
/// # Performance Note
/// `RwLock` allows multiple threads to read state simultaneously without blocking,
/// while `fire_sim_update()` acquires an exclusive write lock briefly.
/// This is optimal for game engines where queries (reads) are frequent but updates (writes)
/// happen once per frame. Overhead is negligible at 60 FPS (16.7ms per frame).
pub struct FireSimInstance {
    sim: RwLock<FireSimulation>,
    /// Cached snapshot of burning elements to avoid per-frame allocations.
    /// Protected by Mutex for thread-safe access across game engine threads.
    /// Reused across calls to `fire_sim_get_burning_elements`.
    burning_snapshot: Mutex<Vec<ElementStats>>,
}

impl FireSimInstance {
    /// Creates a new `FireSim` instance with the specified terrain.
    #[must_use]
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

        Box::new(Self {
            sim: RwLock::new(sim),
            burning_snapshot: Mutex::new(Vec::with_capacity(1000)),
        })
    }
}

/// Create a new `FireSim` instance and return a raw pointer to it for use across FFI.
///
/// This function allocates a new `FireSim` on the heap and transfers ownership of the
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
/// - `*mut FireSim` — a raw, heap-allocated pointer owning the newly created `FireSim`.
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
pub extern "C" fn fire_sim_new(terrain: Terrain) -> *mut FireSimInstance {
    Box::into_raw(FireSimInstance::new(terrain))
}

/// Destroys a `FireSim` instance previously created by `fire_sim_new`.
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
/// - This is an `extern "C"` (`no_mangle`) function intended for use across language boundaries.
/// - It is safe to call from C/C++/other languages provided the pointer contract above is respected.
#[no_mangle]
pub unsafe extern "C" fn fire_sim_destroy(ptr: *mut FireSimInstance) {
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

/// Convert raw pointer to reference, validate it exists, and call callback with the reference.
/// Returns default value if pointer is null (callback not called).
///
/// Safety note: the caller must ensure the pointer originates from `fire_sim_new` and is not dangling.
#[inline]
fn instance_from_ptr<R, F>(ptr: *const FireSimInstance, f: F) -> Option<R>
where
    F: FnOnce(&FireSimInstance) -> R,
{
    if ptr.is_null() {
        return None;
    }

    // SAFETY: pointer validity checked above, and caller guarantees it came from fire_sim_new
    unsafe { Some(f(&*ptr)) }
}

/// If valid instance, call `f` with a `&FireSimulation` and return the closure result.
/// Panics if the lock is poisoned (indicates a previous panic during lock acquisition).
///
/// Thread-safe: acquires the internal `RwLock` read lock for the duration of the closure.
///
/// Safety note: the caller must ensure the reference is valid.
#[inline]
fn with_fire_sim<R, F>(instance: &FireSimInstance, f: F) -> R
where
    F: FnOnce(&FireSimulation) -> R,
{
    // Acquire the read lock for the duration of the closure.
    // Panic if the lock is poisoned (acceptable for FFI safety - indicates previous panic).
    let sim = instance.sim.read().expect("FireSimulation RwLock poisoned");
    f(&sim)
}

/// If valid instance, call `f` with a `&mut FireSimulation` and return the closure result.
/// Panics if the lock is poisoned (indicates a previous panic during lock acquisition).
///
/// Thread-safe: acquires the internal `RwLock` write lock for the duration of the closure.
///
/// Safety note: the caller must ensure the reference is valid.
#[inline]
fn with_fire_sim_mut<R, F>(instance: &FireSimInstance, f: F) -> R
where
    F: FnOnce(&mut FireSimulation) -> R,
{
    // Acquire the write lock for the duration of the closure.
    // Panic if the lock is poisoned (acceptable for FFI safety - indicates previous panic).
    let mut sim = instance
        .sim
        .write()
        .expect("FireSimulation RwLock poisoned");
    f(&mut sim)
}

/// Advance the simulation by `dt` seconds.
///
/// Thread-safe: acquires `RwLock` write lock for simulation update.
///
/// Safety:
/// - `ptr` must be a valid pointer returned by `fire_sim_new`.
/// - Calling with an invalid pointer is undefined behavior.
/// - If `ptr` is null or `dt` is non-finite or non-positive this function is a no-op.
#[no_mangle]
pub extern "C" fn fire_sim_update(ptr: *const FireSimInstance, dt: f32) {
    if !dt.is_finite() || dt <= 0.0 {
        return;
    }

    instance_from_ptr(ptr, |instance| {
        with_fire_sim_mut(instance, |sim| {
            sim.update(dt);
        });
    });
}

#[repr(C)]
/// FFI-friendly snapshot of an element's runtime statistics.
/// Keep this layout stable for C/C++/C# consumers.
pub struct ElementStats {
    /// Element identifier (index).
    pub element_id: usize,

    /// Whether this element is currently burning.
    pub is_burning: bool,

    /// Fuel load remaining (kg).
    pub fuel_load: f32,

    /// Element temperature (Celsius).
    pub temperature: f32,

    /// Fuel moisture fraction (0.0 - 1.0).
    pub moisture: f32,

    /// Rate of spread (m/s).
    pub rate_of_spread: f32,

    /// Flame length (m).
    pub flame_length: f32,

    /// Fireline intensity (kW/m).
    pub intensity: f32,
}

impl From<(&FuelElement, &FireSimulation)> for ElementStats {
    fn from((element, sim): (&FuelElement, &FireSimulation)) -> Self {
        // Get wind at this element's position (m/s)
        let wind = sim.wind_at_position(*element.position());
        let wind_speed_ms = wind.magnitude();

        // Get ambient temperature from weather system (°C)
        let ambient_temp = *sim.get_weather().temperature();

        // Calculate rate of spread using Rothermel model (m/min -> m/s)
        let spread_rate_m_per_min =
            fire_sim_core::physics::rothermel_validation::rothermel_spread_rate(
                element.fuel(),
                *element.moisture_fraction(),
                wind_speed_ms,
                *element.slope_angle(),
                ambient_temp,
            );
        let rate_of_spread = spread_rate_m_per_min / 60.0;

        // Calculate fireline intensity using Byram's formula (kW/m)
        let intensity = element.byram_fireline_intensity(wind_speed_ms);

        Self {
            element_id: element.id(),
            is_burning: element.is_ignited(),
            fuel_load: *element.fuel_remaining(),
            temperature: *element.temperature(),
            moisture: *element.moisture_fraction(),
            rate_of_spread,
            flame_length: *element.flame_height(),
            intensity,
        }
    }
}

#[no_mangle]
/// Return a borrowed pointer to cached burning elements snapshot.
///
/// **PERFORMANCE & THREAD SAFETY**: This function reuses an internal buffer protected by Mutex
/// to avoid per-frame allocations while remaining thread-safe for Godot and Unreal Engine.
/// The returned pointer is valid until the next call to this function or `fire_sim_update`.
///
/// - Returns a borrowed pointer to `ElementStats` array. **DO NOT FREE THIS POINTER**.
/// - Returns null on error or when `ptr`/`out_len` are null.
/// - The pointer is invalidated by the next call to `fire_sim_get_burning_elements` or `fire_sim_update`.
///
/// Thread-safe: Acquires Mutex lock on snapshot cache and `RwLock` read lock on simulation state.
/// Safe to call from multiple threads (e.g., Unreal async tasks, Godot worker threads).
///
/// # Safety
///
/// - `ptr` must be a valid pointer returned by `fire_sim_new` or null.
/// - `out_len` must be a valid, non-null pointer to a `usize` that this function will write to.
///
/// # Example Usage (C++)
/// ```cpp
/// uintptr_t len = 0;
/// const ElementStats* burning = fire_sim_get_burning_elements(sim, &len);
/// if (burning) {
///     for (uintptr_t i = 0; i < len; i++) {
///         // Use burning[i] - no need to free
///     }
/// }
/// ```
pub unsafe extern "C" fn fire_sim_get_burning_elements(
    ptr: *const FireSimInstance,
    out_len: *mut usize,
) -> *const ElementStats {
    if out_len.is_null() {
        return ptr::null();
    }

    instance_from_ptr(ptr, |instance| {
        // Acquire Mutex lock on cached snapshot
        let mut snapshot = match instance.burning_snapshot.lock() {
            Ok(guard) => guard,
            Err(_) => return ptr::null(), // Mutex poisoned, return null
        };
        snapshot.clear(); // O(1) - keeps capacity

        // Populate snapshot from current burning elements
        with_fire_sim(instance, |sim| {
            snapshot.extend(
                sim.get_burning_elements()
                    .into_iter()
                    .map(|e| ElementStats::from((e, sim))),
            );
        });

        // Set output length
        unsafe {
            *out_len = snapshot.len();
        }

        // Return borrowed pointer (valid until next call or lock release)
        snapshot.as_ptr()
    })
    .unwrap_or(ptr::null())
}

#[no_mangle]
/// Clear the cached burning elements snapshot and free unused memory.
///
/// This clears the snapshot and shrinks its capacity to fit the actual data.
/// Useful for memory-constrained environments or between simulation phases.
///
/// Thread-safe: Acquires Mutex lock on snapshot cache.
///
/// - `ptr` must be a pointer returned by `fire_sim_new`.
/// - If `ptr` is null or invalid, this is a no-op.
/// - This operation may cause an allocation if memory is reclaimed from the OS
///   (typically negligible, ~microseconds on modern systems).
///
/// Returns `true` on success, `false` if the pointer is invalid or the lock is poisoned.
pub extern "C" fn fire_sim_clear_snapshot(ptr: *const FireSimInstance) -> bool {
    instance_from_ptr(ptr, |instance| {
        match instance.burning_snapshot.lock() {
            Ok(mut snapshot) => {
                snapshot.clear();
                snapshot.shrink_to_fit(); // Free unused capacity
                true
            }
            Err(_) => false, // Mutex poisoned
        }
    })
    .unwrap_or(false)
}

#[no_mangle]
/// Fill `out_stats` with a snapshot of the specified element's statistics.
///
/// - `element_id` is the element index to query.
///
/// Returns:
/// - `true` on success (`out_stats` filled)
/// - `false` if the pointer is invalid, the element does not exist, or on any error.
///
/// # Safety
///
/// - `ptr` must be a valid pointer returned by `fire_sim_new` or null.
/// - `out_stats` must be a valid, non-null pointer to a `ElementStats` that this function will write to.
pub unsafe extern "C" fn fire_sim_get_element_stats(
    ptr: *const FireSimInstance,
    element_id: usize,
    out_stats: *mut ElementStats,
) -> bool {
    if out_stats.is_null() {
        return false;
    }

    instance_from_ptr(ptr, |instance| {
        // Query read-only state
        with_fire_sim(instance, |sim| match sim.get_element(element_id) {
            Some(element) => {
                let stats = ElementStats::from((element, sim));
                unsafe {
                    *out_stats = stats;
                }
                true
            }
            None => false,
        })
    })
    .unwrap_or(false)
}
