//! Fuzz target: the URL scheme allow-list used by the webview navigation handler.
//!
//! Mirrors the security check at `crates/app-desktop/src/login_window.rs`
//! (navigation handler) and `crates/app-desktop/src/util.rs::validate_base_url`.
//! The webview must only ever navigate to `http`/`https` URLs: `javascript:`,
//! `file:`, `data:`, and other schemes are blocked to prevent privileged-context
//! abuse inside the embedded webview.
//!
//! This target fuzzes the raw `url::Url::parse` + scheme match in isolation,
//! without pulling the entire GUI stack (dioxus/wry) into the fuzz crate. A
//! panic from `url::Url::parse` on pathological input would itself be a bug
//! worth knowing about; we assert it does not happen.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

#![no_main]

use libfuzzer_sys::fuzz_target;
use url::Url;

/// The PRODUCTION allow-list, imported from core-domain (url_policy) — no
/// hand-maintained mirror. If the app ever allows a new scheme, this target's
/// assertions automatically follow.
fn scheme_is_allowed(url: &str) -> bool {
    Url::parse(url).is_ok_and(|u| mailgrit_core_domain::url_policy::scheme_is_allowed(u.scheme()))
}

fuzz_target!(|data: &str| {
    // Contract 1: URL parsing must never panic on arbitrary input.
    // Contract 2: the scheme allow-list is a pure boolean function over the URL.
    let _ = scheme_is_allowed(data);

    // Spot-check the security-critical invariant: a `javascript:` URL is NEVER
    // allowed through, regardless of trailing payload. If this assertion ever
    // fires, the navigation allow-list has a bypass.
    if let Some(rest) = data
        .to_ascii_lowercase()
        .strip_prefix("javascript:")
    {
        // Pathological forms like "java\tscript:" or leading whitespace are how
        // browsers historically got tricked; Url::parse normalizes away some of
        // these. If url accepted it as javascript: scheme, it MUST be rejected
        // by the allow-list.
        if Url::parse(&format!("javascript:{rest}"))
            .is_ok_and(|u| u.scheme() == "javascript")
        {
            // assert (not debug_assert): the invariant must hold in release-
            // profile fuzzing runs too, where debug_assert is a no-op.
            assert!(
                !scheme_is_allowed(&format!("javascript:{rest}")),
                "javascript: URL must never pass the http/https allow-list"
            );
        }
    }
});
