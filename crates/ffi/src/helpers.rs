use crate::error::{with_last_error_mut, FireSimError, FireSimErrorCode};
use std::ffi::CString;

/// Set the thread-local error message and code.
/// Internal helper for FFI functions to record failure details.
/// Accepts any type implementing `FireSimError` trait.
pub(crate) fn set_last_error(error: &impl FireSimError) {
    with_last_error_mut(|(cstring, code)| {
        *cstring = CString::new(error.msg()).ok();
        *code = error.code();
    });
}

/// Track an error by setting it in thread-local storage and returning its code.
/// More efficient than handling results for immediate errors.
#[inline]
pub(crate) fn track_error(error: &impl FireSimError) -> FireSimErrorCode {
    set_last_error(error);
    error.code()
}

/// Clear the thread-local error message and code.
/// Internal helper called on successful operations.
#[expect(dead_code)]
pub(crate) fn clear_last_error() {
    with_last_error_mut(|(cstring, code)| {
        *cstring = None;
        *code = FireSimErrorCode::Ok;
    });
}
