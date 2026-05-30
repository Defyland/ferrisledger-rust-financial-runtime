//! Narrow FFI boundary for checksum experiments.
//!
//! The production store currently uses safe Rust directly. This crate exists to
//! demonstrate how future C callers or C-optimized checksum experiments would
//! be isolated behind a tiny, documented surface.

/// Calculates CRC32 using safe Rust.
#[must_use]
pub fn checksum_bytes(bytes: &[u8]) -> u32 {
    crc32fast::hash(bytes)
}

/// C ABI checksum function.
///
/// # Safety
///
/// `ptr` must point to `len` readable bytes for the duration of the call. A
/// null pointer is accepted only when `len == 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ferrisledger_crc32(ptr: *const u8, len: usize) -> u32 {
    if ptr.is_null() {
        return if len == 0 { 0 } else { u32::MAX };
    }
    // SAFETY: The caller promises that `ptr` is valid for `len` bytes. This is
    // the only unsafe operation in the FFI crate and is covered by tests.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    checksum_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_checksum_matches_safe_wrapper() {
        let bytes = b"ferrisledger";
        let safe = checksum_bytes(bytes);
        // SAFETY: `bytes.as_ptr()` is valid for `bytes.len()` bytes.
        let ffi = unsafe { ferrisledger_crc32(bytes.as_ptr(), bytes.len()) };

        assert_eq!(safe, ffi);
    }

    #[test]
    fn null_empty_pointer_returns_zero() {
        // SAFETY: The FFI contract allows null when len is zero.
        assert_eq!(unsafe { ferrisledger_crc32(std::ptr::null(), 0) }, 0);
    }
}
