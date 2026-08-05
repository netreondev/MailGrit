//! Property-based tests for the CSV parser via `proptest` (spec §testing).
//!
//! Goal: ensure the parser NEVER panics on arbitrary input and correctly
//! classifies valid/invalid rows. proptest generates thousands of random but
//! structured test cases and shrinks a found bug down to a minimal reproducible
//! example (shrinking).

// Documented exception (spec): unwrap/expect/panic are acceptable in tests —
// a panic here is a meaningful test failure.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic
)]

use mailgrit_core_csv::parse_csv_bytes;
// prelude::* imports the proptest! macro and generation strategies.
use proptest::prelude::*;

// Arbitrary input must not cause a panic (the parser contract).
proptest! {
    #[test]
    fn never_panics_on_arbitrary_input(input in ".{0,2000}") {
        let _ = parse_csv_bytes(input.as_bytes());
    }

    #[test]
    fn valid_row_always_parses(
        // Strategy matches the real SanitizedUsername validation rules:
        // a letter/digit at the start and end (no '.' or '-' at the edges).
        user in "[a-z0-9]([a-z0-9._-]{0,28}[a-z0-9])?",
        domain in "[a-z][a-z0-9-]{0,20}\\.[a-z]{2,10}",
        pass in "[A-Za-z0-9!@#$%^&*]{8,30}",
        display in "[A-Za-z ]{1,40}",
        quota in "(|[1-9][0-9]{0,5})"
    ) {
        let line = format!("{domain},{user},{pass},{display},{quota}\n");
        let parsed = parse_csv_bytes(line.as_bytes()).expect("a valid row must parse");
        prop_assert_eq!(parsed.rows.len(), 1, "exactly 1 valid row expected");
        prop_assert!(parsed.failed.is_empty(), "there should be no failed rows");
        prop_assert_eq!(parsed.rows[0].username.as_str(), user);
    }

    #[test]
    fn email_in_domain_always_rejected(
        user in "[a-z]{1,10}",
        host in "[a-z]{1,10}"
    ) {
        let line = format!("{user}@{host},{user},pass,Name,100\n");
        let parsed = parse_csv_bytes(line.as_bytes()).expect("parsing does not panic");
        prop_assert_eq!(parsed.rows.len(), 0, "a row with an email domain must not pass");
        prop_assert_eq!(parsed.failed.len(), 1, "the row must end up in failed");
    }

    #[test]
    fn header_always_skipped(
        data_row in "[a-z]{1,10},[a-z]{1,10},[A-Za-z0-9]{1,10},[A-Za-z]{1,10},100"
    ) {
        let input = format!("domain,username,password,display_name,quota_mb\n{data_row}\n");
        let parsed = parse_csv_bytes(input.as_bytes()).expect("does not panic");
        prop_assert_eq!(parsed.rows.len(), 1, "header skipped, one data row");
    }
}
