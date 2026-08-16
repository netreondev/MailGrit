//! Newtype wrappers for external data (Parse, Don't Validate).
//!
//! Validation is encapsulated in the constructor; the type cannot be created
//! except through the canonical parser-sanitizer. All parsers return `Result`
//! and never panic.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

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
/// sensitivity of the value at its use sites. The manual `Debug` impl never
/// prints the secret — `{:?}` on rows/parse results stays log-safe.
#[derive(Clone, PartialEq, Eq)]
pub struct ValidatedPassword(Arc<str>);

impl std::fmt::Debug for ValidatedPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ValidatedPassword([REDACTED])")
    }
}

impl ValidatedPassword {
    /// Returns the inner value (the secret).
    #[must_use]
    pub fn as_secret_str(&self) -> &str {
        &self.0
    }

    /// Canonical password parser.
    ///
    /// Deliberately does NOT enforce a minimum length or character classes:
    /// this type is shared by Create/Edit/Delete rows (the latter two need no
    /// password at all), and the strength rules differ per deployment (the
    /// `[password_policy]` section). The policy is applied as a NON-blocking
    /// indicator in the UI (`password_policy.rs` — informs rather than blocks),
    /// so an operator may import a CSV whose passwords the server still
    /// accepts. Only the hard transport constraints are enforced here:
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
#[path = "types_tests.rs"]
mod tests;
