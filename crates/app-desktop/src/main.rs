//! `app-desktop` — MailGrit entry point (Dioxus 0.7 desktop).
//!
//! Integration point that wires the core-* crates to the UI. Login is detected
//! in a data-driven way by observing the webview navigating to `/dashboard`;
//! bulk operations run as JS `fetch()` inside the same webview (FortiWeb/WAF
//! does not authenticate the backend when reqwest replays the cookie).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

#![forbid(unsafe_code)]
// Lint policy (missing_docs/dead_code/unused/rust_2018_idioms deny, plus the
// clippy groups) is set centrally in the root Cargo.toml. No crate-level or
// test-only suppressions.

// i18n: translation catalogs are embedded into the binary (locales/app.<lang>.yml),
// with English as the fallback. The `t!` macro is available across the whole crate
// via `#[macro_use]`.
#[macro_use]
extern crate rust_i18n;
rust_i18n::i18n!("locales", fallback = "en");

/// Thin wrapper over [`rust_i18n::t!`] returning a `String` (not `Cow<str>`).
/// Needed for RSX (`IntoDynNode` is implemented for `String`/`&str`, but not for `Cow<str>`).
/// `#[macro_export]` makes it available in all submodules of the crate.
#[macro_use]
mod i18n_macros {
    /// Translate a message key to the active locale, returning an owned `String`.
    ///
    /// Delegates to [`rust_i18n::t!`] and converts the `Cow<str>` into `String`
    /// so the result is usable directly in RSX (Dioxus implements `IntoDynNode`
    /// for `String`/`&str`, but not for `Cow<str>`).
    #[macro_export]
    macro_rules! tr {
        ($($arg:tt)*) => {
            $crate::i18n::__cow_to_string(rust_i18n::t!($($arg)*))
        };
    }
}

mod audit_ui;
mod auth_bridge;
mod batch;
mod brand;
mod components;
mod csv_card;
mod csv_summary;
#[cfg(feature = "e2e")]
mod e2e_state;
mod editable_table_view;
mod error;
mod error_i18n;
mod fs_util;
mod i18n;
mod infra;
mod ipc;
mod language;
mod logging;
mod login_state;
mod login_types;
mod login_window;
mod master_password_modal;
mod nav;
mod op_label;
mod operations_view;
mod ops;
mod ops_export;
mod password_controls;
mod screens;
mod settings;
mod state;
mod theme;
mod util;
mod views;
mod webview_js;
mod webview_js_extra;
mod webview_markers;
mod webview_ops;
mod webview_parse;
mod window_icon;

// Re-export of infrastructure and state (paths like `crate::app_data_dir` are stable).
pub(crate) use infra::{app_data_dir, tokio_runtime};
pub use state::{AppState, AuditEntryView, AuthStatus, OpStatus, Screen};

use dioxus::prelude::*;
use screens::{dashboard_screen, login_screen};
use std::sync::Arc;

use components::titlebar::TitleBar;

/// Application styling: fonts -> design tokens -> base -> components -> screens,
/// inlined as a single `<style>` in head via `custom_head` (no JS toolchain).
/// Order matters: fonts/tokens/base are primary, screens override components.
const APP_STYLES: &str = concat!(
    include_str!("../assets/fonts.css"),
    include_str!("../assets/tokens.css"),
    include_str!("../assets/base.css"),
    include_str!("../assets/components.css"),
    include_str!("../assets/app.css"),
);

/// Main function launching the Dioxus application.
fn main() {
    // Test-only JS emission mode (compiled only with the `e2e` feature):
    // `mailgrit-app-desktop --emit-batch-js <rows.json>` prints the generated
    // batch IIFE to stdout and exits — no window, no webview. Consumed by the
    // Node smoke harness (e2e/js-smoke), which EXECUTES the IIFE against a
    // mocked fetch to test the JS control flow end-to-end (the Rust-side
    // string-presence tests cannot catch a logically broken builder).
    #[cfg(feature = "e2e")]
    if std::env::args().nth(1).as_deref() == Some("--emit-batch-js") {
        let rows_path = std::env::args().nth(2).unwrap_or_default();
        emit_batch_js_main(&rows_path);
        return;
    }

    let _log_guard = logging::init();

    // Login window state is thread_local (WebView is !Send).
    let login_state = login_window::login_state();
    let data_dir = app_data_dir();

    let ls_for_handler = Arc::clone(&login_state);
    let dir_for_handler = data_dir.clone();
    dioxus::LaunchBuilder::new()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_window(
                    dioxus::desktop::WindowBuilder::new()
                        .with_title(brand::APP_NAME)
                        .with_window_icon(window_icon::window_icon())
                        // Frameless window — custom titlebar (components/titlebar).
                        .with_decorations(false)
                        .with_inner_size(dioxus::desktop::LogicalSize::new(1120.0, 780.0))
                        .with_min_inner_size(dioxus::desktop::LogicalSize::new(720.0, 520.0)),
                )
                // Shared data_dir -> shared WebView2 cookie-store with the login webview.
                .with_data_directory(data_dir)
                .with_exits_when_last_window_closes(true)
                // Inline CSS via custom_head.
                // Note: Dioxus 0.7 hardcodes `<title>Dioxus app</title>` in
                // index.html BEFORE CUSTOM HEAD, so the document title is set
                // not here but by a runtime effect (see app() -> apply_document_title).
                .with_custom_head(format!("<style>{APP_STYLES}</style>"))
                // Event handler: builds the login window on demand and handles its close.
                .with_custom_event_handler(move |event, target| {
                    login_window::handle_event(&ls_for_handler, &dir_for_handler, event, target);
                }),
        )
        .launch(app);
}

/// Sets `document.title` (the document title inside the WebView) via evaluate_script.
///
/// Mirror of [`theme::apply_theme`]: same mechanism (`use_window().webview`).
/// Needed because Dioxus 0.7 hardcodes `<title>Dioxus app</title>` in index.html
/// BEFORE the `custom_head` block — the browser uses the first `<title>`, so the
/// static `<title>` in `custom_head` is ignored. Runtime override is reliable.
fn apply_document_title(title: &str) {
    let window = dioxus::desktop::use_window();
    // title is a brand constant (no user input), so it needs no escaping.
    let js = format!("document.title = {title:?};");
    if let Err(e) = window.webview.evaluate_script(&js) {
        tracing::warn!("setting document.title: {e}");
    }
}

/// Root application component: titlebar + shell + routing between screens.
fn app() -> Element {
    let state = use_context_provider(|| Signal::new(AppState::new()));
    // The E2E hook is applied ONCE at startup (use_hook is not re-run on
    // re-renders) — otherwise logout (resetting screen=Login) would immediately
    // revert to Dashboard by re-applying the hook. Applied BEFORE the first
    // read() so that subsequent subscriptions observe the overridden state.
    // Compiled only with the `e2e` cargo feature (test builds); release builds
    // do not contain the hook at all.
    #[cfg(feature = "e2e")]
    use_hook(|| e2e_state::apply_e2e_overrides(&state));
    let screen = state.read().screen;
    // Subscribe to the language: the titlebar subtitle and localized tooltips/window
    // titles (tr!/t! read the global locale, not the Signal) must re-render when the
    // language changes. Without this the titlebar would keep the old language until
    // the screen changed.
    let _language = state.read().language;

    // Apply the theme to <html> on every change. We read the Signal INSIDE the
    // effect so that Dioxus tracks the dependency and re-runs the effect.
    // We also set document.title here: Dioxus 0.7 hardcodes `<title>Dioxus app</title>`
    // in index.html BEFORE the custom_head block (the browser takes the first <title>),
    // so we override it at runtime via evaluate_script. The name is static; the
    // effect runs at startup (the WebView is already ready inside use_effect).
    // a11y: screen readers and tabs read document.title, not the OS window title (with_title).
    use_effect(move || {
        let theme = state.read().theme;
        theme::apply_theme(theme);
        apply_document_title(brand::APP_NAME);
    });

    // Apply the UI language globally (rust_i18n::set_locale) on every change.
    use_effect(move || {
        let lang = state.read().language;
        rust_i18n::set_locale(lang.as_str());
    });

    // Register the data-driven auto-login callback (once at startup).
    // The login webview's page_load_handler wakes handle_event, which checks the
    // login predicate and invokes the callback; the callback mutates Signal<AppState>
    // via spawn_forever.
    //
    // The callback runs on the event-loop thread, OUTSIDE the Dioxus runtime scope,
    // so we capture the Runtime in use_hook and set up a RuntimeGuard before spawning.
    use_hook(|| {
        let ls = login_window::login_state();
        let runtime = dioxus::core::Runtime::current();
        // Pass the session cookie name to LoginWindowState (handle_event has no
        // access to AppState and reads the name from here).
        ls.set_session_cookie_name(state.read().session_cookie_name.clone());
        ls.set_on_login(Box::new(move || {
            let mut state = state;
            // Restore the runtime for a callback invoked from the event loop.
            let _guard = dioxus::core::RuntimeGuard::new(runtime.clone());
            dioxus::core::spawn_forever(async move {
                // Guard against repeated firing (page_load Finished can be
                // called multiple times during operations inside the webview).
                if state.read().session_ok {
                    tracing::debug!("auto-login: already logged in — skipping");
                    return;
                }
                tracing::info!("auto-login: switching to the operations panel");
                let ls = login_window::login_state();
                ls.hide();
                // Cookies for the diagnostics panel (informational).
                let base_url = state.read().base_url.clone();
                let cookies = match auth_bridge::read_cookies_for_panel(&ls, &base_url) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!("failed to read cookies for the panel: {e}");
                        Vec::new()
                    }
                };
                let mut s = state.write();
                s.session_ok = true;
                s.auth_status = AuthStatus::Connected;
                s.screen = Screen::Dashboard;
                s.last_cookies = cookies;
                s.error_msg = None;
                s.refresh_audit();
            });
        }));
    });

    // The titlebar subtitle depends on the current screen.
    let subtitle = match screen {
        Screen::Login => t!("subtitle.login").to_string(),
        Screen::Dashboard => t!("subtitle.dashboard").to_string(),
    };

    rsx! {
        div { class: "app-shell",
            TitleBar { subtitle }
            div { class: "app-screen",
                if screen == Screen::Login {
                    login_screen {}
                } else {
                    dashboard_screen {}
                }
            }
        }
    }
}

/// E2E-only helper for the Node smoke harness (e2e/js-smoke): reads the rows
/// JSON, builds the batch JS via the REAL production builder, prints it to
/// stdout. The rows JSON is an array of 5-string arrays in CSV column order
/// (domain, username, password, display_name, quota) — it goes through the
/// canonical typestate parser, exactly like a loaded CSV.
///
/// No `println!`/`eprintln!` (denied workspace-wide) and no `panic!`: errors
/// go to stderr via `Write` and the process exits with code 2.
#[cfg(feature = "e2e")]
fn emit_batch_js_main(rows_path: &str) {
    use std::io::Write as _;
    fn bail(msg: &str) -> ! {
        let mut err = std::io::stderr();
        let _ = writeln!(err, "emit-batch-js: {msg}");
        let _ = err.flush();
        std::process::exit(2);
    }

    let usage =
        "usage: --emit-batch-js <rows.json> [user|domain|admin] [create|edit|delete] [verify 0|1]";
    if rows_path.is_empty() {
        bail(&format!("rows file missing — {usage}"));
    }
    let target = match std::env::args().nth(3).as_deref() {
        None | Some("user") => mailgrit_core_domain::OperationTarget::User,
        Some("domain") => mailgrit_core_domain::OperationTarget::Domain,
        Some("admin") => mailgrit_core_domain::OperationTarget::Admin,
        Some(other) => bail(&format!("unknown target {other:?} — {usage}")),
    };
    let kind = match std::env::args().nth(4).as_deref() {
        None | Some("create") => mailgrit_core_domain::BulkOperationKind::Create,
        Some("edit") => mailgrit_core_domain::BulkOperationKind::Edit,
        Some("delete") => mailgrit_core_domain::BulkOperationKind::Delete,
        Some(other) => bail(&format!("unknown kind {other:?} — {usage}")),
    };
    let verify = std::env::args().nth(5).as_deref() != Some("0");

    let rows_json = std::fs::read_to_string(rows_path)
        .unwrap_or_else(|e| bail(&format!("reading {rows_path}: {e}")));
    let parsed: Vec<Vec<String>> = serde_json::from_str(&rows_json)
        .unwrap_or_else(|e| bail(&format!("parsing {rows_path} (expected [[domain, username, password, display_name, quota], ...]): {e}")));
    let mut rows = Vec::with_capacity(parsed.len());
    for (i, cols) in parsed.iter().enumerate() {
        let row = mailgrit_core_domain::RawCsvRow::new(cols.to_vec())
            .parse()
            .unwrap_or_else(|e| bail(&format!("row #{i} failed the canonical parser: {e}")));
        rows.push(row);
    }

    let js = webview_ops::build_batch_js(
        1,
        target,
        kind,
        "https://mail.example.com/iredadmin",
        &rows,
        verify,
    );
    let mut out = std::io::stdout();
    let _ = out.write_all(js.as_bytes());
    let _ = out.write_all(b"\n");
    let _ = out.flush();
}
