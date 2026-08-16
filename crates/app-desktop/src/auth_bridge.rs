//! Bridge for the iRedAdmin login window.
//!
//! Previously this held cookie extraction plus reqwest-client construction. That
//! turned out to be unreliable: behind FortiWeb/WAF the backend session is held
//! by the proxy, and replaying the cookie in reqwest does not authenticate
//! against the backend. Also cookie names vary (web.py, Django, `FortiWeb`) —
//! guessing the name is brittle.
//!
//! Now login is detected data-driven — via the login-webview navigation event
//! (`navigation_handler` -> `LoginWindowState::report_navigation` -> the
//! event-loop handler in `login_window::handle_event` checks the login predicate
//! from the URL). Operations also go through the webview
//! (`webview_ops::build_batch_js`/`build_diag_js`).
//!
//! This module is a thin wrapper for requesting the login window and reading
//! cookies (for the panel).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::login_window::{CookieInfo, LoginWindowState};

/// Requests opening the raw iRedAdmin login window.
///
/// It does not build the webview itself (that requires the event-loop thread) —
/// it only queues a request in `LoginWindowState`; the event-loop handler builds
/// the window.
pub fn request_login_window(state: &LoginWindowState, base_url: &str) {
    state.request_open(base_url.to_string());
}

/// Reads all cookies of the domain (by origin) for the diagnostics panel.
///
/// Call ONLY at the top level of the event loop (`handle_event`), NOT from an
/// onclick or a JS callback — otherwise `cookies_for_url` deadlocks the message
/// loop (`wait_with_pump`).
///
/// # Errors
///
/// Returns `Err` on a cookie read error or if the login window is not open.
pub fn read_cookies_for_panel(
    state: &LoginWindowState,
    base_url: &str,
) -> Result<Vec<CookieInfo>, crate::error::AppError> {
    let origin = url_origin(base_url)
        .ok_or_else(|| crate::error::AppError::InvalidBaseUrl(base_url.to_string()))?;
    let cookies = state
        .with_webview_cookies(|wv| wv.cookies_for_url(&origin))
        .ok_or(crate::error::AppError::LoginWindowClosed)?
        .map_err(|e| crate::error::AppError::WebView(e.to_string()))?;
    Ok(cookies
        .iter()
        .map(|c| CookieInfo {
            name: c.name().to_string(),
            domain: c.domain().map(str::to_string),
            path: c.path().map(str::to_string),
            http_only: c.http_only().unwrap_or(false),
            value_len: c.value().len(),
        })
        .collect())
}

/// Returns the URL origin (`scheme://host[:port]`) — without path/query/fragment.
fn url_origin(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    let scheme = parsed.scheme();
    Some(parsed.port().map_or_else(
        || format!("{scheme}://{host}"),
        |p| format!("{scheme}://{host}:{p}"),
    ))
}
