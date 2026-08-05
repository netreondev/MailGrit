//! Server-side password policy.
//!
//! The `core-domain` crate stays pure logic with no serde/network dependencies
//! (see the principle in [`crate`]). So [`PasswordPolicy`] here is only a
//! description of the rules and the [`validate`](PasswordPolicy::validate)
//! function, which returns the list of violations. The rules themselves are
//! loaded from `config.toml` at the application layer
//! ([`settings`](../../mailgrit_app_desktop/settings/index.html)) and converted
//! into [`PasswordPolicy`] at the boundary (parse-don't-validate).
//!
//! Defaults match iRedAdmin: `min_len = 8`, with uppercase/lowercase letters, a
//! digit, and a special character required. The password-strength indicator in
//! the editable table checks passwords against this policy and highlights (with a
//! warning icon and tooltip) rows whose password is weaker — this informs rather
//! than blocks the operation. Enforcing the required character classes in the
//! generator settings guarantees that generated passwords always pass the policy.

/// Password policy: minimum length and required character classes.
///
/// Built via [`default_policy`](Self::default_policy) or constructed explicitly
/// from configuration. The [`validate`](Self::validate) function never panics and
/// returns the list of violations (parse-don't-validate): an empty list means the
/// password satisfies the policy.
//
// struct_excessive_bools: 4 independent character-class requirement flags
// (uppercase/lowercase/number/special) — this is the canonical form of a password
// policy (the same 4 flags in iRedAdmin settings.py and in most systems). Grouping
// them into bitflags/enum hurts readability for no benefit: the user sees explicit
// on/off toggles in config.toml. The same established pattern as in AppState.
#[allow(
    clippy::struct_excessive_bools,
    reason = "4 independent character-class flags are the canonical form of a password policy"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordPolicy {
    /// Minimum password length (in characters). Matches the iRedAdmin default of 8.
    pub min_len: usize,
    /// Whether at least one uppercase letter is required (Unicode `is_uppercase`:
    /// Latin `A`-`Z`, Greek, Cyrillic, ...).
    pub require_uppercase: bool,
    /// Whether at least one lowercase letter is required (Unicode `is_lowercase`:
    /// Latin `a`-`z`, Greek, Cyrillic, ...).
    pub require_lowercase: bool,
    /// Whether at least one digit is required (Unicode `is_numeric`: `0`-`9`, ...).
    pub require_number: bool,
    /// Whether at least one special character is required (ASCII punctuation:
    /// `!@#$%^&*`, etc.).
    pub require_special: bool,
}

impl PasswordPolicy {
    /// Default policy: `min_len = 8`, all character classes required.
    /// Matches the standard iRedAdmin default (without separate
    /// `PASSWORD_MIN_LEN`/`PASSWORD_REQUIRE_*` settings in `settings.py`).
    #[must_use]
    pub const fn default_policy() -> Self {
        Self {
            min_len: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_number: true,
            require_special: true,
        }
    }

    /// Validates the password against the policy and returns the list of violations.
    ///
    /// Returns an empty `Vec` if the password satisfies every active rule. Never
    /// panics: the length is computed via `chars().count()` and comparisons are
    /// strict (`<`), with no overflowing arithmetic.
    ///
    /// Violations are accumulated in deterministic order: length first, then the
    /// character classes in declaration order (uppercase -> lowercase -> digit ->
    /// special).
    #[must_use]
    pub fn validate(&self, password: &str) -> Vec<PasswordWarning> {
        let mut warnings = Vec::new();
        // Length in characters (not bytes): the password "password" is shorter
        // than 8 even though it occupies 12 bytes in UTF-8. `chars().count()` is
        // costly for huge strings, but the password is bounded by
        // `MAX_PASSWORD_LEN` already at the typestate-parsing stage.
        let len = password.chars().count();
        if len < self.min_len {
            warnings.push(PasswordWarning::TooShort {
                min: self.min_len,
                actual: len,
            });
        }
        // Single-pass character classification (no allocations).
        // Uses Unicode-aware methods (`is_uppercase`/`is_lowercase`/`is_numeric`)
        // rather than ASCII-only: passwords often contain non-Latin letters, and a
        // lowercase letter should satisfy require_lowercase. Special characters are
        // ASCII punctuation (`is_ascii_punctuation`) so that fullwidth/index signs
        // are not counted as special (predictability).
        let (mut upper, mut lower, mut number, mut special) = (false, false, false, false);
        for ch in password.chars() {
            if ch.is_uppercase() {
                upper = true;
            } else if ch.is_lowercase() {
                lower = true;
            } else if ch.is_numeric() {
                number = true;
            } else if is_special_char(ch) {
                special = true;
            }
        }
        if self.require_uppercase && !upper {
            warnings.push(PasswordWarning::MissingUppercase);
        }
        if self.require_lowercase && !lower {
            warnings.push(PasswordWarning::MissingLowercase);
        }
        if self.require_number && !number {
            warnings.push(PasswordWarning::MissingNumber);
        }
        if self.require_special && !special {
            warnings.push(PasswordWarning::MissingSpecial);
        }
        warnings
    }
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self::default_policy()
    }
}

/// Special-character predicate for the password policy.
///
/// Special characters are ASCII punctuation (`!`, `@`, `#`, `$`, `%`, `^`, `&`,
/// `*`, brackets, punctuation signs, etc.). Control characters and space are NOT
/// considered special (they are already forbidden at the typestate password-
/// parsing stage, so they cannot appear here, but the classification stays
/// consistent).
const fn is_special_char(ch: char) -> bool {
    ch.is_ascii_punctuation()
}

/// A single password-policy violation.
///
/// Implements [`std::fmt::Display`] for human-readable output in the password-
/// strength indicator tooltip in the table. Variant order is fixed
/// (length -> classes) and is preserved by [`PasswordPolicy::validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordWarning {
    /// The password is shorter than the minimum (`actual < min`).
    TooShort {
        /// Required minimum length.
        min: usize,
        /// Actual password length.
        actual: usize,
    },
    /// An uppercase letter is required but missing.
    MissingUppercase,
    /// A lowercase letter is required but missing.
    MissingLowercase,
    /// A digit is required but missing.
    MissingNumber,
    /// A special character is required but missing.
    MissingSpecial,
}

impl std::fmt::Display for PasswordWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { min, actual } => {
                write!(f, "password too short: {actual} characters, need >= {min}")
            }
            Self::MissingUppercase => write!(f, "missing uppercase letter (A-Z)"),
            Self::MissingLowercase => write!(f, "missing lowercase letter (a-z)"),
            Self::MissingNumber => write!(f, "missing digit (0-9)"),
            Self::MissingSpecial => write!(f, "missing special character (!@#$%^&*...)"),
        }
    }
}

#[cfg(test)]
#[path = "password_policy_tests.rs"]
mod tests;
