use crate::error::DefaultFireSimError;
use crate::helpers::{handle_ffi_result_error, instance_from_ptr, with_fire_sim_mut};
use crate::instance::FireSimInstance;

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

    // Silently ignore errors for void-returning function
    let _ = handle_ffi_result_error(|| {
        let instance = instance_from_ptr(ptr)?;

        with_fire_sim_mut(instance, |sim| {
            sim.update(dt);
        })?;

        Ok::<(), DefaultFireSimError>(())
    });
}
