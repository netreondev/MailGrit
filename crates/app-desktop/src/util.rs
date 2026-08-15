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

/// Validates the iRedAdmin base_url: https + host are required.
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

/// Opens a URL in the user's default SYSTEM browser.
///
/// Neither in-webview mechanic works for external links:
/// - `window.open(url, '_blank')` inside the app's WebView2 is a SILENT NO-OP
///   (wry does not wire `NewWindowRequested` to the OS) — the donate button
///   did nothing because of exactly this;
/// - a plain `<a href>` would navigate the APP's own webview away from the UI.
///
/// So external URLs are delegated to the OS shell opener. Failures are logged,
/// never fatal.
pub fn open_in_system_browser(url: &str) {
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
}
