use fire_sim_core::{FireSimulation, TerrainData};
use std::ptr;
use std::sync::{Mutex, RwLock};

use crate::error::{DefaultFireSimError, FireSimErrorCode};
use crate::helpers::{track_error, track_result};
use crate::queries::ElementStats;
use crate::terrain::Terrain;

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
/// var fire_sim: int = 0  # Opaque pointer to FireSimInstance
///
/// func _ready():
///     # Create flat terrain 1000m x 1000m
///     # Note: Actual Terrain struct construction depends on your GDNative/GDExtension bindings
///     var terrain = Terrain.new()
///     terrain.type = Terrain.FLAT
///     terrain.width = 1000.0
///     terrain.height = 1000.0
///     
///     # In Godot you should call a GDNative/GDExtension wrapper that handles the C out-parameter.
///     # For example, the wrapper can return both error code and instance handle:
///     var result = FireSimFFI.fire_sim_new_godot(terrain)
///     if result.error != FireSimFFI.FireSimErrorCode.Ok:
///         push_error("Failed to create fire simulation")
///         return
///     fire_sim = result.instance
///
/// func _process(delta):
///     # Safe to call from main thread
///     FireSimFFI.fire_sim_update(fire_sim, delta)
///
/// func _exit_tree():
///     if fire_sim != 0:
///         FireSimFFI.fire_sim_destroy(fire_sim)
/// ```
///
/// ## Unreal Engine Example
/// ```cpp
/// // In your actor or component header
/// FireSimInstance* FireSimPtr = nullptr;
///
/// // In BeginPlay or initialization
/// void AFireSimActor::BeginPlay() {
///     // Create flat terrain 1000m x 1000m
///     Terrain terrain;
///     terrain.tag = Terrain::Tag::Flat;
///     terrain.flat.width = 1000.0f;
///     terrain.flat.height = 1000.0f;
///     
///     FireSimErrorCode err = fire_sim_new(terrain, &FireSimPtr);
///     if (err != FireSimErrorCode::Ok) {
///         UE_LOG(LogTemp, Error, TEXT("Failed to create fire simulation"));
///         return;
///     }
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
    pub(crate) sim: RwLock<FireSimulation>,
    /// Cached snapshot of burning elements to avoid per-frame allocations.
    /// Protected by Mutex for thread-safe access across game engine threads.
    /// Reused across calls to `fire_sim_get_burning_elements`.
    pub(crate) burning_snapshot: Mutex<Vec<ElementStats>>,
}

impl FireSimInstance {
    /// Creates a new `FireSim` instance with the specified terrain.
    ///
    /// # Errors
    ///
    /// Returns `FireSimErrorCode::NullPointer` if heightmap pointer is null.
    /// Returns `FireSimErrorCode::InvalidHeightmapDimensions` if heightmap dimensions are zero.
    pub(crate) fn new(terrain: &Terrain) -> Result<Box<Self>, DefaultFireSimError> {
        let terrain = match *terrain {
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
                // Validate input: return error if pointer is null or size is invalid.
                let len = nx.checked_mul(ny).unwrap_or(0);
                if heightmap_ptr.is_null() {
                    return Err(DefaultFireSimError::null_pointer("heightmap_ptr"));
                }
                if len == 0 {
                    return Err(DefaultFireSimError::invalid_heightmap_dimensions(nx, ny));
                }
                let slice = unsafe { std::slice::from_raw_parts(heightmap_ptr, len) };
                TerrainData::from_heightmap(
                    width,
                    height,
                    slice,
                    nx,
                    ny,
                    elevation_scale,
                    base_elevation,
                )
            }
        };

        let sim = FireSimulation::new(5.0, &terrain);

        Ok(Box::new(Self {
            sim: RwLock::new(sim),
            burning_snapshot: Mutex::new(Vec::with_capacity(1000)),
        }))
    }
}

/// Create a new `FireSim` instance and return it via out-parameter.
///
/// This function follows standard C error handling conventions:
/// - Returns `FireSimErrorCode::Ok` (0) on success with valid instance in `out_instance`
/// - Returns non-zero error code on failure with `out_instance` set to null
///
/// Parameters
/// - `terrain`: A `Terrain` value describing the desired terrain configuration.
///   - For `Terrain::FromHeightmap`, the heightmap data is read and copied into Rust-owned
///     memory. After this call the caller may deallocate the original heightmap.
/// - `out_instance`: Pointer to receive the created instance. Must be non-null.
///   - On success: set to valid `FireSimInstance` pointer
///   - On failure: set to null
///
/// Returns
/// - `FireSimErrorCode::Ok` (0) — success, `out_instance` contains valid pointer
/// - `FireSimErrorCode::NullPointer` — heightmap pointer is null
/// - `FireSimErrorCode::InvalidHeightmapDimensions` — heightmap dimensions are zero
/// - `FireSimErrorCode::NullPointer` — `out_instance` parameter is null
///
/// Error Details
/// - Call `fire_sim_get_last_error()` to retrieve human-readable error description
///
/// # Safety
///
/// - `out_instance` must be a valid, non-null pointer to writable memory.
/// - The caller takes ownership of the returned instance and MUST call `fire_sim_destroy`
///   exactly once to avoid memory leaks.
/// - The `terrain` value is moved by value; heightmap pointers are only read and copied.
/// - This function is `extern "C"` and `#[no_mangle]` for FFI usage.
///
/// Example (C++)
/// ```cpp
/// FireSimInstance* sim = nullptr;
/// FireSimErrorCode err = fire_sim_new(terrain, &sim);
/// if (err != FireSimErrorCode::Ok) {
///     const char* error_msg = fire_sim_get_last_error();
///     fprintf(stderr, "Failed to create simulation: %s\n", error_msg);
///     return;
/// }
/// // ... use sim ...
/// fire_sim_destroy(sim);
/// ```
#[no_mangle]
pub unsafe extern "C" fn fire_sim_new(
    terrain: Terrain,
    out_instance: *mut *mut FireSimInstance,
) -> FireSimErrorCode {
    if out_instance.is_null() {
        return track_error(&DefaultFireSimError::null_pointer("out_instance"));
    }

    match track_result(FireSimInstance::new(&terrain)) {
        Ok(instance) => {
            unsafe {
                *out_instance = Box::into_raw(instance);
            }
            FireSimErrorCode::Ok
        }
        Err(code) => {
            unsafe {
                // Set to null on error (per documentation contract)
                *out_instance = ptr::null_mut();
            }

            code
        }
    }
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
