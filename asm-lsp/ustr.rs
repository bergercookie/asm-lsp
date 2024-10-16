use std::str;
// TODO: use uncheck utf8 operations for str since its too costly to verify
// input unnecessarily and from my survey our inputs are mostly ascii

#[must_use]
pub const fn get_str(v: &[u8]) -> &str {
    unsafe { str::from_utf8_unchecked(v) }
}
