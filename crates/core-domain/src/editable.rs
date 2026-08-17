//! Editable user row (the plain-`String` layer of the table).
//!
//! [`SanitizedUserRow`] is intentionally immutable: its fields are `Arc<str>` newtypes,
//! created only through the canonical parser-sanitizer
//! [`RawCsvRow::parse`](crate::RawCsvRow::parse) (Trust-Boundary Tokenization).
//! Therefore direct editing of values in the UI is impossible.
//!
//! [`EditableUserRow`] solves this task: it is a flat `struct` of `String`s,
//! which:
//! - is **initialized** from parsed CSV (`From<&SanitizedUserRow>`);
//! - is **edited** by the user in the table (cells = `<input>`);
//! - is **re-validated** on execution via
//!   [`EditableUserRow::to_sanitized`](EditableUserRow::to_sanitized),
//!   which runs the row through the typestate pipeline again. This guarantees
//!   that only canonically validated values ever reach the server.
//!
//! The parse-don't-validate principle is preserved: `EditableUserRow` is NOT used
//! anywhere except the UI/editing layer; all lower layers (JS builders in
//! `app-desktop`) accept only `SanitizedUserRow`.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::error::CsvRowError;
use crate::typestate::{RawCsvRow, SanitizedUserRow};

/// The field a validation error pertains to. Used by the UI to highlight
/// a specific table cell (rather than the whole row).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditableField {
    /// The domain field.
    Domain,
    /// The username field.
    Username,
    /// The password field.
    Password,
    /// The `display_name` field.
    DisplayName,
    /// The `quota_mb` field.
    Quota,
}

/// A single validation error on an editable cell: field + human-readable reason.
#[derive(Debug, Clone)]
pub struct EditableFieldError {
    /// Which field is invalid.
    pub field: EditableField,
    /// Human-readable message (from the `Display` of the domain error).
    pub message: String,
}

/// An editable CSV row (flat, mutable carrier for the UI table).
///
/// Fields are in canonical order `domain, username, password, display_name,
/// quota_mb` — the same as [`RawCsvRow::fields`](crate::RawCsvRow::fields),
/// so [`to_sanitized`](Self::to_sanitized) assembles the vector without reordering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditableUserRow {
    /// Domain (plain string, not yet validated).
    pub domain: String,
    /// Username (plain string).
    pub username: String,
    /// Password (plain string).
    pub password: String,
    /// Display name (plain string).
    pub display_name: String,
    /// Quota in MiB as a string (parsed into `ValidatedQuota` on re-validation).
    pub quota: String,
}

impl EditableUserRow {
    /// Empty row for the "Add row" button in the UI. Domain/username/password
    /// are empty, `display_name` is empty, quota is the default string (valid).
    #[must_use]
    pub fn empty_with_default_quota() -> Self {
        Self {
            domain: String::new(),
            username: String::new(),
            password: String::new(),
            display_name: String::new(),
            quota: crate::limits::DEFAULT_QUOTA_MB_STR.to_string(),
        }
    }

    /// Re-validates the row through the typestate pipeline and returns a
    /// [`SanitizedUserRow`] if all fields are valid.
    ///
    /// This is the only bridge between the editable layer and the canonical model:
    /// the user's data is sanitized/validated again, so drift between the UI
    /// and the server contracts is impossible.
    ///
    /// # Errors
    ///
    /// Returns [`CsvRowError`] if at least one field is invalid.
    pub fn to_sanitized(&self) -> Result<SanitizedUserRow, CsvRowError> {
        RawCsvRow::new(vec![
            self.domain.clone(),
            self.username.clone(),
            self.password.clone(),
            self.display_name.clone(),
            self.quota.clone(),
        ])
        .parse()
    }

    /// Whether the password is empty (used to auto-fill empty cells with the generator).
    #[must_use]
    pub fn password_is_empty(&self) -> bool {
        self.password.trim().is_empty()
    }
}

impl From<&SanitizedUserRow> for EditableUserRow {
    fn from(row: &SanitizedUserRow) -> Self {
        Self {
            domain: row.domain.as_str().to_string(),
            username: row.username.as_str().to_string(),
            password: row.password.as_secret_str().to_string(),
            display_name: row.display_name.as_str().to_string(),
            quota: row.quota.mb().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_row() -> EditableUserRow {
        EditableUserRow {
            domain: "example.com".into(),
            username: "ivan.petrov".into(),
            password: "S3cur3P@ss1".into(),
            display_name: "Ivan Petrov".into(),
            quota: "1024".into(),
        }
    }

    #[test]
    fn valid_row_roundtrips_through_sanitized() -> Result<(), CsvRowError> {
        let editable = valid_row();
        let sanitized = editable.to_sanitized()?;
        // The reverse conversion restores the same values (display_name is
        // trimmed by the parser, but here it has no edge whitespace — identical).
        assert_eq!(sanitized.domain.as_str(), "example.com");
        assert_eq!(sanitized.username.as_str(), "ivan.petrov");
        assert_eq!(sanitized.password.as_secret_str(), "S3cur3P@ss1");
        assert_eq!(sanitized.display_name.as_str(), "Ivan Petrov");
        assert_eq!(sanitized.quota.mb(), 1024);
        Ok(())
    }

    #[test]
    fn empty_with_default_quota_has_empty_password() {
        let e = EditableUserRow::empty_with_default_quota();
        assert!(e.password_is_empty());
    }

    // `replace password_is_empty -> bool with true` survived because no test
    // checked the non-empty case. Pin both branches.
    #[test]
    fn password_is_empty_is_false_for_nonempty_password() {
        let e = valid_row(); // password = "S3cur3P@ss1"
        assert!(!e.password_is_empty());
        // Whitespace-only password is still "empty" (trims before checking).
        let mut ws = valid_row();
        ws.password = "   ".into();
        assert!(ws.password_is_empty());
    }

    #[test]
    fn to_sanitized_round_trips_back_to_editable() -> Result<(), CsvRowError> {
        let original = valid_row();
        let sanitized = original.to_sanitized()?;
        let back = EditableUserRow::from(&sanitized);
        assert_eq!(back, original);
        Ok(())
    }
}
