use fire_sim_core::{FireSimulation, FuelElement};
use std::ptr;

use crate::error::{DefaultFireSimError, FireSimErrorCode};
use crate::helpers::{handle_ffi_result_error, instance_from_ptr, track_error, with_fire_sim};
use crate::instance::FireSimInstance;

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
                ambient_temp as f32,
            );
        let rate_of_spread = spread_rate_m_per_min / 60.0;

        // Calculate fireline intensity using Byram's formula (kW/m)
        let intensity = element.byram_fireline_intensity(wind_speed_ms);

        Self {
            element_id: element.id(),
            is_burning: element.is_ignited(),
            fuel_load: *element.fuel_remaining(),
            temperature: *element.temperature() as f32,
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
/// - Returns a borrowed pointer to `ElementStats` array via `out_array`. **DO NOT FREE THIS POINTER**.
/// - The pointer is invalidated by the next call to `fire_sim_get_burning_elements` or `fire_sim_update`.
///
/// Thread-safe: Acquires Mutex lock on snapshot cache and `RwLock` read lock on simulation state.
/// Safe to call from multiple threads (e.g., Unreal async tasks, Godot worker threads).
///
/// Returns
/// - `FireSimErrorCode::Ok` (0) on success with valid array in `out_array` and count in `out_len`
/// - `FireSimErrorCode::NullPointer` if `ptr`, `out_len`, or `out_array` is null
/// - `FireSimErrorCode::LockPoisoned` if the internal lock is poisoned
///
/// # Safety
///
/// - `ptr` must be a valid pointer returned by `fire_sim_new` or null.
/// - `out_len` must be a valid, non-null pointer to a `usize` that this function will write to.
/// - `out_array` must be a valid, non-null pointer to a `*const ElementStats` that this function will write to.
///
/// # Example Usage (C++)
/// ```cpp
/// uintptr_t len = 0;
/// const ElementStats* burning = nullptr;
/// FireSimErrorCode err = fire_sim_get_burning_elements(sim, &len, &burning);
/// if (err != FireSimErrorCode::Ok) {
///     fprintf(stderr, "Failed to get burning elements\n");
///     return;
/// }
/// for (uintptr_t i = 0; i < len; i++) {
///     // Use burning[i] - no need to free
/// }
/// ```
pub unsafe extern "C" fn fire_sim_get_burning_elements(
    ptr: *const FireSimInstance,
    out_len: *mut usize,
    out_array: *mut *const ElementStats,
) -> FireSimErrorCode {
    // Validate both pointers before attempting to write to either
    if out_len.is_null() {
        return track_error(&DefaultFireSimError::null_pointer("out_len"));
    }

    if out_array.is_null() {
        // Now safe to write to out_len since we've validated it above
        unsafe {
            *out_len = 0;
        }
        return track_error(&DefaultFireSimError::null_pointer("out_array"));
    }

    let result = handle_ffi_result_error(|| {
        // SAFETY:
        // - We only use the borrowed `&FireSimInstance` to populate an internal `burning_snapshot`
        //   (protected by a Mutex) and then return a pointer to owned snapshot data.
        // - No borrow tied to `instance` escapes this function's scope.
        let instance = unsafe { instance_from_ptr(ptr)? };
        // Acquire Mutex lock on cached snapshot
        let mut snapshot = instance
            .burning_snapshot
            .lock()
            .map_err(|_| DefaultFireSimError::lock_poisoned("burning_snapshot Mutex"))?;
        snapshot.clear(); // O(1) - keeps capacity

        // Populate snapshot from current burning elements
        with_fire_sim(instance, |sim| {
            snapshot.extend(
                sim.get_burning_elements()
                    .into_iter()
                    .map(|e| ElementStats::from((e, sim))),
            );
        });

        // Set output values
        unsafe {
            *out_len = snapshot.len();
            *out_array = snapshot.as_ptr();
        }

        Ok::<(), DefaultFireSimError>(())
    });

    // Set to null on error (per documentation contract)
    if result != FireSimErrorCode::Ok {
        unsafe {
            *out_array = ptr::null();
            *out_len = 0;
        }
    }

    result
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
/// - If `ptr` is null or invalid, returns appropriate error code.
/// - This operation may cause an allocation if memory is reclaimed from the OS
///   (typically negligible, ~microseconds on modern systems).
///
/// Returns
/// - `FireSimErrorCode::Ok` (0) on success
/// - `FireSimErrorCode::NullPointer` if `ptr` is null
/// - `FireSimErrorCode::LockPoisoned` if the internal lock is poisoned
///
/// # Safety
///
/// - `ptr` must be a valid pointer created by `fire_sim_new`.
/// - `ptr` must remain valid for the duration of the call.
pub unsafe extern "C" fn fire_sim_clear_snapshot(ptr: *const FireSimInstance) -> FireSimErrorCode {
    handle_ffi_result_error(|| {
        // SAFETY:
        // - The reference is only used to lock and mutate the instance's internal snapshot under its Mutex.
        // - No borrows or references derived from the `instance` escape the function's scope.
        let instance = unsafe { instance_from_ptr(ptr)? };
        let mut snapshot = instance
            .burning_snapshot
            .lock()
            .map_err(|_| DefaultFireSimError::lock_poisoned("burning_snapshot Mutex"))?;
        snapshot.clear();
        snapshot.shrink_to_fit(); // Free unused capacity
        Ok::<(), DefaultFireSimError>(())
    })
}

#[no_mangle]
/// Fill `out_stats` with a snapshot of the specified element's statistics.
///
/// - `element_id` is the element index to query.
/// - `out_stats` receives the element statistics on success.
/// - `out_found` (optional) receives whether the element was found. If null, ignored.
///
/// Returns
/// - `FireSimErrorCode::Ok` (0) on success (check `out_found` to see if element exists)
/// - `FireSimErrorCode::NullPointer` if `ptr` or `out_stats` is null
/// - `FireSimErrorCode::LockPoisoned` if the internal lock is poisoned
///
/// # Safety
///
/// - `ptr` must be a valid pointer returned by `fire_sim_new` or null.
/// - `out_stats` must be a valid, non-null pointer to a `ElementStats` that this function will write to.
/// - `out_found` if non-null, must be a valid pointer to a `bool`.
pub unsafe extern "C" fn fire_sim_get_element_stats(
    ptr: *const FireSimInstance,
    element_id: usize,
    out_stats: *mut ElementStats,
    out_found: *mut bool,
) -> FireSimErrorCode {
    if out_stats.is_null() {
        return track_error(&DefaultFireSimError::null_pointer("out_stats"));
    }

    handle_ffi_result_error(|| {
        // SAFETY:
        // - We only read from the instance and copy values into `out_stats`; no references tied to the instance are returned.
        // - Any invalid pointer is handled by `instance_from_ptr` returning an error.
        let instance = unsafe { instance_from_ptr(ptr)? };
        // Query read-only state
        with_fire_sim(instance, |sim| {
            if let Some(element) = sim.get_element(element_id) {
                let stats = ElementStats::from((element, sim));
                unsafe {
                    *out_stats = stats;
                    if !out_found.is_null() {
                        *out_found = true;
                    }
                }
            } else {
                // Not an error - element just doesn't exist
                unsafe {
                    if !out_found.is_null() {
                        *out_found = false;
                    }
                }
            }
        });

        Ok::<(), DefaultFireSimError>(())
    })
}

/// Query the grid cell size (spatial resolution) used by the simulation.
///
/// This returns the resolution in meters per grid cell that was configured
/// when the simulation instance was created.
///
/// # Safety
///
/// - `ptr` must be a valid pointer to a `FireSimInstance` created by `fire_sim_new`
///   and not yet destroyed by `fire_sim_destroy`.
///
/// # Returns
///
/// The grid cell size in meters, or 0.0 if `ptr` is null or invalid.
///
/// # Example (C++)
/// ```cpp
/// float cell_size = fire_sim_get_grid_cell_size(sim);
/// printf("Simulation grid resolution: %.2f meters per cell\n", cell_size);
/// ```
#[no_mangle]
pub unsafe extern "C" fn fire_sim_get_grid_cell_size(ptr: *const FireSimInstance) -> f32 {
    // SAFETY: We check that ptr is not null above.
    // The caller must ensure ptr points to a valid FireSimInstance.
    instance_from_ptr(ptr).map_or(0.0, |instance| instance.grid_cell_size)
}
