//! iRedAdmin bulk operations executed THROUGH the login-WebView2 (JS fetch).
//!
//! WHY: your iRedAdmin server sits behind a FortiWeb WAF, which sets its own
//! session cookie (`cookiesession1`) and holds the iRedAdmin backend session at
//! its end. Replaying the cookie in `reqwest` does NOT authenticate against the
//! backend. Therefore the requests are executed INSIDE WebView2 via JS
//! `fetch(...)` — it automatically uses the browser cookies (the legitimate
//! FortiWeb session), and the backend sees the real session.
//!
//! Flow:
//!   1. A panel button submits a request for an operation batch into
//!      `LoginWindowState`.
//!   2. `handle_event` (the top level of the loop) sees the request and starts
//!      processing the queue — one operation at a time via
//!      `evaluate_script_with_callback` (JS `fetch`); the result returns in the
//!      callback → next.
//!   3. The outcome (BatchResult) is sent to the UI thread via oneshot → `spawn`
//!      → `Signal`.
//!
//! Submodules (to comply with the spec's file-size limit of ≤400 lines):
//!   - [`crate::webview_js`]      — generation of JS builders (`build_diag_js`,
//!     `build_batch_js`);
//!   - [`crate::webview_markers`] — success/error markers (D1–D3) and their JS
//!     projection;
//!   - [`crate::webview_parse`]   — parsing of JSON responses
//!     (`parse_batch_result`).

use mailgrit_core_domain::SanitizedUserRow;

pub use crate::webview_js::build_batch_js;
pub use crate::webview_js_extra::build_diag_js;
pub use crate::webview_parse::parse_batch_result;

/// The result of a single operation on a row.
#[derive(Debug, Clone)]
pub struct OpResult {
    /// The username.
    pub username: String,
    /// The domain.
    pub domain: String,
    /// Ok(()) or the failure reason.
    pub outcome: Result<(), String>,
    /// The response HTTP status (0 on a network failure / no response).
    /// Used by the session-expiry detector (P0): an objective signal independent
    /// of the human-readable reason text (which, after E1, no longer contains
    /// "401/403", as the old detector expected).
    pub status: i64,
    /// The final response URL (after redirects), from `dump.responseUrl`.
    /// Contains `/login` on session expiry → the detector (P0). `None` if JS did
    /// not pass dump (network failure).
    pub resp_url: Option<String>,
    /// The final post-verification URL (profile GET), from `dump.verifyUrl`.
    /// The session may expire between a successful POST and the verify-GET: then
    /// this URL contains `/login`, while `resp_url` (the POST-url) does not.
    /// Without this field, the detector missed session expiry in the verify
    /// window (P0).
    pub verify_url: Option<String>,
}

/// Serializes a row into a JSON object (for embedding into JS).
pub fn row_to_json(row: &SanitizedUserRow) -> serde_json::Value {
    serde_json::json!({
        "domain": row.domain.as_str(),
        "username": row.username.as_str(),
        "password": row.password.as_secret_str(),
        "display_name": row.display_name.as_str(),
        "quota": row.quota.mb(),
        "email": format!("{}@{}", row.username.as_str(), row.domain.as_str()),
    })
}

#[cfg(test)]
#[path = "webview_secret_leak_tests.rs"]
mod secret_leak_tests;
