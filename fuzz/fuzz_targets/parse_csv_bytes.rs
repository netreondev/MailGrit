//! Fuzz target: the streaming CSV parser.
//!
//! Goal: confirm through coverage-guided fuzzing that the parser never panics and
//! does not cause undefined behavior on arbitrary byte inputs.
//! Compiled under nightly via libfuzzer-sys (see fuzz/Cargo.toml).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

#![no_main]

use libfuzzer_sys::fuzz_target;
use mailgrit_core_csv::parse_csv_bytes;

fuzz_target!(|data: &[u8]| {
    // Contract: the parser must return a Result, not panic.
    // Any panic here is a critical bug (a DoS vector).
    let _ = parse_csv_bytes(data);
});
