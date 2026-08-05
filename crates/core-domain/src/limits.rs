//! Single source of limit constants for all crates. All values are `const`.

/// Maximum number of bulk-upload CSV rows in a single batch.
pub const MAX_CSV_ROWS: usize = 50_000;

/// Maximum length of an individual CSV field (bytes, after UTF-8 decoding).
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

    #[test]
    fn quota_str_matches_number() {
        assert_eq!(
            DEFAULT_QUOTA_MB_STR.parse::<u32>(),
            Ok(DEFAULT_QUOTA_MB),
            "DEFAULT_QUOTA_MB_STR does not match DEFAULT_QUOTA_MB"
        );
    }
}
