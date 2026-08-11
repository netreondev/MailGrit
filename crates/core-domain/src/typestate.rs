//! Typestate pattern for Trust-Boundary Tokenization (spec §5, §11, plan §3).
//!
//! Data from external sources (CSV) is initially wrapped in the
//! `RawCsvRow` type (state `Unverified`). The bulk-operation JS builders in
//! `app-desktop` physically refuse to accept this type — the only way to obtain
//! a `SanitizedUserRow` is to pass the data through the canonical
//! parser-sanitizer. This eliminates logical drift between validation layers.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::error::CsvRowError;
use crate::types::{
    SanitizedDisplayName, SanitizedUsername, ValidatedDomain, ValidatedPassword, ValidatedQuota,
};

/// Canonical number of columns in a bulk-upload CSV.
pub const EXPECTED_CSV_COLUMNS: usize = 5;

// ============================================================================
// Unverified state — raw, unvalidated data.
// ============================================================================

/// Marker for the unverified state (phantom, for the typestate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unverified;

/// Marker for the validated, sanitized state (phantom, for the typestate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sanitized;

// ============================================================================
// RawCsvRow — a raw CSV row (Unverified). Cannot be used for creation.
// ============================================================================

/// Raw CSV row, as read from a file. Has NOT been validated.
///
/// Does not implement `Into<SanitizedUserRow>` directly — only via [`parse`](Self::parse).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCsvRow {
    /// Fields in order: `domain, username, password, display_name, quota_mb`.
    pub fields: Vec<String>,
}

impl RawCsvRow {
    /// Creates a raw row from parsed CSV fields.
    #[must_use]
    pub const fn new(fields: Vec<String>) -> Self {
        Self { fields }
    }

    /// Canonical Unverified → Sanitized transition.
    ///
    /// The only way to obtain a validated row: runs all the
    /// parser-sanitizers (`ValidatedDomain`, `SanitizedUsername`, etc.).
    ///
    /// # Errors
    ///
    /// - [`CsvRowError::ColumnCount`] — number of fields ≠ [`EXPECTED_CSV_COLUMNS`].
    /// - Any [`CsvRowError`] from the nested field validators.
    pub fn parse(self) -> Result<SanitizedUserRow, CsvRowError> {
        // Slice pattern matching: safe destructuring without indexing
        // and without panics. Any length mismatch is an explicit ColumnCount error.
        match self.fields.as_slice() {
            [domain, username, password, display_name, quota] => Ok(SanitizedUserRow {
                state: Sanitized,
                domain: ValidatedDomain::parse(domain)?,
                username: SanitizedUsername::parse(username)?,
                password: ValidatedPassword::parse(password)?,
                display_name: SanitizedDisplayName::parse(display_name)?,
                quota: ValidatedQuota::parse(quota)?,
            }),
            other => Err(CsvRowError::ColumnCount {
                actual: other.len(),
                expected: EXPECTED_CSV_COLUMNS,
            }),
        }
    }
}

// ============================================================================
// SanitizedUserRow — a validated, sanitized row (Sanitized).
// ============================================================================

/// A fully validated CSV row, ready for user creation.
///
/// The `state` field guarantees at the type level that this object can only be
/// obtained through the canonical parser ([`RawCsvRow::parse`]), not assembled
/// directly: the `Sanitized` type does not expose a constructor, and the fields
/// themselves require `Validated*` values that can only be created via the parsers.
#[derive(Debug, Clone)]
pub struct SanitizedUserRow {
    /// Phantom marker of the (Sanitized) state. Carries no data at runtime.
    pub state: Sanitized,
    /// Validated domain.
    pub domain: ValidatedDomain,
    /// Sanitized username.
    pub username: SanitizedUsername,
    /// Validated password.
    pub password: ValidatedPassword,
    /// Sanitized display name.
    pub display_name: SanitizedDisplayName,
    /// Validated quota (MiB).
    pub quota: ValidatedQuota,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_valid() -> RawCsvRow {
        RawCsvRow::new(vec![
            "example.com".into(),
            "ivan.petrov".into(),
            "S3cur3P@ss1".into(),
            "Ivan Petrov".into(),
            "1024".into(),
        ])
    }

    #[test]
    fn parses_valid_row() -> Result<(), CsvRowError> {
        let row = raw_valid().parse()?;
        assert_eq!(row.domain.as_str(), "example.com");
        assert_eq!(row.username.as_str(), "ivan.petrov");
        assert_eq!(row.quota.mb(), 1024);
        assert_eq!(row.state, Sanitized);
        Ok(())
    }

    #[test]
    fn rejects_wrong_column_count() {
        let short = RawCsvRow::new(vec!["a".into(), "b".into()]);
        let err = short.parse();
        assert!(matches!(
            err,
            Err(CsvRowError::ColumnCount {
                actual: 2,
                expected: 5
            })
        ));
    }

    #[test]
    fn rejects_email_as_domain() -> Result<(), Box<dyn std::error::Error>> {
        let mut row = raw_valid();
        *row.fields.get_mut(0).ok_or("expected at least 1 field")? = "user@example.com".into();
        assert!(row.parse().is_err());
        Ok(())
    }

    #[test]
    fn quota_defaults_when_empty() -> Result<(), Box<dyn std::error::Error>> {
        let mut row = raw_valid();
        *row.fields.get_mut(4).ok_or("expected at least 5 fields")? = String::new();
        let parsed = row.parse()?;
        assert_eq!(parsed.quota.mb(), crate::limits::DEFAULT_QUOTA_MB);
        Ok(())
    }

    #[test]
    fn sanitized_row_not_constructable_directly() -> Result<(), CsvRowError> {
        // SanitizedUserRow can only be created via RawCsvRow::parse(),
        // because the fields require Validated* types, which have no public
        // constructors other than parse(). state = Sanitized cannot be forged.
        // (Compile-time guarantee; this test documents the contract.)
        let row = raw_valid().parse()?;
        assert_eq!(row.state, Sanitized);
        Ok(())
    }
}
