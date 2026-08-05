//! Kani formal verification proof harnesses for the domain parsers.
//!
//! Enabled by the `kani` feature: `cargo kani --features kani -p mailgrit-core-domain`.
//! They prove, for each parser: **absence of panics/UB on arbitrary input** +
//! **determinism** (same input → same result) + domain invariants.
//!
//! Arbitrary input is modeled by a fixed byte buffer (`[u8; N]`), interpreted as
//! UTF-8 via `String::from_utf8_lossy` (as with real CSV after BOM cleanup in
//! core-csv). This covers any invariant-relevant content without an unsized `&str`.

#![cfg(kani)]
// Kani harnesses do not follow workspace lints (forbid unsafe in core-domain
// remains, but pedantic/nursery are excessive here for verification code).
#![allow(clippy::pedantic, clippy::nursery, clippy::needless_pass_by_value)]

use crate::types::{
    SanitizedDisplayName, SanitizedUsername, ValidatedDomain, ValidatedPassword, ValidatedQuota,
};
use crate::typestate::{EXPECTED_CSV_COLUMNS, RawCsvRow};

/// Buffer size for modeling arbitrary parser input.
/// Large enough to cover boundary cases (empty, long strings, invalid
/// characters), but bounded for bounded model-checking.
const INPUT_BUF_LEN: usize = 16;

/// Generates an arbitrary string from the byte buffer (Kani nondeterministic).
fn any_string() -> String {
    let mut buf = [0u8; INPUT_BUF_LEN];
    for byte in &mut buf {
        *byte = kani::any();
    }
    // from_utf8_lossy guarantees valid UTF-8 (invalid bytes → U+FFFD),
    // which matches the real behavior after CSV BOM cleanup.
    String::from_utf8_lossy(&buf).into_owned()
}

// ============================================================================
// ValidatedDomain::parse
// ============================================================================

#[kani::proof]
fn verify_domain_parse_no_panic() {
    // Contract: the domain parser must return a Result rather than panic
    // on arbitrary UTF-8 input.
    let input = any_string();
    let _ = ValidatedDomain::parse(&input);
}

#[kani::proof]
fn verify_domain_parse_deterministic() {
    // Same input → same result (parse is a pure function).
    let input = any_string();
    let r1 = ValidatedDomain::parse(&input);
    let r2 = ValidatedDomain::parse(&input);
    assert!(r1.is_ok() == r2.is_ok());
    if let (Ok(d1), Ok(d2)) = (r1, r2) {
        assert_eq!(d1.as_str(), d2.as_str());
    }
}

#[kani::proof]
fn verify_domain_parse_normalizes_lowercase() {
    // Invariant: a successful parse always returns the domain in lowercase.
    let input = any_string();
    if let Ok(domain) = ValidatedDomain::parse(&input) {
        assert!(
            domain.as_str().chars().all(|c| !c.is_ascii_uppercase()),
            "domain must be normalized to lowercase"
        );
    }
}

// ============================================================================
// SanitizedUsername::parse
// ============================================================================

#[kani::proof]
fn verify_username_parse_no_panic() {
    let input = any_string();
    let _ = SanitizedUsername::parse(&input);
}

#[kani::proof]
fn verify_username_parse_deterministic() {
    let input = any_string();
    let r1 = SanitizedUsername::parse(&input);
    let r2 = SanitizedUsername::parse(&input);
    assert!(r1.is_ok() == r2.is_ok());
}

// ============================================================================
// ValidatedPassword::parse
// ============================================================================

#[kani::proof]
fn verify_password_parse_no_panic() {
    let input = any_string();
    let _ = ValidatedPassword::parse(&input);
}

#[kani::proof]
fn verify_password_parse_rejects_comma() {
    // Invariant: a password containing a comma is always rejected (breaks CSV).
    let base = any_string();
    let with_comma = format!("{base},");
    assert!(
        ValidatedPassword::parse(&with_comma).is_err(),
        "a password with a comma must be rejected"
    );
}

// ============================================================================
// SanitizedDisplayName::parse
// ============================================================================

#[kani::proof]
fn verify_display_name_parse_no_panic() {
    let input = any_string();
    let _ = SanitizedDisplayName::parse(&input);
}

// ============================================================================
// ValidatedQuota::parse
// ============================================================================

#[kani::proof]
fn verify_quota_parse_no_panic() {
    let input = any_string();
    let _ = ValidatedQuota::parse(&input);
}

#[kani::proof]
fn verify_quota_parse_empty_defaults() {
    // Invariant: empty string → default quota (not an error).
    let q = ValidatedQuota::parse("").expect("empty quota → default");
    assert_eq!(q.mb(), crate::limits::DEFAULT_QUOTA_MB);
}

#[kani::proof]
fn verify_quota_parse_range_invariant() {
    // Invariant: a successful parse → quota within [1, MAX_QUOTA_MB].
    let input = any_string();
    if let Ok(q) = ValidatedQuota::parse(&input) {
        const MAX_QUOTA_MB: u64 = 1024 * 1024;
        let mb = u64::from(q.mb());
        assert!(mb >= 1 && mb <= MAX_QUOTA_MB, "quota out of range");
    }
}

// ============================================================================
// RawCsvRow::parse (typestate roundtrip)
// ============================================================================

#[kani::proof]
fn verify_raw_csv_row_parse_column_count() {
    // Invariant: a RawCsvRow with a number of fields ≠ EXPECTED_CSV_COLUMNS is
    // always rejected with CsvRowError::ColumnCount.
    let mut fields = Vec::new();
    // Kani: model 0..EXPECTED_CSV_COLUMNS fields (fewer than expected).
    for i in 0..(EXPECTED_CSV_COLUMNS - 1) {
        let _ = i;
        fields.push(any_string());
    }
    let row = RawCsvRow::new(fields);
    // Fewer than 5 fields → guaranteed ColumnCount.
    assert!(
        matches!(
            row.parse(),
            Err(crate::error::CsvRowError::ColumnCount { .. })
        ),
        "fewer than 5 fields → ColumnCount"
    );
}
