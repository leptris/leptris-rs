//! Pure, std-only checks shared by the wasm face. Kept outside
//! `wasmface` so they run in the native test suite (the face itself
//! only builds for wasm32 with the engine linked in).

use std::os::raw::c_int;

/// Convert the face's `u32` nesting cap to the engine's `c_int`.
///
/// An unchecked `as c_int` wraps values above `i32::MAX` to a
/// negative number, which the engine reads as an invalid or
/// unlimited cap. Such values are refused instead.
pub(crate) fn checked_max_depth(max_depth: u32) -> Result<c_int, String> {
    c_int::try_from(max_depth).map_err(|_| {
        format!(
            "max_depth {max_depth} exceeds the engine maximum {}",
            c_int::MAX
        )
    })
}

/// Convert the `LeptrisParseFlags` bitset to the engine's signed int.
/// Values above `c_int::MAX` would wrap to a negative flag word, so
/// they are refused instead.
pub(crate) fn checked_flags(flags: u32) -> Result<c_int, String> {
    c_int::try_from(flags).map_err(|_| {
        format!(
            "flags {flags:#x} exceeds the engine maximum {:#x}",
            c_int::MAX
        )
    })
}

/// Read one NUL-terminated entry of a snapshot lane starting at
/// `start`. An out-of-range offset is an error (the no-namespace
/// `SIZE_MAX` slot is handled by the caller before it gets here).
/// Invalid UTF-8 is an error, never silently repaired: an element
/// whose attribute data contains bytes the engine emitted for a
/// non-scalar reference such as `&#xD800;` fails as a whole
/// (pinned by `lane_rejects_surrogate_bytes`).
pub(crate) fn lane_str(blob: &[u8], start: usize) -> Result<String, String> {
    if start >= blob.len() {
        return Err(format!(
            "attribute offset {start} is outside the {}-byte lane",
            blob.len()
        ));
    }
    let tail = &blob[start..];
    let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
    std::str::from_utf8(&tail[..end])
        .map(str::to_owned)
        .map_err(|e| {
            format!(
                "attribute data is not valid UTF-8 (at byte {})",
                e.valid_up_to()
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_depth_in_range_passes_through() {
        assert_eq!(checked_max_depth(0), Ok(0));
        assert_eq!(checked_max_depth(5), Ok(5));
        assert_eq!(checked_max_depth(i32::MAX as u32), Ok(i32::MAX));
    }

    #[test]
    fn max_depth_above_i32_max_is_refused_not_wrapped() {
        assert!(checked_max_depth(i32::MAX as u32 + 1).is_err());
        assert!(checked_max_depth(u32::MAX).is_err());
    }

    #[test]
    fn lane_reads_to_first_nul() {
        assert_eq!(lane_str(b"ab\0cd\0", 3).as_deref(), Ok("cd"));
    }

    #[test]
    fn lane_out_of_range_offset_is_an_error() {
        assert!(lane_str(b"ab\0", 99).is_err());
        assert!(lane_str(b"ab\0", usize::MAX).is_err());
        assert!(lane_str(b"", 0).is_err());
    }

    #[test]
    fn lane_rejects_surrogate_bytes() {
        /* CESU-style encoding of U+D800, as an engine may emit it. */
        assert!(lane_str(b"\xED\xA0\x80\0", 0).is_err());
    }

    #[test]
    fn flags_in_range_pass_through() {
        assert_eq!(checked_flags(0), Ok(0));
        assert_eq!(checked_flags(63), Ok(63));
        assert_eq!(checked_flags(i32::MAX as u32), Ok(i32::MAX));
    }

    #[test]
    fn flags_above_i32_max_are_refused_not_wrapped() {
        assert!(checked_flags(i32::MAX as u32 + 1).is_err());
        assert!(checked_flags(0xFFFF_FFFF).is_err());
    }

    #[test]
    fn lane_rejects_invalid_utf8() {
        assert!(lane_str(b"a\xFFb\0", 0).is_err());
    }
}
