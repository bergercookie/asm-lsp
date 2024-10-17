//! use uncheck utf8 operations for str since its too costly to verify
//! input unnecessarily and from my survey our inputs are mostly ascii
use std::str;

#[must_use]
pub const fn get_str(v: &[u8]) -> &str {
    unsafe { str::from_utf8_unchecked(v) }
}

#[must_use]
pub fn get_string(v: Vec<u8>) -> String {
    unsafe { String::from_utf8_unchecked(v) }
}
