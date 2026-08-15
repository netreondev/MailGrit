//! URL scheme policy shared by the app and the fuzz targets.
//!
//! Single source of truth for the scheme allow-list used by the webview
//! navigation handler (and link opening): the embedded webview is a privileged
//! context, so only `http`/`https` may ever be navigated to — `javascript:`,
//! `file:`, `data:` and every other scheme are blocked. The fuzz target
//! `fuzz/fuzz_targets/url_parse.rs` imports THIS function (previously it kept a
//! hand-maintained copy with a "kept in sync manually" comment).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/// Whether a URL scheme may be navigated to / opened (http/https only).
#[must_use]
pub fn scheme_is_allowed(scheme: &str) -> bool {
    matches!(scheme, "http" | "https")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_and_https_allowed() {
        assert!(scheme_is_allowed("http"));
        assert!(scheme_is_allowed("https"));
    }

    #[test]
    fn dangerous_schemes_blocked() {
        for scheme in [
            "javascript",
            "file",
            "data",
            "vbscript",
            "about",
            "blob",
            "ftp",
            "ws",
            "wss",
            "",
        ] {
            assert!(
                !scheme_is_allowed(scheme),
                "{scheme:?} must never be allowed"
            );
        }
    }
}
