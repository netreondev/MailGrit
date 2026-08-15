//! Localization of core-* crate errors at the application layer.
//!
//! By design the core crates (`core-domain`, `core-csv`, `core-storage`) carry no
//! i18n dependencies ("pure logic"). Their errors are strict typed enums
//! ([`thiserror::Error`]). This module maps each variant to a human-readable
//! **localized** string through the translation catalog (`err.*` keys in
//! `locales/app.<lang>.yml`), reading the current global locale.
//!
//! This keeps the core crates clean and provides a single error-translation
//! layer. All user-facing error paths in `app-desktop` must go through this
//! module rather than `e.to_string()` (which would return the core's internal
//! text).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

#![forbid(unsafe_code)]

use mailgrit_core_csv::CsvParseError;
use mailgrit_core_domain::{
    CsvRowError, DisplayNameError, DomainError, EditableField, EditableFieldError, EditableUserRow,
    PasswordError, PasswordWarning, QuotaError, UsernameError,
};

// ============================================================================
// UrlError (app layer)
// ============================================================================

/// Localized message for [`crate::error::UrlError`] (base-URL validation).
#[must_use]
pub fn url_error(e: &crate::error::UrlError) -> String {
    match e {
        crate::error::UrlError::Invalid => tr!("url.invalid"),
        crate::error::UrlError::NotHttps { scheme } => {
            tr!("url.not_https", scheme = scheme)
        }
        crate::error::UrlError::NoHost => tr!("url.no_host"),
    }
}

// ============================================================================
// CsvParseError (core-csv)
// ============================================================================

/// Localized message for [`CsvParseError`] (rejected CSV rows, load errors).
/// Recursively translates nested domain errors.
#[must_use]
pub fn csv_parse_error(e: &CsvParseError) -> String {
    match e {
        CsvParseError::TooManyRows { actual, max } => {
            tr!("err.csv.too_many_rows", actual = actual, max = max)
        }
        CsvParseError::LineTooLong { line_no, max } => {
            tr!("err.csv.line_too_long", line_no = line_no, max = max)
        }
        CsvParseError::FieldTooLong { line_no, max } => {
            tr!("err.csv.field_too_long", line_no = line_no, max = max)
        }
        CsvParseError::Io(io) => tr!("err.csv.io", error = io),
        CsvParseError::InvalidUtf8 { line_no } => {
            tr!("err.csv.invalid_utf8", line_no = line_no)
        }
        CsvParseError::Row { line_no, source } => {
            tr!(
                "err.csv.row",
                line_no = line_no,
                detail = csv_row_error(source)
            )
        }
    }
}

// ============================================================================
// CsvRowError (core-domain) — aggregate of CSV-row errors
// ============================================================================

/// Localized message for [`CsvRowError`] (CSV-row validation error).
#[must_use]
pub fn csv_row_error(e: &CsvRowError) -> String {
    match e {
        CsvRowError::Domain(d) => domain_error(d),
        CsvRowError::Username(u) => username_error(u),
        CsvRowError::Password(p) => password_error(p),
        CsvRowError::DisplayName(d) => display_name_error(d),
        CsvRowError::Quota(q) => quota_error(q),
        CsvRowError::ColumnCount { actual, expected } => {
            tr!("err.column_count", actual = actual, expected = expected)
        }
        CsvRowError::MissingRequiredField { field } => {
            tr!("err.missing_field", field = field)
        }
    }
}

// ============================================================================
// DomainError (core-domain)
// ============================================================================

/// Localized message for [`DomainError`].
#[must_use]
pub fn domain_error(e: &DomainError) -> String {
    match e {
        DomainError::Empty => tr!("err.domain.empty"),
        DomainError::TooLong { actual, max } => {
            tr!("err.domain.too_long", actual = actual, max = max)
        }
        DomainError::InvalidChar { char, pos } => {
            tr!("err.domain.invalid_char", char = char, pos = pos)
        }
        DomainError::EmailProvided => tr!("err.domain.email_provided"),
        DomainError::InvalidLabel(detail) => {
            tr!("err.domain.invalid_label", detail = detail)
        }
    }
}

/// Localized message for [`UsernameError`].
#[must_use]
pub fn username_error(e: &UsernameError) -> String {
    match e {
        UsernameError::Empty => tr!("err.username.empty"),
        UsernameError::TooLong { actual, max } => {
            tr!("err.username.too_long", actual = actual, max = max)
        }
        UsernameError::InvalidChar { char } => {
            tr!("err.username.invalid_char", char = char)
        }
        UsernameError::BadEdges => tr!("err.username.bad_edges"),
    }
}

/// Localized message for [`PasswordError`].
#[must_use]
pub fn password_error(e: &PasswordError) -> String {
    match e {
        PasswordError::ContainsComma => tr!("err.password.contains_comma"),
        PasswordError::TooLong { actual, max } => {
            tr!("err.password.too_long", actual = actual, max = max)
        }
    }
}

/// Localized message for [`DisplayNameError`].
#[must_use]
pub fn display_name_error(e: &DisplayNameError) -> String {
    match e {
        DisplayNameError::TooLong { actual, max } => {
            tr!("err.display_name.too_long", actual = actual, max = max)
        }
    }
}

/// Localized message for [`QuotaError`].
#[must_use]
pub fn quota_error(e: &QuotaError) -> String {
    match e {
        QuotaError::Parse(p) => tr!("err.quota.parse", error = p),
        QuotaError::OutOfRange { value, max } => {
            tr!("err.quota.out_of_range", value = value, max = max)
        }
    }
}

// ============================================================================
// PasswordWarning (core-domain password_policy) — password-strength indicator
// ============================================================================

/// Localized message for [`PasswordWarning`] (the strength-indicator tooltip).
/// It does not block the operation — it only reports server-policy violations.
#[must_use]
pub fn password_warning(w: &PasswordWarning) -> String {
    match w {
        PasswordWarning::TooShort { min, actual } => {
            tr!("err.pw.too_short", actual = actual, min = min)
        }
        PasswordWarning::MissingUppercase => tr!("err.pw.missing_uppercase"),
        PasswordWarning::MissingLowercase => tr!("err.pw.missing_lowercase"),
        PasswordWarning::MissingNumber => tr!("err.pw.missing_number"),
        PasswordWarning::MissingSpecial => tr!("err.pw.missing_special"),
    }
}

// ============================================================================
// EditableUserRow — localized field validation for the UI table
// ============================================================================

/// Localized counterpart of [`EditableUserRow::validate_fields`]: re-runs field
/// validation through the core-domain typestate parsers and returns errors with
/// a translated `message`. A mirror of the core logic (the same set of parsers in
/// the same order), but `message` is the result of `domain_error`/
/// `username_error`/... instead of the core `Display`.
///
/// Used in `editable_table_view` for cell highlighting and tooltips. The core
/// crate stays free of i18n — translation lives at this layer.
#[must_use]
pub fn validate_fields_localized(row: &EditableUserRow) -> Vec<EditableFieldError> {
    let mut errors = Vec::new();
    if let Err(e) = mailgrit_core_domain::ValidatedDomain::parse(&row.domain) {
        errors.push(EditableFieldError {
            field: EditableField::Domain,
            message: domain_error(&e),
        });
    }
    if let Err(e) = mailgrit_core_domain::SanitizedUsername::parse(&row.username) {
        errors.push(EditableFieldError {
            field: EditableField::Username,
            message: username_error(&e),
        });
    }
    if let Err(e) = mailgrit_core_domain::ValidatedPassword::parse(&row.password) {
        errors.push(EditableFieldError {
            field: EditableField::Password,
            message: password_error(&e),
        });
    }
    if let Err(e) = mailgrit_core_domain::SanitizedDisplayName::parse(&row.display_name) {
        errors.push(EditableFieldError {
            field: EditableField::DisplayName,
            message: display_name_error(&e),
        });
    }
    if let Err(e) = mailgrit_core_domain::ValidatedQuota::parse(&row.quota) {
        errors.push(EditableFieldError {
            field: EditableField::Quota,
            message: quota_error(&e),
        });
    }
    errors
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::i18n::tests::LOCALE_TEST_LOCK;

    /// In Ukrainian a core error yields the translated text.
    #[test]
    fn domain_errors_uk_match_previous_display() {
        let _guard = LOCALE_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        rust_i18n::set_locale("uk");
        assert_eq!(
            domain_error(&DomainError::EmailProvided),
            "домен містить '@' (передано email замість домену)"
        );
        rust_i18n::set_locale("en");
    }

    /// CsvRowError with ColumnCount interpolates its values.
    #[test]
    fn csv_row_column_count_en() {
        let _guard = LOCALE_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        rust_i18n::set_locale("en");
        let e = CsvRowError::ColumnCount {
            actual: 4,
            expected: 5,
        };
        let s = csv_row_error(&e);
        assert!(
            s.contains('4') && s.contains('5'),
            "columns not interpolated: {s}"
        );
        rust_i18n::set_locale("en");
    }

    /// PasswordWarning is translated for the current locale.
    #[test]
    fn password_warning_uk() {
        let _guard = LOCALE_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        rust_i18n::set_locale("uk");
        let s = password_warning(&PasswordWarning::MissingNumber);
        assert_eq!(s, "немає цифри (0–9)");
        rust_i18n::set_locale("en");
    }

    /// CsvParseError::Io wraps an io::Error.
    #[test]
    fn csv_parse_io_error_en() {
        let _guard = LOCALE_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        rust_i18n::set_locale("en");
        let e = CsvParseError::Io(std::io::Error::other("boom"));
        let s = csv_parse_error(&e);
        assert!(s.contains("boom"), "io error not interpolated: {s}");
    }
}
