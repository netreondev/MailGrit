//! Global (main-thread) state for the login window.
//!
//! The `WebView` holds an HWND (!Send), so this state cannot live in the Dioxus
//! context (which requires Send). `thread_local!` stores !Send values and is
//! reachable from UI components and the event handler (both on the main Dioxus
//! thread).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::ipc::PendingMap;
use crate::login_types::{BatchOpResult, DiagRequest, LoginRequest, OpRequest};
use std::sync::{Arc, Mutex};

thread_local! {
    /// Global (main-thread) state for the login window.
    static LOGIN_STATE: std::cell::OnceCell<Arc<LoginWindowState>> = const { std::cell::OnceCell::new() };
}

/// Returns the global login-window state (creating it on the first call).
#[must_use]
pub fn login_state() -> Arc<LoginWindowState> {
    LOGIN_STATE.with(|cell| cell.get_or_init(LoginWindowState::new).clone())
}

/// Locks `m` with poison recovery (`into_inner`) — the concrete form of the
/// mutex policy documented on [`LoginWindowState`]: a dropped request is far
/// worse for the user than acting on possibly-stale state.
pub fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Shared login-window state — reachable from both the UI and the event-loop
/// handler. Internally `Arc` + `Mutex` (the handler is `FnMut + 'static`).
pub struct LoginWindowState {
    /// Request from the "Open login form" button (cleared after the window is built).
    pub request: Mutex<Option<LoginRequest>>,
    /// Request for a batch of operations (executed in `handle_event` via JS fetch).
    pub op_request: Mutex<Option<OpRequest>>,
    /// Request for form diagnostics (GET + return HTML for field analysis).
    pub diag_request: Mutex<Option<DiagRequest>>,
    /// Pending login-webview navigation event (data-driven login trigger).
    pub auth_event: Mutex<Option<crate::login_types::AuthEvent>>,
    /// Successful-auth callback (registered from `app()`). `handle_event` invokes
    /// it when login is confirmed. `!Send + Sync` is justified: the state is
    /// `thread_local` (main Dioxus thread), and the webview/window is already
    /// `!Send` (HWND).
    pub on_login: Mutex<Option<Box<dyn Fn()>>>,
    /// The session cookie name (from config.toml). `handle_event` checks for its
    /// presence for data-driven login confirmation. Populated from `app()`.
    pub session_cookie_name: Mutex<Option<String>>,
    /// Table of pending IPC responses: id → `oneshot::Sender`.
    pub pending: PendingMap,
    /// Correlation-id counter for IPC.
    pub next_ipc_id: Mutex<u64>,
    /// Raw login webview (kept alive, otherwise it closes).
    pub webview: Mutex<Option<wry::WebView>>,
    /// Raw tao window (must outlive the webview).
    pub window: Mutex<Option<Arc<tao::window::Window>>>,
    /// The webview's `WebContext` (must outlive the webview — wry holds &mut).
    pub web_ctx: Mutex<Option<wry::WebContext>>,
}

impl Default for LoginWindowState {
    fn default() -> Self {
        Self {
            request: Mutex::new(None),
            op_request: Mutex::new(None),
            diag_request: Mutex::new(None),
            auth_event: Mutex::new(None),
            on_login: Mutex::new(None),
            session_cookie_name: Mutex::new(None),
            pending: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            next_ipc_id: Mutex::new(0),
            webview: Mutex::new(None),
            window: Mutex::new(None),
            web_ctx: Mutex::new(None),
        }
    }
}

impl LoginWindowState {
    /// Creates the shared state.
    #[must_use]
    pub fn new() -> Arc<Self> {
        // Built via `Arc::from(Box<T>)` rather than `Arc::new(T)`: this is the
        // idiomatic way to construct an `Arc<T>` for a `T: !Send` value without
        // tripping the `arc_with_non_send_sync` pedantic lint. The state is
        // main-thread only (stored in a `thread_local!`), so the lack of
        // `Send + Sync` is by design — the webview/window it holds are `!Send`.
        Arc::from(Box::new(Self::default()))
    }

    // Mutex policy (uniform across app-desktop): a poisoned lock means a panic
    // happened while it was held — the guarded data is still structurally valid.
    // We recover with `unwrap_or_else(PoisonError::into_inner)` instead of
    // silently dropping the request: a dropped login/operation request is far
    // worse for the user than acting on possibly-stale state.

    /// Requests opening the login window (called from a UI button).
    pub fn request_open(&self, base_url: String) {
        tracing::info!("request to open the login window: {base_url}");
        *lock(&self.request) = Some(LoginRequest { base_url });
    }

    /// Registers the successful-auth callback (called from `app()`).
    pub fn set_on_login(&self, cb: Box<dyn Fn()>) {
        *lock(&self.on_login) = Some(cb);
    }

    /// Stores the session cookie name (from config.toml) for the login predicate.
    pub fn set_session_cookie_name(&self, name: String) {
        *lock(&self.session_cookie_name) = Some(name);
    }

    /// Records the login-webview load event (called from `page_load_handler`).
    /// `final_url` is the final URL after redirects (for the `/dashboard` predicate).
    pub fn report_page_load(&self, base_url: String, final_url: String) {
        *lock(&self.auth_event) = Some(crate::login_types::AuthEvent {
            base_url,
            final_url,
        });
    }

    /// Requests a batch of operations via the login-webview (JS fetch).
    /// Returns `None` if a request is already pending (an operation is in flight).
    pub fn request_op(
        &self,
        target: mailgrit_core_domain::OperationTarget,
        kind: mailgrit_core_domain::BulkOperationKind,
        base_url: String,
        rows: Vec<mailgrit_core_domain::SanitizedUserRow>,
    ) -> Option<tokio::sync::oneshot::Receiver<BatchOpResult>> {
        let (tx, rx) = tokio::sync::oneshot::channel::<BatchOpResult>();
        tracing::info!(
            "batch operation request: {} rows, {}",
            rows.len(),
            crate::op_label::operation_label(target, kind),
        );
        {
            let mut req = lock(&self.op_request);
            if req.is_some() {
                tracing::warn!("batch operation request rejected: a request is already pending");
                return None;
            }
            *req = Some(OpRequest {
                target,
                kind,
                base_url,
                rows,
                tx,
            });
        }
        Some(rx)
    }

    /// Requests diagnostics for the create form (GET + returns the form fields' HTML).
    /// Returns `None` if a request is already pending.
    pub fn request_diag(&self, domain: String) -> Option<tokio::sync::oneshot::Receiver<String>> {
        let (tx, rx) = tokio::sync::oneshot::channel::<String>();
        tracing::info!("request for form diagnostics for domain {domain}");
        {
            let mut req = lock(&self.diag_request);
            if req.is_some() {
                tracing::warn!("diagnostics request rejected: a request is already pending");
                return None;
            }
            *req = Some(DiagRequest { domain, tx });
        }
        Some(rx)
    }

    /// Hides the login window (after a successful login).
    pub fn hide(&self) {
        if let Some(w) = lock(&self.window).as_ref() {
            w.set_visible(false);
            tracing::info!("login window hidden");
        }
    }

    /// Reads cookies while holding the webview mutex. Returns `None` if the window is not open.
    pub fn with_webview_cookies<R>(&self, f: impl FnOnce(&wry::WebView) -> R) -> Option<R> {
        lock(&self.webview).as_ref().map(f)
    }

    /// Evaluates `js` in the login webview; `Err` ("login webview is gone")
    /// when the webview is closed — the single mapping of "no webview" to a
    /// `wry` error for the batch/diag dispatch paths.
    pub fn evaluate_script(&self, js: &str) -> wry::Result<()> {
        self.with_webview_cookies(|wv| wv.evaluate_script(js))
            .unwrap_or_else(|| {
                Err(wry::Error::Io(std::io::Error::other(
                    "login webview is gone",
                )))
            })
    }
}
