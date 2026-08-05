//! Tests for the local password policy.
//!
#![allow(clippy::unwrap_used, reason = "tests intentionally use unwrap")]

use super::*;

/// A strong password (per the default policy) yields an empty violation list.
#[test]
fn strong_password_has_no_warnings() {
    let policy = PasswordPolicy::default_policy();
    let warnings = policy.validate("S3cur3P@ss1");
    assert!(
        warnings.is_empty(),
        "a strong password should not produce warnings"
    );
}

/// A weak password: too short and missing all classes -> 4 violations.
#[test]
fn weak_short_password_reports_all_classes() {
    let policy = PasswordPolicy::default_policy();
    let warnings = policy.validate("a");
    // "a": length 1 < 8 (TooShort), no uppercase, no digit, no special.
    // Lowercase is present -> MissingLowercase is not added.
    assert_eq!(warnings.len(), 4, "expected 4 violations for \"a\"");
    assert!(warnings.contains(&PasswordWarning::TooShort { min: 8, actual: 1 }));
    assert!(warnings.contains(&PasswordWarning::MissingUppercase));
    assert!(warnings.contains(&PasswordWarning::MissingNumber));
    assert!(warnings.contains(&PasswordWarning::MissingSpecial));
    // Lowercase is present -> this violation is absent.
    assert!(!warnings.contains(&PasswordWarning::MissingLowercase));
}

/// A password of exactly min_len (8) with all classes is valid.
#[test]
fn exactly_min_len_with_all_classes_is_valid() {
    let policy = PasswordPolicy::default_policy();
    // "Aa1!aaaa" — 8 characters, all classes.
    let warnings = policy.validate("Aa1!aaaa");
    assert!(
        warnings.is_empty(),
        "a min_len password with all classes is valid"
    );
}

/// A password of length min_len-1 -> only TooShort (classes are present).
#[test]
fn one_char_short_reports_only_too_short() {
    let policy = PasswordPolicy::default_policy();
    // "Aa1!aaa" — 7 characters (< 8), all classes present.
    let warnings = policy.validate("Aa1!aaa");
    assert_eq!(
        warnings.len(),
        1,
        "only TooShort when all classes are present"
    );
    assert_eq!(warnings[0], PasswordWarning::TooShort { min: 8, actual: 7 });
}

/// Length counts characters, not bytes (UTF-8), and non-Latin lowercase counts.
#[test]
fn length_counts_chars_not_bytes() {
    let policy = PasswordPolicy::default_policy();
    // "Перевір1!" (Ukrainian) — 9 characters (>= 8), all classes: uppercase
    // Cyrillic П, lowercase Cyrillic (incl. the Ukrainian-specific і), digit 1,
    // special "!". Unicode classification counts Cyrillic as lowercase -> no
    // MissingLowercase; valid.
    let warnings = policy.validate("Перевір1!");
    assert!(
        warnings.is_empty(),
        "a 9-character UTF-8 password with all classes is valid: {warnings:?}"
    );
    // "Перевір1" — 8 characters (at the min_len boundary), but NO special
    // character -> exactly one violation (MissingSpecial). TooShort is absent
    // (8 >= 8); letter/digit classes are present.
    let warnings = policy.validate("Перевір1");
    assert_eq!(
        warnings.len(),
        1,
        "an 8-character UTF-8 password without a special char = 1 violation: {warnings:?}"
    );
    assert_eq!(warnings[0], PasswordWarning::MissingSpecial);
    // "еревір1" — 7 characters, no uppercase and no special -> 3 violations.
    let warnings = policy.validate("еревір1");
    assert_eq!(warnings.len(), 3);
    assert!(warnings.contains(&PasswordWarning::TooShort { min: 8, actual: 7 }));
    assert!(warnings.contains(&PasswordWarning::MissingUppercase));
    assert!(warnings.contains(&PasswordWarning::MissingSpecial));
}

/// A relaxed policy (length only) accepts simple passwords.
#[test]
fn relaxed_policy_accepts_simple_password() {
    let policy = PasswordPolicy {
        min_len: 4,
        require_uppercase: false,
        require_lowercase: false,
        require_number: false,
        require_special: false,
    };
    // "abcd" — 4 characters, no classes required -> valid.
    let warnings = policy.validate("abcd");
    assert!(warnings.is_empty());
    // "ab" — shorter than 4 -> only TooShort.
    let warnings = policy.validate("ab");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0], PasswordWarning::TooShort { min: 4, actual: 2 });
}

/// Default policy: fixed values (min_len=8, all classes true).
#[test]
fn default_policy_matches_iredadmin_defaults() {
    let policy = PasswordPolicy::default_policy();
    assert_eq!(policy.min_len, 8);
    assert!(policy.require_uppercase);
    assert!(policy.require_lowercase);
    assert!(policy.require_number);
    assert!(policy.require_special);
}

/// The Default trait equals default_policy() (consistency).
#[test]
fn default_trait_eq_default_policy() {
    assert_eq!(PasswordPolicy::default(), PasswordPolicy::default_policy());
}

/// Display: each violation yields a human-readable (non-empty) string.
#[test]
fn warnings_display_readable() {
    let policy = PasswordPolicy::default_policy();
    let warnings = policy.validate("a");
    let texts: Vec<String> = warnings.iter().map(ToString::to_string).collect();
    // Every string is non-empty.
    for t in &texts {
        assert!(!t.is_empty(), "display text is empty");
    }
    // TooShort mentions the min_len value.
    assert!(
        texts.iter().any(|t| t.contains('8')),
        "TooShort should mention min=8"
    );
}

/// An empty password: only TooShort (classes cannot be checked for an empty
/// string, but all missing-class violations are also added because there is not
/// a single character).
#[test]
fn empty_password_reports_too_short_and_all_missing_classes() {
    let policy = PasswordPolicy::default_policy();
    let warnings = policy.validate("");
    assert_eq!(
        warnings.len(),
        5,
        "empty password = TooShort + 4 missing-class violations"
    );
    assert!(warnings.contains(&PasswordWarning::TooShort { min: 8, actual: 0 }));
}

/// All ASCII punctuation counts as special characters.
#[test]
fn special_chars_include_punctuation() {
    let policy = PasswordPolicy {
        min_len: 1,
        require_uppercase: false,
        require_lowercase: false,
        require_number: false,
        require_special: true,
    };
    for ch in [
        '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '-', '_', '=', '+',
    ] {
        let pwd = format!("A{ch}"); // 2 characters to pass min_len=1.
        let warnings = policy.validate(&pwd);
        assert!(
            warnings.is_empty(),
            "character \"{ch}\" should count as a special character"
        );
    }
}
