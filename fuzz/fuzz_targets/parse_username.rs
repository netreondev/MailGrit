//! Fuzz target: the username sanitizer (`SanitizedUsername::parse`).
//!
//! Goal: confirm that the username sanitizer never panics on arbitrary UTF-8
//! input and always returns a `Result`. A panic here would be a DoS vector when
//! processing a malicious bulk-import CSV. Compiled under nightly via
//! libfuzzer-sys (see fuzz/Cargo.toml).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

#![no_main]

use libfuzzer_sys::fuzz_target;
use mailgrit_core_domain::SanitizedUsername;

fuzz_target!(|data: &str| {
    // Contract: the sanitizer must return a Result, not panic.
    let _ = SanitizedUsername::parse(data);
});
