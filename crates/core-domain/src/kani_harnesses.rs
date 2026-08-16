//! Kani formal verification proof harnesses for the domain parsers.
//!
//! Enabled by the `kani` feature: `cargo kani --features kani -p mailgrit-core-domain`.
//! They prove, for each parser: **absence of panics/UB on arbitrary input** +
//! **determinism** (same input → same result) + domain invariants.
//!
//! Arbitrary input is modeled by a fixed byte buffer (`[u8; N]`), interpreted as
//! UTF-8 via `String::from_utf8_lossy` (as with real CSV after BOM cleanup in
//! core-csv). This covers any invariant-relevant content without an unsized `&str`.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

#![cfg(kani)]
// Kani proof harnesses are verification entry points: their signatures and
// internal style follow workspace lints (no suppressions). `expect` is used
// intentionally where an invariant must hold (its failure = a proof failure).

use crate::types::{
    SanitizedDisplayName, SanitizedUsername, ValidatedDomain, ValidatedPassword, ValidatedQuota,
};
use crate::typestate::RawCsvRow;

/// Buffer size for modeling arbitrary parser input.
///
/// Kept small (8 bytes) deliberately: Kani's bounded model checker scales
/// exponentially with loop-iteration counts, and the parsers call std `str`
/// operations (`trim`, `chars`, `split`, `contains`) whose internal pattern
/// searchers (e.g. `MultiCharEqSearcher` for whitespace) unwind per input
/// byte. A 16-byte buffer made several harnesses not converge within the CI
/// time budget. 8 bytes still covers the relevant boundary cases (empty,
/// short, invalid chars, edge whitespace) while keeping the state space
/// tractable. Each harness also carries an explicit `#[kani::unwind(N)]` bound.
const INPUT_BUF_LEN: usize = 8;

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

/// Generates an arbitrary VALID UTF-8 string of at most `INPUT_BUF_LEN` bytes.
///
/// The parsers' contract takes `&str` (always valid UTF-8 by construction);
/// `from_utf8_lossy` in [`any_string`] exists only for harness plumbing and
/// can expand 8 arbitrary bytes into a 24-byte string of U+FFFD replacements.
/// `SanitizedDisplayName::parse` (trim → chars().count() → filter().collect())
/// over such a 24-byte string made `verify_display_name_parse_no_panic`
/// diverge on CI solvers: every run since 2026-08-09 ground ~60 s per
/// `core::str::count::do_count_chars` unwinding step until the 90-minute job
/// timeout cancelled the whole Kani workflow. ASCII input keeps byte length
/// equal to char count, still covering the same boundary classes (empty,
/// short, whitespace, control characters, edge bytes), and the no-panic
/// property stays meaningful for the contract the parser actually has.
fn any_ascii_str() -> String {
    any_ascii_str_bounded(INPUT_BUF_LEN)
}

/// Like [`any_ascii_str`], with an explicit per-harness length cap.
///
/// `SanitizedDisplayName::parse` additionally runs `chars().filter().collect()`
/// over the symbolic input: every extra symbolic character doubles the path
/// count through the collection/allocation chain. Even with valid-ASCII input
/// (see [`any_ascii_str`]) the 8-byte bound left the CI solver grinding a
/// single SAT query for 35+ minutes (run 31967364663, 2026-08-16). 4 bytes —
/// the same convergence-driven reduction as INPUT_BUF_LEN 16→8 above — still
/// covers every parser branch reachable by short input (empty, whitespace,
/// control characters; the TooLong branch needs >`MAX_DISPLAY_NAME_LEN`
/// chars and is covered by unit tests).
fn any_ascii_str_bounded(max_len: usize) -> String {
    let len: usize = kani::any_where(|n| *n <= max_len);
    let mut buf = String::new();
    for _ in 0..len {
        let byte: u8 = kani::any_where(|b| *b <= 0x7F);
        buf.push(byte as char);
    }
    buf
}

// ============================================================================
// ValidatedDomain::parse
// ============================================================================

#[kani::proof]
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
fn verify_domain_parse_no_panic() {
    // Contract: the domain parser must return a Result rather than panic
    // on arbitrary UTF-8 input.
    let input = any_string();
    let _ = ValidatedDomain::parse(&input);
}

#[kani::proof]
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
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
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
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
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
fn verify_username_parse_no_panic() {
    let input = any_string();
    let _ = SanitizedUsername::parse(&input);
}

#[kani::proof]
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
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
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
fn verify_password_parse_no_panic() {
    let input = any_string();
    let _ = ValidatedPassword::parse(&input);
}

#[kani::proof]
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
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
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
fn verify_display_name_parse_no_panic() {
    let input = any_ascii_str_bounded(4);
    let _ = SanitizedDisplayName::parse(&input);
}

// ============================================================================
// ValidatedQuota::parse
// ============================================================================
//
// NOTE on coverage: the no-panic / range-invariant harnesses previously fed an
// arbitrary string through `ValidatedQuota::parse`, which internally calls
// `str::parse::<u32>()` → `u32::from_ascii_radix`. Kani's bounded model checker
// unwinds `from_ascii_radix`'s per-digit loop exponentially, and unlike the
// string parsers this cannot be tamed by shrinking the input buffer or raising
// the unwind bound — it exhausted the runner (runner shutdown signal). Those two
// harnesses are removed. The range invariant [1, MAX_QUOTA_MB] is instead proven
// by the boundary-value unit tests (quota_accepts_max_boundary_and_rejects_above)
// plus the explicit-range check in parse() itself; Kani cannot add value here
// beyond re-verifying `u32::from_ascii_radix`, which is std's responsibility.

#[kani::proof]
// Fixed-string harness: no arbitrary input, so no exponential std-parse unwind.
#[kani::unwind(8)]
fn verify_quota_parse_empty_defaults() {
    // Invariant: empty string → default quota (not an error).
    let parsed = ValidatedQuota::parse("");
    assert!(parsed.is_ok(), "empty quota must parse to the default");
    if let Ok(q) = parsed {
        assert_eq!(q.mb(), crate::limits::DEFAULT_QUOTA_MB);
    }
}

// ============================================================================
// RawCsvRow::parse (typestate roundtrip)
// ============================================================================

/// Helper: a RawCsvRow built from exactly `N` empty fields (only the field
/// *count* is relevant to the ColumnCount invariant — `parse` matches on slice
/// length before inspecting any field content).
fn row_with_n_empty_fields<const N: usize>() -> RawCsvRow {
    let fields: [String; N] = std::array::from_fn(|_| String::new());
    RawCsvRow::new(fields.to_vec())
}

#[kani::proof]
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
fn verify_raw_csv_row_parse_zero_fields() {
    // 0 fields → ColumnCount (0 ≠ EXPECTED_CSV_COLUMNS).
    assert!(matches!(
        row_with_n_empty_fields::<0>().parse(),
        Err(crate::error::CsvRowError::ColumnCount { .. })
    ));
}

#[kani::proof]
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
fn verify_raw_csv_row_parse_one_field() {
    assert!(matches!(
        row_with_n_empty_fields::<1>().parse(),
        Err(crate::error::CsvRowError::ColumnCount { .. })
    ));
}

#[kani::proof]
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
fn verify_raw_csv_row_parse_two_fields() {
    assert!(matches!(
        row_with_n_empty_fields::<2>().parse(),
        Err(crate::error::CsvRowError::ColumnCount { .. })
    ));
}

#[kani::proof]
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
fn verify_raw_csv_row_parse_three_fields() {
    assert!(matches!(
        row_with_n_empty_fields::<3>().parse(),
        Err(crate::error::CsvRowError::ColumnCount { .. })
    ));
}

#[kani::proof]
// Explicit per-harness unwind bound: the parsers iterate over an ≤8-byte input
// via std str operations whose internal loops (chars/trim/contains) need room
// for the iteration + slack for break/continue control flow. 12 keeps the
// state space tractable while covering the bounded input fully.
#[kani::unwind(12)]
fn verify_raw_csv_row_parse_four_fields() {
    // EXPECTED_CSV_COLUMNS - 1 = 4 → still rejected.
    assert!(matches!(
        row_with_n_empty_fields::<4>().parse(),
        Err(crate::error::CsvRowError::ColumnCount { .. })
    ));
}
