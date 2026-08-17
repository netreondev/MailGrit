//! Registry of iRedAdmin bulk operations — generation of JS code executed inside
//! the login-WebView2.
//!
//! See the rationale for the webview approach (FortiWeb/WAF) at the root of
//! [`crate::webview_ops`].
//!
//! `MailGrit` works only with open-source iRedAdmin (OSE, HTML forms). The main
//! entry point [`build_batch_js`] dispatches by
//! [`mailgrit_core_domain::OperationTarget`]:
//! - `User` → [`user::build_user_batch_js`] (CSRF + create/edit/delete user form);
//! - `Domain` → [`domain::build_domain_batch_js`] (OSE domain forms);
//! - `Admin` → [`admin::build_admin_batch_js`] (OSE admin forms).
//!
//! This keeps `mod.rs` compact (≤400 lines), with each target's implementation in
//! its own module. The diagnostics builder is in [`crate::webview_js_extra`].
//!
//! # Operation success verdict
//! iRedAdmin returns HTTP 200 even on error (`ALREADY_EXISTS`, etc.), so the status
//! code alone is insufficient. See the target modules ([`user`], [`domain`],
//! [`admin`]) for verdict details.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use mailgrit_core_domain::{BulkOperationKind, OperationTarget, SanitizedUserRow};

mod admin;
mod domain;
mod helpers;
mod shared;
mod user;

/// Main entry point of the operation registry: builds the JS code for a batch by
/// the (target, kind) pair of an OSE operation. Called from the login-webview
/// `handle_event`.
///
/// Dispatch by `target`:
/// - [`OperationTarget::User`] → user pipeline (CSRF + form
///   `username`/`newpw`/`confirmpw`/`cn`/`preferredLanguage`/`mailQuota`);
/// - [`OperationTarget::Domain`] → OSE domain forms (`domainName`/`quota`/
///   `transport`/`is_backupmx`);
/// - [`OperationTarget::Admin`] → OSE admin forms (`mail`/`newpw`).
///
/// The result is an IIFE string `(async () => { ... })()` executed via
/// `evaluate_script`. The response arrives over IPC as `batch:{id}:{json}`.
#[must_use]
pub fn build_batch_js(
    id: u64,
    target: OperationTarget,
    kind: BulkOperationKind,
    base_url: &str,
    rows: &[SanitizedUserRow],
    verify: bool,
) -> String {
    match target {
        OperationTarget::User => user::build_user_batch_js(id, kind, base_url, rows, verify),
        OperationTarget::Domain => domain::build_domain_batch_js(id, kind, base_url, rows, verify),
        OperationTarget::Admin => admin::build_admin_batch_js(id, kind, base_url, rows, verify),
    }
}

#[cfg(test)]
#[path = "../webview_js_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../webview_js_domain_tests.rs"]
mod domain_admin_tests;
