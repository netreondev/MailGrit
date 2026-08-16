//! Auxiliary pure functions for the application.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::error::UrlError;
use time::OffsetDateTime;

/// Current time in RFC3339 (UTC).
pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Validates the iRedAdmin `base_url`: https + host are required.
/// iRedAdmin over HTTP would leak the session cookie — hence https is required.
///
/// # Errors
///
/// - [`UrlError::Invalid`] — not a parseable URL.
/// - [`UrlError::NotHttps`] — the scheme is not `https`.
/// - [`UrlError::NoHost`] — no host part.
///
/// The error is TYPED; the localized user-facing text is produced at the
/// display boundary via [`crate::error_i18n::url_error`].
pub fn validate_base_url(base: &str) -> Result<(), UrlError> {
    let parsed = url::Url::parse(base).map_err(|_| UrlError::Invalid)?;
    if parsed.scheme() != "https" {
        return Err(UrlError::NotHttps {
            scheme: parsed.scheme().to_string(),
        });
    }
    if parsed.host_str().is_none() {
        return Err(UrlError::NoHost);
    }
    Ok(())
}

/// Whether the PATH of `url` contains the segment `seg` (e.g. `login`,
/// `dashboard`), comparing whole path segments of the parsed URL.
///
/// Substring matching (`url.contains("/login")`) misfires on hostnames:
/// `https://login.example.com/iredadmin` contains `//login`, and
/// `https://dashboard.example.com/...` contains `/dashboard` — a deployment
/// host whose name merely starts with `login`/`dashboard` would then silently
/// wipe the user's session on every all-rows-fail batch (see
/// `ops::is_session_expired`). Whole-segment comparison on the parsed path is
/// unambiguous; unparseable input matches nothing.
#[must_use]
pub fn url_path_has_segment(url: &str, seg: &str) -> bool {
    url::Url::parse(url).is_ok_and(|u| {
        u.path_segments()
            .is_some_and(|mut segments| segments.any(|s| s == seg))
    })
}

/// Free-text variant of [`url_path_has_segment`] for reason strings that may
/// quote a URL: `/{seg}` counts only at a path boundary (end of text or the
/// next char is not a path-continuation `[A-Za-z0-9-_.]`) and only when it is
/// NOT part of a `//authority` prefix — the host `login.example.com` must not
/// count as the path `/login`. `.` counts as a continuation for the same
/// reason: a dotted token like `/login.example.com` is one segment to the URL
/// variant, so the text variant must agree (the cost: a sentence-final
/// `/login.` no longer counts).
#[must_use]
pub fn text_mentions_path_segment(text: &str, seg: &str) -> bool {
    let needle = format!("/{seg}");
    let mut search_from = 0;
    while let Some(found) = text[search_from..].find(&needle) {
        let at = search_from.saturating_add(found);
        let after = at.saturating_add(needle.len());
        let part_of_authority = at > 0 && text[..at].ends_with('/');
        let at_boundary = after == text.len()
            || !text[after..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
        if !part_of_authority && at_boundary {
            return true;
        }
        search_from = at.saturating_add(1);
    }
    false
}

/// Whether `url` may be handed to the OS shell opener: the same http/https
/// allow-list as the webview navigation handler ([`crate::login_window`]). The
/// opener interprets the string via `cmd.exe`/`xdg-open`, so a caller-supplied
/// string with a `file:`/`smb:` scheme (or shell metacharacters) must never
/// reach the shell.
#[must_use]
pub fn url_openable(url: &str) -> bool {
    url::Url::parse(url)
        .is_ok_and(|u| mailgrit_core_domain::url_policy::scheme_is_allowed(u.scheme()))
}

/// Opens a URL in the user's default SYSTEM browser.
///
/// Neither in-webview mechanic works for external links:
/// - `window.open(url, '_blank')` inside the app's `WebView2` is a SILENT NO-OP
///   (wry does not wire `NewWindowRequested` to the OS) — the donate button
///   did nothing because of exactly this;
/// - a plain `<a href>` would navigate the APP's own webview away from the UI.
///
/// So external URLs are delegated to the OS shell opener. Failures are logged,
/// never fatal.
pub fn open_in_system_browser(url: &str) {
    // The URL is interpreted by cmd.exe — refuse anything outside the http/https
    // allow-list BEFORE spawning (see url_openable).
    if !url_openable(url) {
        tracing::warn!("refusing to open a non-http(s) URL in the system browser: {url}");
        return;
    }
    // Windows: `start "" <url>` — the empty title argument is required, an URL
    // containing `&` would otherwise be consumed as the window title.
    #[cfg(target_os = "windows")]
    let result = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
    };
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();

    match result {
        Ok(_) => tracing::info!("opened {url} in the system browser"),
        Err(e) => tracing::warn!("opening {url} in the system browser failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Has a machine side effect (opens the default browser), so it is NOT part
    // of the normal suite. Manual verification:
    //   cargo nextest run -p mailgrit-app-desktop -E 'test(opens_url)' --run-ignored ignored-only
    #[test]
    #[ignore = "opens the system browser (side effect); manual verification only"]
    fn opens_url_in_system_browser() {
        open_in_system_browser("https://example.com/");
    }

    #[test]
    fn url_error_display_mentions_scheme() {
        let e = UrlError::NotHttps {
            scheme: "http".into(),
        };
        assert!(e.to_string().contains("http"));
    }

    // Whole-segment matching: hostnames must never count as path segments.
    #[test]
    fn url_path_segment_matches_whole_segments_only() {
        assert!(url_path_has_segment("https://x/iredadmin/login", "login"));
        assert!(url_path_has_segment("https://x/login", "login"));
        assert!(url_path_has_segment("https://x/login?msg=1", "login"));
        assert!(url_path_has_segment("https://x/dashboard", "dashboard"));
        // The deployment host login.example.com is NOT a /login redirect.
        assert!(!url_path_has_segment(
            "https://login.example.com/iredadmin",
            "login"
        ));
        assert!(!url_path_has_segment(
            "https://dashboard.example.com/iredadmin/users",
            "dashboard"
        ));
        assert!(!url_path_has_segment("https://x/logins", "login"));
        assert!(!url_path_has_segment("not a url", "login"));
    }

    // The same boundary rigor for free text (the reason-text fallback).
    #[test]
    fn text_path_segment_respects_boundaries() {
        assert!(text_mentions_path_segment("redirected to /login", "login"));
        assert!(text_mentions_path_segment(
            "url https://x/iredadmin/login?msg=1",
            "login"
        ));
        assert!(text_mentions_path_segment("/login/", "login"));
        // Authority component ≠ path; longer segments do not match.
        assert!(!text_mentions_path_segment(
            "https://login.example.com/iredadmin",
            "login"
        ));
        assert!(!text_mentions_path_segment(
            "host login.example.com is down",
            "login"
        ));
        // A dotted token after the slash is one segment, same as the URL
        // variant — `/login.example.com` is not the path `/login`.
        assert!(!text_mentions_path_segment(
            "blocked at /login.example.com",
            "login"
        ));
        assert!(!text_mentions_path_segment("path /login2 blocked", "login"));
        assert!(!text_mentions_path_segment("no mention at all", "login"));
    }

    #[test]
    fn system_browser_open_guard_allows_only_http_s() {
        assert!(url_openable("https://example.com/"));
        assert!(url_openable("http://example.com/"));
        assert!(!url_openable("file:///C:/Windows/System32/calc.exe"));
        assert!(!url_openable("javascript:alert(1)"));
        assert!(!url_openable("smb://nas/share"));
        assert!(!url_openable("not a url"));
    }
}
