use crate::error::DefaultFireSimError;
use crate::helpers::{handle_ffi_result_error, instance_from_ptr, with_fire_sim_mut};
use crate::instance::FireSimInstance;

/// Advance the simulation by `dt` seconds.
///
/// Thread-safe: acquires `RwLock` write lock for simulation update.
///
/// # Safety
///
/// - `ptr` must be non-null and point to a valid `FireSimInstance` created by `fire_sim_new`.
/// - `ptr` must remain valid for the duration of the call.
/// - Null or invalid pointers are reported as errors; non-finite or non-positive `dt` are ignored.
#[no_mangle]
pub unsafe extern "C" fn fire_sim_update(ptr: *const FireSimInstance, dt: f32) {
    if !dt.is_finite() || dt <= 0.0 {
        return;
    }

    // Silently ignore errors for void-returning function
    let _ = handle_ffi_result_error(|| {
        // SAFETY:
        // - Converting `ptr` to `&FireSimInstance` is safe here because the borrowed reference
        //   is used only within this function for a short-lived update and no derived borrows escape.
        // - Null or invalid pointers are handled by `instance_from_ptr` returning an error.
        let instance = unsafe { instance_from_ptr(ptr)? };

        with_fire_sim_mut(instance, |sim| {
            sim.update(dt);
        });

        Ok::<(), DefaultFireSimError>(())
    });
}
