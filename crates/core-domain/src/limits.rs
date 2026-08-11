//! Single source of limit constants for all crates. All values are `const`.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

/// Maximum number of bulk-upload CSV rows in a single batch.
pub const MAX_CSV_ROWS: usize = 50_000;

/// Maximum length of an individual CSV field, measured in **bytes** (after UTF-8
/// decoding). This is a coarse DoS guard applied at the CSV layer, BEFORE any
/// semantic validation.
///
/// Note the deliberate two-regime design: this limit counts **bytes**, whereas
/// the semantic per-field limits below (`MAX_USERNAME_LEN`,
/// `MAX_DISPLAY_NAME_LEN`, …) count **Unicode chars**. They are not redundant:
/// a field may pass this byte budget and still be rejected by the semantic
/// layer for having too many chars (e.g. a long multi-byte display name), or
/// vice versa. `MAX_CSV_FIELD_BYTES` is intentionally large (4 KiB) so it never
/// pre-empts a semantically-valid field (see `csv_byte_budget_covers_char_limits`).
pub const MAX_CSV_FIELD_BYTES: usize = 4096;

/// Default mailbox quota (MiB) when the `quota_mb` column is empty.
pub const DEFAULT_QUOTA_MB: u32 = 1024;

/// String representation of [`DEFAULT_QUOTA_MB`] for declarative operation profiles.
/// `FieldSpec::default` requires `&'static str`; the link to the number is checked by the test below.
pub const DEFAULT_QUOTA_MB_STR: &str = "1024";

/// Maximum username length.
pub const MAX_USERNAME_LEN: usize = 64;

/// Maximum domain length (RFC 1035 recommends ≤253).
pub const MAX_DOMAIN_LEN: usize = 253;

/// Maximum display_name length.
pub const MAX_DISPLAY_NAME_LEN: usize = 256;

/// Maximum password length (upper bound to guard against anomalous input).
pub const MAX_PASSWORD_LEN: usize = 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_limits_are_nonzero_and_sane() {
        const { assert!(MAX_CSV_ROWS > 0) };
        const { assert!(MAX_CSV_FIELD_BYTES > 0) };
        const { assert!(DEFAULT_QUOTA_MB == 1024) };
        const { assert!(MAX_USERNAME_LEN > 0 && MAX_USERNAME_LEN <= 256) };
        const { assert!(MAX_DOMAIN_LEN > 0 && MAX_DOMAIN_LEN <= 253) };
        const { assert!(MAX_DISPLAY_NAME_LEN > 0) };
        const { assert!(MAX_PASSWORD_LEN > 0) };
    }

    /// The byte-budget DoS guard (`MAX_CSV_FIELD_BYTES`) must be large enough
    /// that any semantically-valid field (worst case: `len` chars × 4 bytes/char
    /// for a maximally multibyte UTF-8 string) fits within it. This guarantees
    /// the byte limit never pre-empts a semantically-valid value — i.e. the only
    /// reason a field can be rejected at the CSV layer is genuine size abuse.
    #[test]
    fn csv_byte_budget_covers_char_limits() {
        const {
            assert!(MAX_USERNAME_LEN * 4 <= MAX_CSV_FIELD_BYTES);
            assert!(MAX_DOMAIN_LEN * 4 <= MAX_CSV_FIELD_BYTES);
            assert!(MAX_DISPLAY_NAME_LEN * 4 <= MAX_CSV_FIELD_BYTES);
            assert!(MAX_PASSWORD_LEN * 4 <= MAX_CSV_FIELD_BYTES);
        }
    }

    #[test]
    fn quota_str_matches_number() {
        assert_eq!(
            DEFAULT_QUOTA_MB_STR.parse::<u32>(),
            Ok(DEFAULT_QUOTA_MB),
            "DEFAULT_QUOTA_MB_STR does not match DEFAULT_QUOTA_MB"
        );
    }
}
