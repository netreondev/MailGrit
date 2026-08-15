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
