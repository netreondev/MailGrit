//! Common low-level helpers for CSV parsing (private module).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/// Maximum length of a single CSV line (bytes), including delimiters.
///
/// Also the per-record raw cap in [`crate::record`] (a record spanning several
/// physical lines via quoted newlines is capped by its total size). Enforced
/// BEFORE buffering: the record reader never holds more than this + 1 bytes.
pub const MAX_LINE_BYTES: usize = 16 * 1024;

/// Strips the UTF-8 BOM (`EF BB BF`) from the start of the data, if present.
///
/// Excel/Word add a BOM; without stripping it, it attaches to the first field
/// (domain) and breaks validation. Returns the slice without the BOM (or the
/// original if no BOM is present).
#[must_use]
pub fn strip_bom(data: &[u8]) -> &[u8] {
    data.strip_prefix(b"\xef\xbb\xbf").unwrap_or(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bom_is_stripped() {
        assert_eq!(strip_bom(b"\xef\xbb\xbfabc"), b"abc");
        assert_eq!(strip_bom(b"abc"), b"abc");
        assert_eq!(strip_bom(b""), b"");
    }

    #[test]
    fn max_line_bytes_is_16k() {
        // Pin the DoS budget: 16 KiB per record.
        assert_eq!(MAX_LINE_BYTES, 16 * 1024);
    }
}
