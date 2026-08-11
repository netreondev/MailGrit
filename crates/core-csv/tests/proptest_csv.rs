//! Property-based tests for the CSV parser via `proptest` (spec §testing).
//!
//! Goal: ensure the parser NEVER panics on arbitrary input and correctly
//! classifies valid/invalid rows. proptest generates thousands of random but
//! structured test cases and shrinks a found bug down to a minimal reproducible
//! example (shrinking).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

//
// Not compiled under Miri: proptest runs thousands of iterations and Miri
// interprets each byte of std code per iteration — a single proptest case here
// was observed running >90 min under Miri without finishing. Property-based
// coverage under Miri adds no UB signal beyond the unit tests (mapping_tests,
// parser::tests) which Miri DOES run. UB hunting is covered by those; proptest
// stays for native-test coverage only.
#![cfg(not(miri))]

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
        let parsed = parse_csv_bytes(line.as_bytes());
        prop_assert!(parsed.is_ok(), "a valid row must parse");
        let Ok(parsed) = parsed else {
            return Err(TestCaseError::fail("unreachable: is_ok asserted"));
        };
        prop_assert_eq!(parsed.rows.len(), 1, "exactly 1 valid row expected");
        prop_assert!(parsed.failed.is_empty(), "there should be no failed rows");
        prop_assert_eq!(parsed.rows.first().map(|r| r.username.as_str()), Some(user.as_str()));
    }

    #[test]
    fn email_in_domain_always_rejected(
        user in "[a-z]{1,10}",
        host in "[a-z]{1,10}"
    ) {
        let line = format!("{user}@{host},{user},pass,Name,100\n");
        let parsed = parse_csv_bytes(line.as_bytes());
        prop_assert!(parsed.is_ok(), "parsing does not panic");
        let Ok(parsed) = parsed else {
            return Err(TestCaseError::fail("unreachable: is_ok asserted"));
        };
        prop_assert_eq!(parsed.rows.len(), 0, "a row with an email domain must not pass");
        prop_assert_eq!(parsed.failed.len(), 1, "the row must end up in failed");
    }

    #[test]
    fn header_always_skipped(
        data_row in "[a-z]{1,10},[a-z]{1,10},[A-Za-z0-9]{1,10},[A-Za-z]{1,10},100"
    ) {
        let input = format!("domain,username,password,display_name,quota_mb\n{data_row}\n");
        let parsed = parse_csv_bytes(input.as_bytes());
        prop_assert!(parsed.is_ok(), "does not panic");
        let Ok(parsed) = parsed else {
            return Err(TestCaseError::fail("unreachable: is_ok asserted"));
        };
        prop_assert_eq!(parsed.rows.len(), 1, "header skipped, one data row");
    }
}
