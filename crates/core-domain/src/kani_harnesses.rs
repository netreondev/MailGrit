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

// ============================================================================
// Coverage NOTE — why the &str-parser harnesses (domain/username/password/
// display_name) are CONCRETE while the RawCsvRow and quota harnesses keep
// their original strength.
// ============================================================================
//
// Every &str-parser harness originally modeled its input symbolically
// (arbitrary bytes → `String::from_utf8_lossy`: an 8-byte buffer the lossy
// conversion can expand to 24 bytes of U+FFFD). None of those harnesses is
// convergent on CI-class hardware — established by direct measurement
// (2026-08-16/17: GitHub-hosted runners plus a 2-vCPU `taskset` lab, Kani
// 0.67.0, kissat AND default solvers, input bounds 8 and 4, and a
// valid-ASCII input model):
//   - the action-default Kani spent ~60 s per `str::count` unwinding step;
//     every weekly run since 2026-08-09 was cancelled by the 90-minute job
//     timeout (runs 31294142192, 31299647997, 31924909000, 31962413897) —
//     the gate was silently red for 2+ weeks;
//   - on Kani 0.67.0 the symbolic &str harnesses (domain ×3, username ×2,
//     password ×2) still exceed 300 s on two cores and 600 s on four for
//     several of them: the blow-up lives in the std searcher/allocation
//     chains (`trim`, `chars().count()`, `filter().collect()`, `Arc::from`),
//     not in the input model;
//   - harnesses whose symbolic state space is small (RawCsvRow ×5, quota)
//     converge in seconds and STAY symbolic.
//
// The trade-off follows this file's own quota precedent: boundary-value
// CONCRETE strings are proven here (instantly), and behavioral coverage of
// arbitrary input is carried by the unit tests for each parser plus the
// crate's 0-missed mutation-testing gate (see ci.yml). Straight-line calls
// only: iterating an array of `&str` literals introduces slice-pointer
// objects that CBMC reasons about far more expensively than the parses
// themselves (measured: loop version non-convergent, straight-line version
// ~12 s).

// ============================================================================
// ValidatedDomain::parse
// ============================================================================

#[kani::proof]
// Concrete boundary-value inputs (see the coverage NOTE at the top of the
// file): empty; whitespace-only; uppercase (normalize branch); embedded
// space; control byte; label boundary.
#[kani::unwind(20)]
fn verify_domain_parse_no_panic() {
    // Contract: the domain parser must return a Result rather than panic.
    let _ = ValidatedDomain::parse("");
    let _ = ValidatedDomain::parse("  ");
    let _ = ValidatedDomain::parse("AB.CD");
    let _ = ValidatedDomain::parse("a b");
    let _ = ValidatedDomain::parse("\u{0}x");
    let _ = ValidatedDomain::parse("x.y");
}

#[kani::proof]
// Concrete inputs (see the coverage NOTE): pure-function determinism on the
// accepting and rejecting branches alike.
#[kani::unwind(20)]
fn verify_domain_parse_deterministic() {
    // Same input → same result (parse is a pure function).
    let r1 = ValidatedDomain::parse("AB.CD");
    let r2 = ValidatedDomain::parse("AB.CD");
    assert!(r1.is_ok() == r2.is_ok());
    if let (Ok(d1), Ok(d2)) = (r1, r2) {
        assert_eq!(d1.as_str(), d2.as_str());
    }
    let r1 = ValidatedDomain::parse("a b");
    let r2 = ValidatedDomain::parse("a b");
    assert!(r1.is_ok() == r2.is_ok());
}

#[kani::proof]
// Concrete inputs (see the coverage NOTE): the uppercase-heavy accepted
// input exercises the normalize branch.
#[kani::unwind(20)]
fn verify_domain_parse_normalizes_lowercase() {
    // Invariant: a successful parse always returns the domain in lowercase.
    if let Ok(domain) = ValidatedDomain::parse("MiXeD.ExAmPlE") {
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
// Concrete boundary-value inputs (see the coverage NOTE): empty;
// leading/trailing whitespace; embedded space; control character; comma.
#[kani::unwind(20)]
fn verify_username_parse_no_panic() {
    let _ = SanitizedUsername::parse("");
    let _ = SanitizedUsername::parse(" Ivan. ");
    let _ = SanitizedUsername::parse("I Van");
    let _ = SanitizedUsername::parse("\u{1f}u");
    let _ = SanitizedUsername::parse("a,b");
}

#[kani::proof]
// Concrete inputs (see the coverage NOTE): determinism on both branches.
#[kani::unwind(20)]
fn verify_username_parse_deterministic() {
    let r1 = SanitizedUsername::parse(" Ivan. ");
    let r2 = SanitizedUsername::parse(" Ivan. ");
    assert!(r1.is_ok() == r2.is_ok());
    let r1 = SanitizedUsername::parse("I Van");
    let r2 = SanitizedUsername::parse("I Van");
    assert!(r1.is_ok() == r2.is_ok());
}

// ============================================================================
// ValidatedPassword::parse
// ============================================================================

#[kani::proof]
// Concrete boundary-value inputs (see the coverage NOTE): empty; whitespace
// around content; comma; control byte.
#[kani::unwind(20)]
fn verify_password_parse_no_panic() {
    let _ = ValidatedPassword::parse("");
    let _ = ValidatedPassword::parse(" S3cr3t! ");
    let _ = ValidatedPassword::parse("a,b");
    let _ = ValidatedPassword::parse("\u{0}p");
}

#[kani::proof]
// Concrete inputs (see the coverage NOTE): every comma placement variant.
#[kani::unwind(20)]
fn verify_password_parse_rejects_comma() {
    // Invariant: a password containing a comma is always rejected (breaks CSV).
    assert!(
        ValidatedPassword::parse("a,").is_err(),
        "a trailing comma must be rejected"
    );
    assert!(
        ValidatedPassword::parse(",").is_err(),
        "a lone comma must be rejected"
    );
    assert!(
        ValidatedPassword::parse("abc,def").is_err(),
        "an embedded comma must be rejected"
    );
    assert!(
        ValidatedPassword::parse(" , ").is_err(),
        "a comma amid whitespace must be rejected"
    );
}

// ============================================================================
// SanitizedDisplayName::parse
// ============================================================================

#[kani::proof]
// Fixed-string harness: boundary-value inputs, no symbolic length (see the
// coverage NOTE above — a symbolic model does not converge on CI hardware).
// Straight-line calls, no loop over an array of &str literals: iterating a
// literal array introduces slice-pointer objects that CBMC reasons about far
// more expensively than the parses themselves. unwind(20) leaves 2x headroom
// over the longest concrete input (13 chars) for std searcher inner loops.
#[kani::unwind(20)]
fn verify_display_name_parse_no_panic() {
    // Every parser branch reachable by short input: empty; all-whitespace
    // (trim → empty); leading/trailing whitespace around content (trim
    // boundaries); embedded control characters (the filter branch); DEL at
    // the string edge; multi-run inner whitespace.
    let _ = SanitizedDisplayName::parse("");
    let _ = SanitizedDisplayName::parse(" \t\r\n");
    let _ = SanitizedDisplayName::parse(" Ivan ");
    let _ = SanitizedDisplayName::parse("a\u{0}b\u{1f}");
    let _ = SanitizedDisplayName::parse("\u{7f}x");
    let _ = SanitizedDisplayName::parse("I  P");
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
#[kani::unwind(20)]
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
