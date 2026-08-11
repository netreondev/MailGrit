//! Auxiliary pure functions for the application.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

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
/// Returns `Err(message)` with a clear reason (failed to parse / not https / no host).
pub fn validate_base_url(base: &str) -> Result<(), String> {
    let parsed = url::Url::parse(base).map_err(|_| t!("url.invalid").to_string())?;
    if parsed.scheme() != "https" {
        return Err(t!("url.not_https", scheme = parsed.scheme()).to_string());
    }
    if parsed.host_str().is_none() {
        return Err(t!("url.no_host").to_string());
    }
    Ok(())
}
