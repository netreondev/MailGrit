//! Fuzz target: the quota parser (`ValidatedQuota::parse`).
//!
//! Goal: confirm that the quota parser never panics on arbitrary UTF-8 input
//! (including non-numeric, huge, negative-looking, and overflow-looking
//! strings) and always returns a `Result`. The parser must also uphold its
//! invariant: a successful parse always yields a quota within the documented
//! range. Compiled under nightly via libfuzzer-sys (see fuzz/Cargo.toml).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

#![no_main]

use libfuzzer_sys::fuzz_target;
use mailgrit_core_domain::ValidatedQuota;

fuzz_target!(|data: &str| {
    // Contract 1: the parser must return a Result, not panic.
    if let Ok(q) = ValidatedQuota::parse(data) {
        // Contract 2 (invariant): a successful parse always yields a quota in
        // the documented range [1, MAX_QUOTA_MB] — BOTH bounds. assert (not
        // debug_assert) so release-profile fuzz runs check it too; the old
        // version asserted only the lower bound while documenting a range.
        assert!(
            q.mb() >= 1,
            "quota parser must never return a value below the minimum"
        );
        assert!(
            u64::from(q.mb()) <= mailgrit_core_domain::MAX_QUOTA_MB,
            "quota parser must never return a value above MAX_QUOTA_MB"
        );
    }
});
