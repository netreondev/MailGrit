//! Newtype wrappers for external data (Parse, Don't Validate).
//!
//! Validation is encapsulated in the constructor; the type cannot be created
//! except through the canonical parser-sanitizer. All parsers return `Result`
//! and never panic.

use crate::error::{DisplayNameError, DomainError, PasswordError, QuotaError, UsernameError};
use crate::limits::{
    DEFAULT_QUOTA_MB, MAX_DISPLAY_NAME_LEN, MAX_DOMAIN_LEN, MAX_PASSWORD_LEN, MAX_USERNAME_LEN,
};
use std::sync::Arc;

/// Reasonable upper bound for mailbox quota (1 TiB).
pub const MAX_QUOTA_MB: u64 = 1024 * 1024;

/// Sanitized username. Allows `[a-zA-Z0-9._-]`, with no leading/trailing dot or hyphen.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SanitizedUsername(Arc<str>);

impl SanitizedUsername {
    /// Returns the inner value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Canonical username parser-sanitizer.
    ///
    /// # Errors
    /// - [`UsernameError::Empty`] — empty input after trimming.
    /// - [`UsernameError::TooLong`] — length > [`MAX_USERNAME_LEN`].
    /// - [`UsernameError::BadEdges`] — starts/ends with `.` or `-`.
    /// - [`UsernameError::InvalidChar`] — a character outside `[a-zA-Z0-9._-]`.
    pub fn parse(input: &str) -> Result<Self, UsernameError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(UsernameError::Empty);
        }
        let len = trimmed.chars().count();
        if len > MAX_USERNAME_LEN {
            return Err(UsernameError::TooLong {
                actual: len,
                max: MAX_USERNAME_LEN,
            });
        }
        if trimmed.starts_with('.')
            || trimmed.ends_with('.')
            || trimmed.starts_with('-')
            || trimmed.ends_with('-')
        {
            return Err(UsernameError::BadEdges);
        }
        for ch in trimmed.chars() {
            if !is_valid_username_char(ch) {
                return Err(UsernameError::InvalidChar { char: ch });
            }
        }
        Ok(Self(Arc::from(trimmed)))
    }
}

const fn is_valid_username_char(ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_')
}

/// Validated domain. Contains no `@`, no empty labels, ≤[`MAX_DOMAIN_LEN`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedDomain(Arc<str>);

impl ValidatedDomain {
    /// Returns the inner value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Canonical domain parser-sanitizer.
    ///
    /// # Errors
    /// - [`DomainError::Empty`] — empty input.
    /// - [`DomainError::EmailProvided`] — contains `@`.
    /// - [`DomainError::TooLong`] — length > [`MAX_DOMAIN_LEN`].
    /// - [`DomainError::InvalidLabel`] — empty label / >63 characters / trailing dot.
    /// - [`DomainError::InvalidChar`] — a character outside `[a-zA-Z0-9-]`.
    pub fn parse(input: &str) -> Result<Self, DomainError> {
        let trimmed = input.trim();
        let len = trimmed.len();
        if len == 0 {
            return Err(DomainError::Empty);
        }
        if trimmed.contains('@') {
            return Err(DomainError::EmailProvided);
        }
        if len > MAX_DOMAIN_LEN {
            return Err(DomainError::TooLong {
                actual: len,
                max: MAX_DOMAIN_LEN,
            });
        }
        let mut last_was_dot = true; // start is treated as "right after a dot"
        let mut label_len: usize = 0;
        for (idx, ch) in trimmed.char_indices() {
            match ch {
                '.' => {
                    if last_was_dot {
                        return Err(DomainError::InvalidLabel("empty domain label"));
                    }
                    if label_len > 63 {
                        return Err(DomainError::InvalidLabel("label exceeds 63 characters"));
                    }
                    last_was_dot = true;
                    label_len = 0;
                }
                c if is_valid_domain_char(c) => {
                    last_was_dot = false;
                    label_len = label_len.saturating_add(1);
                }
                c => return Err(DomainError::InvalidChar { char: c, pos: idx }),
            }
        }
        // Length check for the last label: inside the loop the length is only
        // checked when a dot is encountered (the line above), so the final label
        // without a trailing dot would otherwise remain unchecked — bypassing RFC 1035.
        if label_len > 63 {
            return Err(DomainError::InvalidLabel("label exceeds 63 characters"));
        }
        if last_was_dot {
            return Err(DomainError::InvalidLabel("domain ends with a dot"));
        }
        // Domains are case-insensitive — normalize to lowercase.
        let normalized = trimmed.to_ascii_lowercase();
        Ok(Self(Arc::from(normalized.as_str())))
    }
}

const fn is_valid_domain_char(ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-')
}

/// Validated mailbox quota (MiB). Range 1..=[`MAX_QUOTA_MB`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedQuota(u32);

impl ValidatedQuota {
    /// Returns the quota value in MiB.
    #[must_use]
    pub const fn mb(self) -> u32 {
        self.0
    }

    /// Canonical quota parser. Empty string → default [`DEFAULT_QUOTA_MB`].
    ///
    /// # Errors
    /// - [`QuotaError::Parse`] — not an integer.
    /// - [`QuotaError::OutOfRange`] — `0` or > `MAX_QUOTA_MB`.
    pub fn parse(input: &str) -> Result<Self, QuotaError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(Self(DEFAULT_QUOTA_MB));
        }
        let value = trimmed.parse::<u32>()?;
        if value == 0 || u64::from(value) > MAX_QUOTA_MB {
            return Err(QuotaError::OutOfRange {
                value: u64::from(value),
                max: MAX_QUOTA_MB,
            });
        }
        Ok(Self(value))
    }

    /// Default quota (when the column is empty).
    #[must_use]
    pub const fn default_quota() -> Self {
        Self(DEFAULT_QUOTA_MB)
    }
}

/// Validated display name. Control characters (CR/LF/NUL) are rejected
/// (protection against log/UI injection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedDisplayName(Arc<str>);

impl SanitizedDisplayName {
    /// Returns the inner value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Canonical display name parser-sanitizer.
    ///
    /// # Errors
    /// - [`DisplayNameError::TooLong`] — length > [`MAX_DISPLAY_NAME_LEN`].
    pub fn parse(input: &str) -> Result<Self, DisplayNameError> {
        let trimmed = input.trim();
        let len = trimmed.chars().count();
        if len > MAX_DISPLAY_NAME_LEN {
            return Err(DisplayNameError::TooLong {
                actual: len,
                max: MAX_DISPLAY_NAME_LEN,
            });
        }
        // Remove control characters (CR/LF/NUL and other C0 characters).
        let sanitized: String = trimmed.chars().filter(|&c| !c.is_control()).collect();
        Ok(Self(Arc::from(sanitized.as_str())))
    }
}

/// Validated password. Contains no comma (breaks CSV), length ≤[`MAX_PASSWORD_LEN`].
///
/// Note: the value is stored as `Arc<str>` and is NOT zeroed from memory when
/// it goes out of scope (memory zeroization is not implemented at this layer).
/// Access is via [`as_secret_str`](Self::as_secret_str), to emphasize the
/// sensitivity of the value at its use sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPassword(Arc<str>);

impl ValidatedPassword {
    /// Returns the inner value (the secret).
    #[must_use]
    pub fn as_secret_str(&self) -> &str {
        &self.0
    }

    /// Canonical password parser.
    ///
    /// # Errors
    /// - [`PasswordError::TooLong`] — length > [`MAX_PASSWORD_LEN`].
    /// - [`PasswordError::ContainsComma`] — contains `,` (breaks CSV).
    pub fn parse(input: &str) -> Result<Self, PasswordError> {
        let len = input.chars().count();
        if len > MAX_PASSWORD_LEN {
            return Err(PasswordError::TooLong {
                actual: len,
                max: MAX_PASSWORD_LEN,
            });
        }
        if input.contains(',') {
            return Err(PasswordError::ContainsComma);
        }
        Ok(Self(Arc::from(input)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_valid() -> Result<(), UsernameError> {
        let u = SanitizedUsername::parse("ivan.petrov")?;
        assert_eq!(u.as_str(), "ivan.petrov");
        Ok(())
    }

    #[test]
    fn username_rejects_empty() {
        assert!(matches!(
            SanitizedUsername::parse("  "),
            Err(UsernameError::Empty)
        ));
    }

    #[test]
    fn username_rejects_bad_edges() {
        assert!(matches!(
            SanitizedUsername::parse(".ivan"),
            Err(UsernameError::BadEdges)
        ));
        assert!(matches!(
            SanitizedUsername::parse("ivan-"),
            Err(UsernameError::BadEdges)
        ));
    }

    #[test]
    fn username_rejects_invalid_char() {
        assert!(matches!(
            SanitizedUsername::parse("ivan petrov"),
            Err(UsernameError::InvalidChar { char: ' ' })
        ));
    }

    #[test]
    fn domain_normalizes_lowercase() -> Result<(), DomainError> {
        let d = ValidatedDomain::parse("Example.COM")?;
        assert_eq!(d.as_str(), "example.com");
        Ok(())
    }

    #[test]
    fn domain_rejects_email() {
        assert!(matches!(
            ValidatedDomain::parse("user@example.com"),
            Err(DomainError::EmailProvided)
        ));
    }

    #[test]
    fn domain_rejects_empty_label() {
        assert!(ValidatedDomain::parse("example..com").is_err());
    }

    #[test]
    fn domain_rejects_trailing_dot() {
        assert!(ValidatedDomain::parse("example.com.").is_err());
    }

    // Regression: the length of the LAST label was not checked (the check only
    // existed in the '.' branch). A final label without a trailing dot bypassed the 63 limit.
    #[test]
    fn domain_rejects_oversized_last_label() {
        // 63 characters — boundary, accepted.
        let ok = format!("example.{}", "a".repeat(63));
        assert!(ValidatedDomain::parse(&ok).is_ok());
        // 64 characters — exceeds RFC 1035, rejected.
        let too_long = format!("example.{}", "a".repeat(64));
        assert!(matches!(
            ValidatedDomain::parse(&too_long),
            Err(DomainError::InvalidLabel(_))
        ));
    }

    // Regression hardening: an oversized middle label is still caught
    // (the in-loop check is not broken after adding the post-loop check).
    #[test]
    fn domain_rejects_oversized_middle_label() {
        let too_long = format!("{}.com", "a".repeat(64));
        assert!(matches!(
            ValidatedDomain::parse(&too_long),
            Err(DomainError::InvalidLabel(_))
        ));
        // 63 in the middle — boundary, passes.
        let ok = format!("{}.com", "a".repeat(63));
        assert!(ValidatedDomain::parse(&ok).is_ok());
    }

    // The FIRST label also obeys the limit (it is the "last" label for a single-label
    // domain) — pin down that a single-label domain with >63 characters is rejected.
    #[test]
    fn domain_rejects_oversized_single_label() {
        assert!(ValidatedDomain::parse(&"a".repeat(64)).is_err());
        assert!(ValidatedDomain::parse(&"a".repeat(63)).is_ok());
    }

    // Three-label domain with an oversized final TLD — the primary bug case.
    #[test]
    fn domain_rejects_oversized_final_tld_three_labels() {
        let bad = format!("a.b.{}", "x".repeat(64));
        assert!(matches!(
            ValidatedDomain::parse(&bad),
            Err(DomainError::InvalidLabel(_))
        ));
    }

    // A valid multi-segment domain with a long (63) final TLD is accepted.
    #[test]
    fn domain_accepts_long_valid_tld() {
        let ok = format!("sub.example.{}", "com".repeat(20));
        // 60 characters — within 63.
        let tld = ok.rsplit('.').next().map_or(0, str::len);
        assert!(tld <= 63);
        assert!(ValidatedDomain::parse(&ok).is_ok());
    }

    #[test]
    fn quota_defaults_on_empty() -> Result<(), QuotaError> {
        let q = ValidatedQuota::parse("")?;
        assert_eq!(q.mb(), DEFAULT_QUOTA_MB);
        Ok(())
    }

    #[test]
    fn quota_rejects_zero_and_huge() {
        assert!(ValidatedQuota::parse("0").is_err());
        assert!(ValidatedQuota::parse("999999999999").is_err());
    }

    #[test]
    fn quota_accepts_valid() -> Result<(), QuotaError> {
        let q = ValidatedQuota::parse("2048")?;
        assert_eq!(q.mb(), 2048);
        Ok(())
    }

    #[test]
    fn password_rejects_comma() {
        assert!(matches!(
            ValidatedPassword::parse("pass,word"),
            Err(PasswordError::ContainsComma)
        ));
    }

    #[test]
    fn display_name_strips_control_chars() -> Result<(), DisplayNameError> {
        let d = SanitizedDisplayName::parse("Ivan\r\nPetrov")?;
        assert_eq!(d.as_str(), "IvanPetrov");
        Ok(())
    }
}
