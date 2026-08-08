//! Constant-time comparison of byte slices.
//!
//! Used for comparing cryptographic tokens/MACs so as not to reveal
//! information about a match via timing. The canonical `!=` on slices
//! performs a byte-by-byte comparison with early exit — this is a timing
//! channel, albeit a weak one for a local application. [`subtle::ConstantTimeEq`]
//! compares in a fixed number of cycles regardless of the position of the
//! first discrepancy.

use subtle::ConstantTimeEq;

/// Compares two byte slices in constant time.
///
/// Returns `true` if the slices are equal (same length and same bytes). The
/// length also participates in the comparison: slices of different lengths are
/// always not equal, but the check is not interrupted at the first byte
/// discrepancy.
///
/// # Examples
///
/// ```
/// use mailgrit_core_security::constant_time_eq;
/// assert!(constant_time_eq(b"abc", b"abc"));
/// assert!(!constant_time_eq(b"abc", b"abd"));
/// assert!(!constant_time_eq(b"abc", b"abcd"));
/// ```
#[must_use]
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_slices_are_equal() {
        assert!(constant_time_eq(b"", b""));
        assert!(constant_time_eq(b"abc", b"abc"));
        let long = vec![42u8; 256];
        assert!(constant_time_eq(&long, &long));
    }

    #[test]
    fn differing_byte_not_equal() {
        assert!(!constant_time_eq(b"abc", b"abd"));
        // Discrepancy in the first byte.
        assert!(!constant_time_eq(b"xbc", b"abc"));
        // Discrepancy in the last byte.
        assert!(!constant_time_eq(b"abx", b"abc"));
    }

    #[test]
    fn different_lengths_not_equal() {
        assert!(!constant_time_eq(b"a", b""));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(!constant_time_eq(b"abcd", b"abc"));
    }
}
