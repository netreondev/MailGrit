//! Library part of app-desktop.
//!
//! Previously this exported the reqwest operation implementations (iredadmin_ops,
//! iredadmin_api) and endpoint configuration, but that path was unreachable behind
//! FortiWeb/WAF and was never invoked from the UI — the real engine works through
//! the login webview (JS fetch). Those modules and their integration tests were
//! removed; the endpoint configuration was removed as unused. The library part is
//! empty — all application code lives in the binary crate (`main.rs`).

#![forbid(unsafe_code)]
#![allow(clippy::option_if_let_else)]
// Documented exception (spec §Clippy): unwrap_used/expect_used/indexing_slicing/
// arithmetic_side_effects/panic are forbidden in production code but allowed in
// tests, where a panic = a test failure (intentionally). Applies to all test modules of the crate.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::panic
    )
)]
