use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// Common interface for FFI error types.
///
/// This trait provides a unified way to handle errors across the FFI boundary,
/// allowing both simple error codes and custom error messages.
///
/// # Design
/// - `code()` - Returns the error code to be passed across FFI boundary
/// - `msg()` - Returns the error message for diagnostic purposes
///
/// # Example
/// ```rust
/// // Simple error code wrapped in DefaultFireSimError
/// let err = DefaultFireSimError::null_pointer("ptr");
/// assert_eq!(err.code(), FireSimErrorCode::NullPointer);
/// assert_eq!(err.msg(), "Parameter 'ptr' cannot be null");
/// ```
pub(crate) trait FireSimError {
    /// Returns the error code to be returned across the FFI boundary.
    fn code(&self) -> FireSimErrorCode;

    /// Returns the human-readable error message.
    ///
    /// Default implementation uses the error code's `Display` implementation.
    /// Custom error types can override this to provide more specific messages.
    fn msg(&self) -> &str;
}

/// Default implementation of `FireSimError` for common FFI error scenarios.
///
/// This struct wraps a `FireSimErrorCode` and provides convenient constructors
/// for each error type (except Ok, which represents success).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DefaultFireSimError {
    code: FireSimErrorCode,
    msg: String,
}

impl DefaultFireSimError {
    /// Create error for null pointer passed where non-null required.
    ///
    /// # Arguments
    /// * `param_name` - The name of the parameter that was null (e.g., `"out_instance"`, `"ptr"`)
    pub fn null_pointer(param_name: &str) -> Self {
        Self {
            code: FireSimErrorCode::NullPointer,
            msg: format!("Parameter '{param_name}' cannot be null"),
        }
    }

    /// Create error for poisoned lock.
    ///
    /// # Arguments
    /// * `lock_name` - The name of the lock that was poisoned (e.g., `"RwLock"`, `"Mutex"`)
    #[allow(dead_code)]
    pub fn lock_poisoned(lock_name: &str) -> Self {
        Self {
            code: FireSimErrorCode::LockPoisoned,
            msg: format!("Lock '{lock_name}' was poisoned by a panic in another thread"),
        }
    }

    /// Create error for invalid terrain parameters with a custom message.
    ///
    /// # Arguments
    /// * `param_name` - The name of the invalid parameter (e.g., `"width"`, `"height"`, `"nx"`)
    /// * `message` - A description of the validation error
    pub fn invalid_terrain_parameter_msg(param_name: &str, message: &str) -> Self {
        Self {
            code: FireSimErrorCode::InvalidTerrainParameters,
            msg: format!("Terrain parameter {param_name}: {message}"),
        }
    }

    /// Create error for invalid terrain parameters (f32 values).
    ///
    /// # Arguments
    /// * `param_name` - The name of the invalid parameter (e.g., `"width"`, `"height"`, `"resolution"`)
    /// * `value` - The invalid value
    #[allow(dead_code)]
    pub fn invalid_terrain_parameter(param_name: &str, value: f32) -> Self {
        Self::invalid_terrain_parameter_msg(
            param_name,
            &format!("must be finite and positive, got {value}"),
        )
    }

    /// Create error for invalid terrain parameters (usize values).
    ///
    /// # Arguments
    /// * `param_name` - The name of the invalid parameter (e.g., `"nx"`, `"ny"`, `"grid_dimension"`)
    /// * `value` - The invalid value
    /// * `constraint` - Description of the constraint (e.g., `"must be positive"`, `"exceeds maximum"`)
    #[allow(dead_code)]
    pub fn invalid_terrain_parameter_usize(
        param_name: &str,
        value: usize,
        constraint: &str,
    ) -> Self {
        Self::invalid_terrain_parameter_msg(param_name, &format!("{constraint}, got {value}"))
    }

    /// Create error for invalid parameter.
    ///
    /// # Arguments
    /// * `message` - Description of the error
    pub fn invalid_parameter(message: String) -> Self {
        Self {
            code: FireSimErrorCode::InvalidParameter,
            msg: message,
        }
    }
}

impl FireSimError for DefaultFireSimError {
    fn code(&self) -> FireSimErrorCode {
        self.code
    }

    fn msg(&self) -> &str {
        &self.msg
    }
}

/// FFI error codes returned by fire simulation functions.
/// Follows standard C convention: 0 = success, non-zero = error.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireSimErrorCode {
    /// Operation completed successfully.
    Ok = 0,

    /// Invalid pointer: null pointer passed where non-null required.
    NullPointer = 1,

    /// Lock poisoned: internal synchronization primitive was poisoned by a panic.
    LockPoisoned = 2,

    /// Invalid terrain parameters: width, height, resolution, or dimensions must be valid
    /// (finite and positive for f32, or within representable range for usize).
    InvalidTerrainParameters = 3,

    /// Invalid parameter passed to function.
    InvalidParameter = 4,
}

impl From<DefaultFireSimError> for FireSimErrorCode {
    fn from(error: DefaultFireSimError) -> Self {
        error.code
    }
}

thread_local! {
    /// Thread-local storage for the most recent FFI error (C string, error code).
    /// Allows callers to retrieve diagnostic information after operations that return null.
    /// The CString is stored to prevent memory leaks when returning raw pointers via FFI.
    static LAST_ERROR: RefCell<(Option<CString>, FireSimErrorCode)> = const { RefCell::new((None, FireSimErrorCode::Ok)) };
}

/// Internal helper to read `LAST_ERROR` thread-local storage (cstring, code).
pub(crate) fn with_last_error<F, R>(f: F) -> R
where
    F: FnOnce(&(Option<CString>, FireSimErrorCode)) -> R,
{
    LAST_ERROR.with_borrow(f)
}

/// Internal helper to mutate `LAST_ERROR` thread-local storage (cstring, code).
pub(crate) fn with_last_error_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut (Option<CString>, FireSimErrorCode)) -> R,
{
    LAST_ERROR.with_borrow_mut(f)
}

/// Retrieve the most recent FFI error message as a null-terminated C string.
///
/// Returns:
/// - A borrowed pointer to the error message if an error occurred.
/// - `null` if no error has occurred or the error message cannot be converted to C string.
///
/// # Thread Safety
/// Error messages are stored per-thread (thread-local storage), so this is thread-safe.
/// Each thread has its own independent error state.
///
/// # Lifetime
/// The returned pointer is valid until:
/// - The next FFI call on this thread that sets or clears the error
/// - The thread terminates
///
/// **DO NOT FREE THIS POINTER** - it is managed internally.
///
/// Example:
/// ```cpp
/// FireSimInstance* sim = nullptr;
/// FireSimErrorCode err = fire_sim_new(terrain, &sim);
/// if (err != FireSimErrorCode::Ok) {
///     const char* error = fire_sim_get_last_error();
///     if (error) {
///         printf("Fire sim creation failed: %s\n", error);
///     }
/// }
/// ```
#[no_mangle]
pub extern "C" fn fire_sim_get_last_error() -> *const c_char {
    with_last_error(|(cstring, _code)| cstring.as_ref().map_or(ptr::null(), |cs| cs.as_ptr()))
}

/// Retrieve the most recent FFI error code.
///
/// Returns:
/// - `FireSimErrorCode::Ok` (0) if no error has occurred
/// - The specific error code from the last failed operation
///
/// # Thread Safety
/// Error codes are stored per-thread (thread-local storage), so this is thread-safe.
/// Each thread has its own independent error state.
///
/// Example:
/// ```cpp
/// FireSimInstance* sim = nullptr;
/// FireSimErrorCode err = fire_sim_new(terrain, &sim);
/// if (err != FireSimErrorCode::Ok) {
///     FireSimErrorCode last_err = fire_sim_get_last_error_code();
///     // last_err == err
/// }
/// ```
#[no_mangle]
pub extern "C" fn fire_sim_get_last_error_code() -> FireSimErrorCode {
    with_last_error(|(_cstring, code)| *code)
}
