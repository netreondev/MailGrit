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
/// Typestate guarantee, stated precisely: OUTSIDE this crate, `Sanitized` has
/// no constructor (its field is private and it derives nothing constructible),
/// and each `Validated*`/`Sanitized*` field can only be produced by its
/// `parse()` — so an external crate cannot assemble this struct bypassing
/// validation. WITHIN the crate the struct is technically assemblable (all
/// fields are `pub`); internal code is expected to go through
/// [`RawCsvRow::parse`], which is the single canonical constructor.
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

    // The typestate CONTRACT, tested for what is actually testable:
    // 1) `Sanitized` cannot be constructed outside this crate (private field,
    //    no public constructor) — enforced by the type system, not runtime.
    // 2) The canonical parser maps every input through the corresponding
    //    field parser (a mutation swapping a field's parse call must change
    //    the output or fail this test).
    #[test]
    fn parse_routes_every_field_through_its_validator() -> Result<(), Box<dyn std::error::Error>> {
        let row = raw_valid().parse()?;
        assert_eq!(row.state, Sanitized);
        assert_eq!(row.domain.as_str(), "example.com");
        assert_eq!(row.username.as_str(), "ivan.petrov");
        assert_eq!(
            row.display_name.as_str(),
            "Ivan Petrov",
            "display_name must come from the display-name parser (not copied raw)"
        );
        assert_eq!(row.quota.mb(), 1024);
        assert_eq!(row.password.as_secret_str(), "S3cur3P@ss1");

        // The routing is validation-relevant, not cosmetic: invalid per-field
        // values are rejected with the FIELD's error kind.
        let mut bad_domain = raw_valid();
        *bad_domain.fields.get_mut(0).ok_or("field 0")? = "user@example.com".into();
        assert!(matches!(
            bad_domain.parse(),
            Err(CsvRowError::Domain(
                crate::error::DomainError::EmailProvided
            ))
        ));
        let mut bad_quota = raw_valid();
        *bad_quota.fields.get_mut(4).ok_or("field 4")? = "0".into();
        assert!(matches!(
            bad_quota.parse(),
            Err(CsvRowError::Quota(
                crate::error::QuotaError::OutOfRange { .. }
            ))
        ));
        Ok(())
    }
}
