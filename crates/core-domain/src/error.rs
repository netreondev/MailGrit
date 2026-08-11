//! Domain errors — strict enums via `thiserror`.
//!
//! Principle: no dynamic `anyhow::Error` in module public APIs. Each error is
//! designed so that the caller handles every boundary case at compile time. The
//! UI receives structured codes for message localization.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use std::num::ParseIntError;

/// Domain-name validation error.
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    /// The domain is empty.
    #[error("domain is empty")]
    Empty,
    /// The domain exceeds the allowed length.
    #[error("domain exceeds {max} characters: length {actual}")]
    TooLong {
        /// Actual length (in characters).
        actual: usize,
        /// Allowed maximum.
        max: usize,
    },
    /// The domain contains an invalid character.
    #[error("domain contains invalid character '{char}' at position {pos}")]
    InvalidChar {
        /// The invalid character.
        char: char,
        /// Position (byte offset) of the character.
        pos: usize,
    },
    /// The domain contains '@' — this is an email, not a domain.
    #[error("domain contains '@' (an email was provided instead of a domain)")]
    EmailProvided,
    /// A domain label (the part between dots) is empty or too long.
    #[error("invalid domain label: {0}")]
    InvalidLabel(&'static str),
}

/// Username validation error.
#[derive(Debug, thiserror::Error)]
pub enum UsernameError {
    /// The username is empty.
    #[error("username is empty")]
    Empty,
    /// The allowed length is exceeded.
    #[error("username exceeds {max} characters: length {actual}")]
    TooLong {
        /// Actual length.
        actual: usize,
        /// Allowed maximum.
        max: usize,
    },
    /// Invalid character.
    #[error("username contains invalid character '{char}'")]
    InvalidChar {
        /// The invalid character.
        char: char,
    },
    /// The name starts or ends with a dot or hyphen.
    #[error("username starts or ends with an invalid character")]
    BadEdges,
}

/// Quota (MiB) parsing error.
#[derive(Debug, thiserror::Error)]
pub enum QuotaError {
    /// The integer could not be parsed.
    #[error("invalid quota value: {0}")]
    Parse(#[from] ParseIntError),
    /// The quota is zero or exceeds a reasonable limit.
    #[error("quota out of range: {value} (expected 1..={max})")]
    OutOfRange {
        /// Actual value.
        value: u64,
        /// Allowed maximum.
        max: u64,
    },
}

/// Password parsing error.
#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    /// The password contains a comma (breaks CSV).
    #[error("password contains a comma (not allowed in CSV)")]
    ContainsComma,
    /// The password exceeds the allowed length.
    #[error("password exceeds {max} characters: length {actual}")]
    TooLong {
        /// Actual length.
        actual: usize,
        /// Allowed maximum.
        max: usize,
    },
}

/// Display-name validation error.
#[derive(Debug, thiserror::Error)]
pub enum DisplayNameError {
    /// The display name exceeds the allowed length.
    #[error("display_name exceeds {max} characters: length {actual}")]
    TooLong {
        /// Actual length.
        actual: usize,
        /// Allowed maximum.
        max: usize,
    },
}

/// Aggregated CSV-row error.
#[derive(Debug, thiserror::Error)]
pub enum CsvRowError {
    /// Domain error.
    #[error("field domain: {0}")]
    Domain(#[from] DomainError),
    /// Username error.
    #[error("field username: {0}")]
    Username(#[from] UsernameError),
    /// Password error.
    #[error("field password: {0}")]
    Password(#[from] PasswordError),
    /// Display-name error.
    #[error("field display_name: {0}")]
    DisplayName(#[from] DisplayNameError),
    /// Quota error.
    #[error("field quota_mb: {0}")]
    Quota(#[from] QuotaError),
    /// Wrong number of columns in the row.
    #[error("wrong column count: {actual} (expected {expected})")]
    ColumnCount {
        /// Actual number of columns.
        actual: usize,
        /// Expected number of columns.
        expected: usize,
    },
    /// A required field is missing from the source (no column under flexible
    /// mapping). The typestate pipeline does not catch this (for example, an
    /// empty password passes `ValidatedPassword`), so the check is lifted to the
    /// mapping layer, which reports the name of the skipped field.
    #[error("required field is missing: {field}")]
    MissingRequiredField {
        /// Canonical name of the required field that was not matched to a column.
        field: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_error_display() {
        let e = DomainError::EmailProvided;
        assert_eq!(
            e.to_string(),
            "domain contains '@' (an email was provided instead of a domain)"
        );
    }

    #[test]
    fn username_error_is_send_sync() {
        // Guarantee the errors can be moved between threads (required by app-desktop).
        // Through a closure with trait bounds the compiler verifies Send+Sync for all types.
        fn assert_send_sync<T>()
        where
            T: Send + Sync + 'static,
        {
        }
        assert_send_sync::<UsernameError>();
        assert_send_sync::<DomainError>();
        assert_send_sync::<CsvRowError>();
    }
}
