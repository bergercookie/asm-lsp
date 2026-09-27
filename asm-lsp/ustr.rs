//! use uncheck utf8 operations for str since its too costly to verify
//! input unnecessarily and from my survey our inputs are mostly ascii

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
