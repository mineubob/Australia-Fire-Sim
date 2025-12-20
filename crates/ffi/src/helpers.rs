use fire_sim_core::FireSimulation;

use crate::error::{with_last_error_mut, DefaultFireSimError, FireSimError, FireSimErrorCode};
use crate::instance::FireSimInstance;

/// Set the thread-local error message and code.
/// Internal helper for FFI functions to record failure details.
/// Accepts any type implementing `FireSimError` trait.
pub(crate) fn set_last_error(error: &impl FireSimError) {
    with_last_error_mut(|(msg, code)| {
        *msg = Some(error.msg());
        *code = error.code();
    });
}

/// Track an error by setting it in thread-local storage and returning its code.
/// More efficient than `handle_ffi_result_error` for immediate errors.
///
/// # Example
/// ```rust
/// if out_instance.is_null() {
///     return track_error(&DefaultFireSimError::null_pointer(\"out_instance\"));
/// }
/// ```
#[inline]
pub(crate) fn track_error(error: &impl FireSimError) -> FireSimErrorCode {
    set_last_error(error);
    error.code()
}

/// Track a Result by setting any error in thread-local storage.
/// Clears the error on success, or sets the error and returns its code on failure.
///
/// # Example
/// ```rust
/// let instance = track_result(FireSimInstance::new(&terrain))?;
/// ```
#[inline]
pub(crate) fn track_result<T, E: FireSimError>(
    result: Result<T, E>,
) -> Result<T, FireSimErrorCode> {
    match result {
        Ok(value) => {
            clear_last_error();
            Ok(value)
        }
        Err(error) => Err(track_error(&error)),
    }
}

/// Clear the thread-local error message and code.
/// Internal helper called on successful operations.
pub(crate) fn clear_last_error() {
    with_last_error_mut(|(msg, code)| {
        *msg = None;
        *code = FireSimErrorCode::Ok;
    });
}

/// Execute a callback and return `FireSimErrorCode` directly.
/// Returns `FireSimErrorCode::Ok` on success, or the error code on failure.
///
/// This is a convenience wrapper for functions that return errors implementing `FireSimError`.
/// It handles error tracking automatically by calling `set_last_error` with the returned error.
///
/// # Example
/// ```rust
/// handle_ffi_result_error(|| {
///     if ptr.is_null() {
///         return Err(DefaultFireSimError::null_pointer("ptr"));
///     }
///     Ok(())
/// })
/// ```
#[inline]
pub(crate) fn handle_ffi_result_error<E, F>(f: F) -> FireSimErrorCode
where
    E: FireSimError,
    F: FnOnce() -> Result<(), E>,
{
    match f() {
        Ok(()) => {
            clear_last_error();
            FireSimErrorCode::Ok
        }
        Err(error) => {
            set_last_error(&error);
            error.code()
        }
    }
}

/// Convert raw pointer to reference, validate it exists, and return the reference.
/// Returns error if pointer is null.
///
/// Safety note: the caller must ensure the pointer originates from `fire_sim_new` and is not dangling.
#[inline]
pub(crate) fn instance_from_ptr<'a>(
    ptr: *const FireSimInstance,
) -> Result<&'a FireSimInstance, DefaultFireSimError> {
    if ptr.is_null() {
        return Err(DefaultFireSimError::null_pointer("ptr"));
    }

    // SAFETY: pointer validity checked above, and caller guarantees it came from fire_sim_new
    unsafe { Ok(&*ptr) }
}

/// If valid instance, call `f` with a `&FireSimulation` and return the closure result.
/// Returns `Err(FireSimErrorCode::LockPoisoned)` if the lock is poisoned (indicates a previous panic).
///
/// Thread-safe: acquires the internal `RwLock` read lock for the duration of the closure.
///
/// Safety note: the caller must ensure the reference is valid.
#[inline]
pub(crate) fn with_fire_sim<R, F>(
    instance: &FireSimInstance,
    f: F,
) -> Result<R, DefaultFireSimError>
where
    F: FnOnce(&FireSimulation) -> R,
{
    // Acquire the read lock for the duration of the closure.
    // If the lock is poisoned, propagate an explicit error instead of panicking across FFI.
    let sim = instance
        .sim
        .read()
        .map_err(|_| DefaultFireSimError::lock_poisoned("FireSimulation RwLock"))?;
    Ok(f(&sim))
}

/// If valid instance, call `f` with a `&mut FireSimulation` and return the closure result.
/// Returns `Err(FireSimErrorCode::LockPoisoned)` if the lock is poisoned (indicates a previous panic).
///
/// Thread-safe: acquires the internal `RwLock` write lock for the duration of the closure.
///
/// Safety note: the caller must ensure the reference is valid.
#[inline]
pub(crate) fn with_fire_sim_mut<R, F>(
    instance: &FireSimInstance,
    f: F,
) -> Result<R, DefaultFireSimError>
where
    F: FnOnce(&mut FireSimulation) -> R,
{
    // Acquire the write lock for the duration of the closure.
    // If the lock is poisoned, propagate an explicit error instead of panicking across FFI.
    let mut sim = instance
        .sim
        .write()
        .map_err(|_| DefaultFireSimError::lock_poisoned("FireSimulation RwLock"))?;
    Ok(f(&mut sim))
}
