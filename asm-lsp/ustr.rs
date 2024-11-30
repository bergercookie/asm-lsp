//! use uncheck utf8 operations for str since its too costly to verify
//! input unnecessarily and from my survey our inputs are mostly ascii
use std::str;

/// Converts a byte slice to a string slice without checking for valid UTF-8.
///
/// # Safety
///
/// The caller must ensure that the input slice is valid UTF-8. Passing invalid UTF-8
/// to this function invokes undefined behavior.
#[must_use]
pub const fn get_str(v: &[u8]) -> &str {
    unsafe { str::from_utf8_unchecked(v) }
}

/// Converts a byte vector to a string without checking for valid UTF-8.
///
/// # Safety
///
/// The caller must ensure that the input vector is valid UTF-8. Passing invalid UTF-8
/// to this function invokes undefined behavior.
#[must_use]
pub fn get_string(v: Vec<u8>) -> String {
    unsafe { String::from_utf8_unchecked(v) }
}
