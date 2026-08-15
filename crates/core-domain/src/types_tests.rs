//! Unit tests moved out of the production file (the `#[path]` pattern
//! used across the workspace; keeps the prod file under the 400-line spec).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

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

// `{:?}` must never leak the secret — rows embedding ValidatedPassword are
// debug-logged (ParsedCsv, SanitizedUserRow derive Debug).
#[test]
fn password_debug_output_is_redacted() -> Result<(), PasswordError> {
    let p = ValidatedPassword::parse("S3cret-Value!")?;
    let dbg = format!("{p:?}");
    assert_eq!(dbg, "ValidatedPassword([REDACTED])");
    assert!(!dbg.contains("S3cret"));
    Ok(())
}

#[test]
fn display_name_strips_control_chars() -> Result<(), DisplayNameError> {
    let d = SanitizedDisplayName::parse("Ivan\r\nPetrov")?;
    assert_eq!(d.as_str(), "IvanPetrov");
    Ok(())
}

// ---- Boundary-value coverage (mutation-killing) -------------------------
// Each parser guards with `if len > MAX_*_LEN { Err }`. Without a test at
// the exact boundary (len == MAX, which must succeed), the mutants
// `replace > with ==` and `replace > with >=` survive. Pin every boundary.

#[test]
fn username_accepts_max_length_boundary() -> Result<(), UsernameError> {
    // Exactly MAX_USERNAME_LEN chars — must succeed (the `>` guard rejects only above).
    let max = "a".repeat(MAX_USERNAME_LEN);
    let u = SanitizedUsername::parse(&max)?;
    assert_eq!(u.as_str().chars().count(), MAX_USERNAME_LEN);
    // One more char — must be rejected as TooLong.
    let over = "a".repeat(MAX_USERNAME_LEN + 1);
    assert!(matches!(
        SanitizedUsername::parse(&over),
        Err(UsernameError::TooLong { .. })
    ));
    Ok(())
}

#[test]
fn domain_accepts_max_length_boundary() -> Result<(), DomainError> {
    // Exactly MAX_DOMAIN_LEN chars, built from labels each ≤63 (RFC 1035):
    // 63 + '.' + 62 + '.' + 62 + '.' + 63 = 253 = MAX_DOMAIN_LEN.
    let max = format!(
        "{}.{}.{}.{}",
        "a".repeat(63),
        "a".repeat(62),
        "a".repeat(62),
        "a".repeat(63)
    );
    assert_eq!(max.len(), MAX_DOMAIN_LEN);
    let d = ValidatedDomain::parse(&max)?;
    assert_eq!(d.as_str().len(), MAX_DOMAIN_LEN);
    // MAX_DOMAIN_LEN + 1 = 254, with every label still ≤63
    // (63 + '.' + 63 + '.' + 63 + '.' + 62 = 254). This isolates the
    // total-length guard: the input is refused ONLY for exceeding 253.
    let over = format!(
        "{}.{}.{}.{}",
        "a".repeat(63),
        "a".repeat(63),
        "a".repeat(63),
        "a".repeat(62)
    );
    assert_eq!(over.len(), MAX_DOMAIN_LEN + 1);
    assert!(matches!(
        ValidatedDomain::parse(&over),
        Err(DomainError::TooLong { .. })
    ));
    Ok(())
}

#[test]
fn quota_accepts_max_boundary_and_rejects_above() {
    // Exactly MAX_QUOTA_MB — must succeed.
    assert!(ValidatedQuota::parse(&MAX_QUOTA_MB.to_string()).is_ok());
    // One above — rejected as OutOfRange.
    let over = MAX_QUOTA_MB + 1;
    assert!(matches!(
        ValidatedQuota::parse(&over.to_string()),
        Err(QuotaError::OutOfRange { .. })
    ));
}

#[test]
fn display_name_accepts_max_length_boundary() -> Result<(), DisplayNameError> {
    let max = "a".repeat(MAX_DISPLAY_NAME_LEN);
    let d = SanitizedDisplayName::parse(&max)?;
    assert_eq!(d.as_str().chars().count(), MAX_DISPLAY_NAME_LEN);
    let over = "a".repeat(MAX_DISPLAY_NAME_LEN + 1);
    assert!(matches!(
        SanitizedDisplayName::parse(&over),
        Err(DisplayNameError::TooLong { .. })
    ));
    Ok(())
}

#[test]
fn password_accepts_max_length_boundary() -> Result<(), PasswordError> {
    let max = "x".repeat(MAX_PASSWORD_LEN);
    let p = ValidatedPassword::parse(&max)?;
    assert_eq!(p.as_secret_str().chars().count(), MAX_PASSWORD_LEN);
    let over = "x".repeat(MAX_PASSWORD_LEN + 1);
    assert!(matches!(
        ValidatedPassword::parse(&over),
        Err(PasswordError::TooLong { .. })
    ));
    Ok(())
}

#[test]
fn username_rejects_leading_and_trailing_dot_and_hyphen() {
    // Each of the four BadEdges conditions independently must trigger — if
    // any `||` is mutated to `&&`, one of these would slip through.
    assert!(matches!(
        SanitizedUsername::parse("-ivan"),
        Err(UsernameError::BadEdges)
    ));
    assert!(matches!(
        SanitizedUsername::parse("ivan."),
        Err(UsernameError::BadEdges)
    ));
}

#[test]
fn max_quota_mb_constant_is_one_tib() {
    // `replace * with +` on the constant would change the value — pin it.
    assert_eq!(MAX_QUOTA_MB, 1_048_576);
}
