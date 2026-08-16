//! Login-webview lifecycle: building the window, handling event-loop events,
//! data-driven navigation-based auto-auth, and dispatching operations/diagnostics.
//!
//! Raw `wry`/`tao` is used (rather than the Dioxus webview) because Dioxus
//! forces external URLs open in the system browser, whereas we need a real
//! window with a webview to log in to iRedAdmin.
//!
//! Submodules: [`crate::login_types`] (data types), [`crate::login_state`] (state).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use std::path::Path;
use std::sync::{Arc, Mutex};
use tao::event::{Event, WindowEvent};
use tao::event_loop::EventLoopWindowTarget;
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

// Re-export of the public API (the `crate::login_window::*` paths are stable after the split).
use crate::login_state::lock;
pub use crate::login_state::{LoginWindowState, login_state};
pub use crate::login_types::{CookieInfo, LoginRequest};

/// Event-loop handler: builds the login window on request and handles its
/// closure. `data_dir` is the same path as the main window (shared cookie-store).
///
/// Delegates four independent tasks to dedicated functions
/// ([`handle_open_request`], [`handle_auth_event`], [`handle_op_request`],
/// [`handle_diag_request`]); window closure is handled inline because it matches
/// the `event` itself. The split removes `#[expect(too_many_lines)]` and makes
/// each task readable/testable in isolation.
pub fn handle_event<T: 'static>(
    state: &Arc<LoginWindowState>,
    data_dir: &Path,
    event: &Event<'_, T>,
    target: &EventLoopWindowTarget<T>,
) {
    handle_open_request(state, data_dir, target);
    handle_auth_event(state);
    handle_op_request(state);
    handle_diag_request(state);
    handle_close_event(state, event);
}

/// 1. Handle the request to open the login window (once per request).
fn handle_open_request<T: 'static>(
    state: &Arc<LoginWindowState>,
    data_dir: &Path,
    target: &EventLoopWindowTarget<T>,
) {
    let request = take_optional(&state.request);
    if let Some(req) = request
        && let Err(e) = build_login_window(state, &req, data_dir, target)
    {
        tracing::error!("failed to create the login window: {e}");
    }
}

/// 2. Data-driven auto-auth: on a login-webview load event, check the login
///    predicate and, on success, invoke `on_login`.
///
///    The PREDICATE is hybrid (FortiWeb-proof):
///    - the final URL's PATH has the segment `dashboard` (the canonical
///      post-login redirect of iRedAdmin) — the PRIMARY signal, because behind
///      FortiWeb/WAF the real `webpy_session_id` cookie is invisible (only the
///      WAF cookiesession1 cookie is visible, and it is ALWAYS present);
///    - OR the `webpy_session_id` cookie is present — a fallback signal for
///      environments without a WAF.
fn handle_auth_event(state: &Arc<LoginWindowState>) {
    let Some(ev) = take_optional(&state.auth_event) else {
        return;
    };
    // Read the iRedAdmin domain cookies.
    let cookies = match crate::auth_bridge::read_cookies_for_panel(state, &ev.base_url) {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!("reading cookies for login check: {e}");
            Vec::new()
        }
    };
    // Session cookie name (from config). Poison recovery: see login_state.rs.
    let cookie_name = lock(&state.session_cookie_name).clone();
    // Fallback signal: a session cookie with a non-empty value (for environments without a WAF).
    let has_session_cookie = cookie_name
        .as_deref()
        .is_some_and(|name| cookies.iter().any(|c| c.name == name && c.value_len > 0));
    // Primary signal: the final URL's path has the `dashboard` segment (whole
    // segment — `contains("/dashboard")` would also fire on a
    // dashboard.example.com HOST, confirming login on the login page itself).
    let is_dashboard = crate::util::url_path_has_segment(&ev.final_url, "dashboard");
    let is_logged_in = is_dashboard || has_session_cookie;
    let cookie_names: Vec<&str> = cookies.iter().map(|c| c.name.as_str()).collect();
    tracing::info!(
        "login check: url={} dashboard={} cookie={}=[{}] → {}",
        ev.final_url,
        is_dashboard,
        cookie_name.as_deref().unwrap_or("(not set)"),
        cookie_names.join(", "),
        if is_logged_in {
            "LOGIN CONFIRMED"
        } else {
            "waiting for login"
        }
    );
    if is_logged_in {
        // Invoke the callback → it mutates Signal<AppState> via spawn_forever.
        // Poison recovery as everywhere (login_state.rs): a dropped login
        // confirmation is worse than acting on possibly-stale state.
        let invoked = lock(&state.on_login).as_ref().is_some_and(|cb| {
            cb();
            true
        });
        if !invoked {
            tracing::warn!("on_login callback is not registered — login not handled");
        }
    }
}

/// 2b. Execute a batch of operations via the login-webview (JS fetch).
fn handle_op_request(state: &Arc<LoginWindowState>) {
    let Some(oreq) = take_optional(&state.op_request) else {
        return;
    };
    tracing::info!("executing batch of operations via the login-webview (JS fetch)");
    if lock(&state.webview).is_none() {
        tracing::warn!("login-webview is closed — batch of operations not executed");
        let _ = oreq.tx.send(Vec::new());
        return;
    }
    let (id, rx) = crate::ipc::register(&state.pending, &state.next_ipc_id);
    let js = crate::webview_ops::build_batch_js(
        id,
        oreq.target,
        oreq.kind,
        &oreq.base_url,
        &oreq.rows,
        true,
    );
    tracing::info!("batch: evaluate_script id={id}, {} rows", oreq.rows.len());
    if let Err(e) = state.evaluate_script(&js) {
        tracing::warn!("batch: evaluate_script failed id={id}: {e}");
        let _ = oreq.tx.send(Vec::new());
        return;
    }
    let tx = oreq.tx;
    // Adaptive timeout: base + per row, capped. See the constants below.
    let rows_len = u64::try_from(oreq.rows.len()).unwrap_or(0);
    let timeout_secs = BATCH_TIMEOUT_BASE_SECS
        .saturating_add(rows_len.saturating_mul(BATCH_TIMEOUT_PER_ROW_SECS))
        .min(BATCH_TIMEOUT_MAX_SECS);
    let pending = state.pending.clone();
    crate::tokio_runtime().spawn(async move {
        let results = if let Ok(Ok(crate::ipc::IpcReply::Batch(r))) =
            tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await
        {
            r
        } else {
            // Drop the stuck pending entry on timeout (otherwise it leaks a record).
            crate::ipc::cancel(&pending, id);
            tracing::warn!("batch: timeout {timeout_secs} s id={id} ({rows_len} rows)");
            Vec::new()
        };
        let _ = tx.send(results);
    });
}

/// 2c. Form diagnostics — GET the create page + return the fields' HTML via IPC.
fn handle_diag_request(state: &Arc<LoginWindowState>) {
    let Some(dreq) = take_optional(&state.diag_request) else {
        return;
    };
    tracing::info!("form diagnostics for domain {}", dreq.domain);
    if lock(&state.webview).is_none() {
        tracing::warn!("login-webview is closed — diagnostics not executed");
        let _ = dreq.tx.send(r#"{"error":"webview closed"}"#.to_string());
        return;
    }
    let (id, rx) = crate::ipc::register(&state.pending, &state.next_ipc_id);
    let js = crate::webview_ops::build_diag_js(id, &dreq.domain);
    tracing::info!("diag: evaluate_script id={id}");
    if let Err(e) = state.evaluate_script(&js) {
        tracing::warn!("diag: evaluate_script failed id={id}: {e}");
        let _ = dreq.tx.send(r#"{"error":"eval failed"}"#.to_string());
        return;
    }
    let tx = dreq.tx;
    crate::tokio_runtime().spawn(async move {
        let result = if let Ok(Ok(crate::ipc::IpcReply::Diag(json))) =
            tokio::time::timeout(std::time::Duration::from_secs(DIAG_TIMEOUT_SECS), rx).await
        {
            json
        } else {
            tracing::warn!("diag: timeout/error {DIAG_TIMEOUT_SECS} s id={id}");
            r#"{"error":"timeout"}"#.to_string()
        };
        let _ = tx.send(result);
    });
}

/// 3. Handle login-window closure: releases the webview/window/ctx and clears
///    the pending IPC map (otherwise entries leak — the webview is dead, no
///    replies will arrive).
fn handle_close_event<T: 'static>(state: &Arc<LoginWindowState>, event: &Event<'_, T>) {
    let Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        window_id,
        ..
    } = event
    else {
        return;
    };
    let mine = lock(&state.window)
        .as_ref()
        .is_some_and(|w| w.id() == *window_id);
    if !mine {
        return;
    }
    tracing::info!("login window closed by the user");
    // Poison recovery everywhere (login_state.rs): cleanup must run even after
    // a panic elsewhere — otherwise the webview/window leak while appearing closed.
    *lock(&state.webview) = None;
    *lock(&state.window) = None;
    *lock(&state.web_ctx) = None;
    let leaked = {
        let mut pending = lock(&state.pending);
        let leaked = pending.len();
        pending.clear();
        leaked
    };
    if leaked > 0 {
        tracing::warn!("webview close: cleared {leaked} stuck IPC requests");
    }
}

/// Extracts the `Option` from a `Mutex<Option<T>>`, recovering from a poisoned
/// state (see [`lock`]). Removes 4 copies of the same match.
fn take_optional<T>(slot: &Mutex<Option<T>>) -> Option<T> {
    lock(slot).take()
}

/// Base component of the adaptive operation-batch timeout (seconds).
const BATCH_TIMEOUT_BASE_SECS: u64 = 60;
/// Per-row component of the operation-batch timeout (seconds).
const BATCH_TIMEOUT_PER_ROW_SECS: u64 = 10;
/// Cap of the adaptive operation-batch timeout (30 minutes).
const BATCH_TIMEOUT_MAX_SECS: u64 = 1800;
/// Form diagnostics timeout (seconds).
const DIAG_TIMEOUT_SECS: u64 = 15;

/// Builds the login window + webview on the Dioxus event-loop.
fn build_login_window<T: 'static>(
    state: &Arc<LoginWindowState>,
    req: &LoginRequest,
    data_dir: &Path,
    target: &EventLoopWindowTarget<T>,
) -> Result<(), crate::error::AppError> {
    tracing::info!("building the raw wry login window on the Dioxus event-loop");

    // Test mode (only compiled with the `e2e` cargo feature): MAILGRIT_LOGIN_URL
    // overrides the URL (diagnostics without iRedAdmin). The URL goes through
    // the same validation as the main base_url (https + host): iRedAdmin over
    // HTTP would leak the session cookie (see util::validate_base_url), so the
    // env override must not bypass this requirement.
    #[cfg(feature = "e2e")]
    let url = std::env::var("MAILGRIT_LOGIN_URL").map_or_else(
        |_| req.base_url.clone(),
        |env_url| {
            if let Err(e) = crate::util::validate_base_url(&env_url) {
                tracing::warn!(
                    "MAILGRIT_LOGIN_URL rejected ({e}), using base_url: {}",
                    req.base_url
                );
                req.base_url.clone()
            } else {
                tracing::warn!("MAILGRIT_LOGIN_URL overrides the URL: {env_url}");
                env_url
            }
        },
    );
    #[cfg(not(feature = "e2e"))]
    let url = req.base_url.clone();

    // 1. A tao window on the same event-loop. The title is localized (rust_i18n).
    let window = Arc::new(
        WindowBuilder::new()
            .with_title(tr!("window.login_title"))
            .with_window_icon(crate::window_icon::window_icon())
            .with_inner_size(tao::dpi::LogicalSize::new(1100.0, 780.0))
            .build(target)
            .map_err(|e| crate::error::AppError::Other(format!("window creation: {e}")))?,
    );

    // 2. A WebContext with the same data_dir → a shared WebView2 cookie-store with the main window.
    let mut web_ctx = wry::WebContext::new(Some(data_dir.to_path_buf()));

    // 3. The raw wry WebView. navigation_handler allows only http/https.
    //    Login detection is via page_load_handler (Finished): it fires AFTER
    //    loading and redirects, yielding the final URL for the login predicate.
    let pending_map = state.pending.clone();
    let nav_base_url = req.base_url.clone();
    let page_load_state = Arc::clone(state);
    let webview = WebViewBuilder::new_with_web_context(&mut web_ctx)
        .with_url(&url)
        .with_navigation_handler(|nav_url: String| {
            // Allow only http/https (block javascript:, file:, data:, etc.).
            // The allow-list itself lives in core-domain (url_policy) — the
            // same function the fuzz target imports.
            let allowed = url::Url::parse(&nav_url)
                .is_ok_and(|u| mailgrit_core_domain::url_policy::scheme_is_allowed(u.scheme()));
            if !allowed {
                tracing::warn!("login webview navigation blocked: {nav_url}");
            }
            allowed
        })
        .with_on_page_load_handler(move |event, loaded_url: String| {
            // Finished fires after loading and redirects. The event only wakes
            // handle_event via report_page_load + request_redraw (page_load does
            // not itself produce a tao event — without request_redraw there was a delay).
            if matches!(event, wry::PageLoadEvent::Finished) {
                tracing::debug!("login webview page loaded: {loaded_url}");
                page_load_state.report_page_load(nav_base_url.clone(), loaded_url);
                if let Some(w) = lock(&page_load_state.window).as_ref() {
                    w.request_redraw();
                }
            }
        })
        .with_ipc_handler(move |req: http::Request<String>| {
            // Deliver the reply to the correct oneshot::Sender by id.
            crate::ipc::dispatch(&pending_map, req.body());
        })
        .build(&window)
        .map_err(|e| crate::error::AppError::Other(format!("webview creation: {e}")))?;

    tracing::info!("login webview created, navigating to {url}");

    // 4. Store the webview/window/context (order matters for lifetimes).
    // Poison recovery (login_state.rs): without it a poisoned lock would
    // silently DROP the freshly built window — the user's click would do
    // nothing, with no log line.
    *lock(&state.webview) = Some(webview);
    *lock(&state.window) = Some(window);
    *lock(&state.web_ctx) = Some(web_ctx);

    Ok(())
}
