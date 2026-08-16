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

    // --- validate_base_url (typed https+host requirement) ---

    #[test]
    fn validate_base_url_accepts_https_with_host() {
        assert_eq!(
            validate_base_url("https://mail.example.com/iredadmin"),
            Ok(())
        );
    }

    #[test]
    fn validate_base_url_rejects_http_with_typed_error() -> Result<(), Box<dyn std::error::Error>> {
        let Err(err) = validate_base_url("http://mail.example.com/") else {
            return Err("plain http must be rejected as NotHttps".into());
        };
        assert!(
            matches!(err, UrlError::NotHttps { ref scheme } if scheme == "http"),
            "plain http must be rejected as NotHttps: {err}"
        );
        Ok(())
    }

    #[test]
    fn validate_base_url_rejects_unparseable_and_hostless() {
        assert!(
            matches!(validate_base_url("not a url"), Err(UrlError::Invalid)),
            "garbage must be rejected as Invalid"
        );
        assert!(
            validate_base_url("https://").is_err(),
            "an https URL without a host must be rejected"
        );
    }

    // --- now_rfc3339 (audit timestamps must be real RFC3339) ---

    #[test]
    fn now_rfc3339_is_a_parseable_rfc3339_timestamp() -> Result<(), Box<dyn std::error::Error>> {
        let now = now_rfc3339();
        assert!(!now.is_empty(), "the timestamp must not be empty");
        assert_ne!(now, "xyzzy");
        assert_ne!(now, "unknown", "the format fallback must not be hit");
        time::OffsetDateTime::parse(&now, &time::format_description::well_known::Rfc3339)?;
        Ok(())
    }

    // --- text_mentions_path_segment: authority vs path (extended) ---
    //
    // The mutants `at > 0` -> `at == 0`/`at < 0` in part_of_authority survive
    // dotted-host tests because '.' is a path-continuation char. They are
    // killed by a SINGLE-LABEL host or a bare "//login": there the needle is
    // preceded by '/' and followed by a boundary, so only the authority check
    // keeps them from counting as the path.

    #[test]
    fn text_path_segment_single_label_host_is_authority_not_path() {
        // Internal deployments use single-label hosts; "https://login" is the
        // AUTHORITY "login", not the path "/login".
        assert!(!text_mentions_path_segment("https://login", "login"));
        // A bare scheme-relative "//login" likewise.
        assert!(!text_mentions_path_segment("blocked at //login", "login"));
        // …while a real path on such a host still counts.
        assert!(text_mentions_path_segment(
            "redirect to https://x/iredadmin/login",
            "login"
        ));
    }

    #[test]
    fn text_path_segment_dash_and_underscore_continue_the_segment() {
        // "/login-x" and "/login_x" are single segments (same as "/login.example.com"):
        // '-' and '_' are path-continuation characters and must not end a match.
        assert!(!text_mentions_path_segment(
            "path /login-x blocked",
            "login"
        ));
        assert!(!text_mentions_path_segment(
            "path /login_x blocked",
            "login"
        ));
        // A non-continuation character right after the needle IS a boundary.
        assert!(text_mentions_path_segment("go to /login?next=/x", "login"));
        assert!(text_mentions_path_segment(
            "see /login, then retry",
            "login"
        ));
    }

    // --- open_in_system_browser: the shell-bridge guard ---
    //
    // The refusal (never hand a non-http(s) URL to cmd.exe) is observable via
    // the emitted warning; the happy path opens a real browser and is covered
    // by the ignored manual test below.

    /// A `MakeWriter` that captures formatted log lines into shared storage.
    #[derive(Clone, Default)]
    struct SharedBuf(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
    impl std::io::Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    struct CaptureMaker(SharedBuf);
    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CaptureMaker {
        type Writer = SharedBuf;
        fn make_writer(&'a self) -> Self::Writer {
            self.0.clone()
        }
    }

    #[test]
    fn system_browser_refuses_non_http_urls_before_spawning() {
        let buf = SharedBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(CaptureMaker(buf.clone()))
            .with_max_level(tracing::Level::INFO)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            open_in_system_browser("file:///C:/Windows/System32/calc.exe");
            open_in_system_browser("javascript:alert(1)");
        });
        let captured = buf
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let out = String::from_utf8_lossy(&captured).into_owned();
        assert_eq!(
            out.matches("refusing to open a non-http(s) URL").count(),
            2,
            "both non-http(s) URLs must be refused (and logged): {out}"
        );
    }
}
