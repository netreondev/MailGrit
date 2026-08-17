//! Request/result types for the login-webview. See the lifecycle overview in
//! the root of [`crate::login_window`].
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/// A single cookie for display in the diagnostics panel (Send — for oneshot).
#[derive(Debug, Clone)]
pub struct CookieInfo {
    /// Cookie name.
    pub name: String,
    /// Domain.
    pub domain: Option<String>,
    /// Path.
    pub path: Option<String>,
    /// `HttpOnly` flag.
    pub http_only: bool,
    /// Value length (the value itself is not shown — it is a secret).
    pub value_len: usize,
}

/// Result of executing a batch of operations (an array of per-row results).
pub type BatchOpResult = Vec<crate::webview_ops::OpResult>;

/// Pending request to execute a batch of operations via the login-webview.
pub struct OpRequest {
    /// Operation target (domain/user/administrator).
    pub target: mailgrit_core_domain::OperationTarget,
    /// Operation kind (Create/Edit/Delete).
    pub kind: mailgrit_core_domain::BulkOperationKind,
    /// iRedAdmin Base URL.
    pub base_url: String,
    /// CSV rows to process.
    pub rows: Vec<mailgrit_core_domain::SanitizedUserRow>,
    /// Result return channel.
    pub tx: tokio::sync::oneshot::Sender<BatchOpResult>,
}

/// Pending request for form diagnostics (GET + returns the form fields' HTML).
pub struct DiagRequest {
    /// Domain for the create form.
    pub domain: String,
    /// JSON diagnostics return channel.
    pub tx: tokio::sync::oneshot::Sender<String>,
}

/// Request to open the login window (iRedAdmin `base_url`).
#[derive(Debug, Clone)]
pub struct LoginRequest {
    /// Full iRedAdmin URL (<https://host/iredadmin>).
    pub base_url: String,
}

/// Login-webview load event — the trigger for data-driven auto-auth.
///
/// Login predicate: the URL contains `/dashboard` (the canonical post-login
/// redirect of iRedAdmin; more reliable behind `FortiWeb` — the WAF hides the
/// backend cookie) OR the `webpy_session_id` cookie is present (for environments
/// without a WAF).
#[derive(Debug, Clone)]
pub struct AuthEvent {
    /// iRedAdmin Base URL (origin for reading the domain cookie).
    pub base_url: String,
    /// Final URL after loading and redirects (from `page_load` Finished).
    pub final_url: String,
}
