//! Fuzz target: the domain validator (`ValidatedDomain::parse`).
//!
//! Goal: confirm that the domain parser does not panic on arbitrary UTF-8 strings.
//! Compiled under nightly via libfuzzer-sys.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

#![no_main]

use libfuzzer_sys::fuzz_target;
use mailgrit_core_domain::ValidatedDomain;

fuzz_target!(|data: &str| {
    // Contract: the domain validator must return a Result, not panic.
    let _ = ValidatedDomain::parse(data);
});
