//! Library part of app-desktop.
//!
//! Previously this exported the reqwest operation implementations (`iredadmin_ops`,
//! `iredadmin_api`) and endpoint configuration, but that path was unreachable behind
//! FortiWeb/WAF and was never invoked from the UI — the real engine works through
//! the login webview (JS fetch). Those modules and their integration tests were
//! removed; the endpoint configuration was removed as unused. The library part is
//! empty — all application code lives in the binary crate (`main.rs`).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

#![forbid(unsafe_code)]
// Lint policy (missing_docs/dead_code/unused/rust_2018_idioms deny, plus the
// clippy groups) is set centrally in the root Cargo.toml. No crate-level or
// test-only suppressions.
