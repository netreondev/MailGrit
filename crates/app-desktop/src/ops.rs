//! Handlers for bulk operations launched from the UI.
//!
//! Wire the UI state (`Signal<AppState>`) to the webview operations
//! (`login_window`), the audit log, and the local encrypted export.

use crate::batch::{BatchResult, CredentialRow, RowFailure};
use crate::login_window;
use crate::op_label::operation_label;
use crate::state::{AppState, AuthStatus, OpStatus, Screen};
use crate::util::now_rfc3339;
use dioxus::prelude::*;
use mailgrit_core_domain::{BulkOperationKind, OperationTarget};
use mailgrit_core_storage::AuditAction;
use std::sync::Arc;

// Re-export of the export entry point from ops_export (extracted for ≤400
// lines/file).
pub use crate::ops_export::{do_export, open_export_choice};

/// Launches a bulk operation (create/edit/delete) via the login-webview.
///
/// The requests are executed INSIDE the login-webview (JS fetch), because behind
/// a FortiWeb WAF replaying the cookie in reqwest does not authenticate against
/// the backend. Result → oneshot → spawn → Signal.
#[expect(
    clippy::too_many_lines,
    reason = "a single operation pipeline with sequential checks; splitting harms locality"
)]
pub fn launch_op(state: &mut Signal<AppState>, target: OperationTarget, kind: BulkOperationKind) {
    let base_url;
    let rows;
    let edit_errors;
    {
        let read = state.read();
        if !read.session_ok {
            drop(read);
            state.write().error_msg = Some(t!("operr.no_session").to_string());
            return;
        }
        if read.editable_rows.as_ref().is_none_or(Vec::is_empty) {
            drop(read);
            state.write().error_msg = Some(t!("operr.no_rows").to_string());
            return;
        }
        base_url = read.base_url.clone();
        drop(read);
        // Re-validate the editable rows through the typestate pipeline; invalid
        // ones are skipped (fail-soft).
        let (valid, errors) = crate::screens::csv_load::collect_sanitized_rows(state);
        rows = valid;
        edit_errors = errors;
    }
    if rows.is_empty() {
        let detail = if edit_errors.is_empty() {
            t!("operr.no_valid_rows").to_string()
        } else {
            t!("operr.no_valid_rows_with_errors", n = edit_errors.len()).to_string()
        };
        state.write().error_msg = Some(detail);
        return;
    }

    let op_label = operation_label(target, kind);
    tracing::info!("batch {}: {} rows", op_label, rows.len());

    // Expected number of rows: an empty result on a non-empty batch indicates
    // failure.
    let expected = u64::try_from(rows.len()).unwrap_or(0);

    // Take the credential snapshot for export BEFORE `rows` is moved into
    // `request_op`. Passwords are only needed for create; for edit/delete the
    // snapshot is empty, but the copy is still cheap (Arc-backed strings).
    let credential_snapshot: Vec<CredentialRow> = match kind {
        BulkOperationKind::Create => rows
            .iter()
            .map(|r| CredentialRow {
                domain: r.domain.as_str().to_owned(),
                username: r.username.as_str().to_owned(),
                password: r.password.as_secret_str().to_owned(),
                display_name: r.display_name.as_str().to_owned(),
                quota_mb: r.quota.mb(),
            })
            .collect(),
        BulkOperationKind::Edit | BulkOperationKind::Delete => Vec::new(),
    };

    let login_state = login_window::login_state();
    let Some(rx) = login_state.request_op(target, kind, base_url, rows) else {
        tracing::warn!(
            "batch {}: request rejected — an operation is already running",
            op_label
        );
        state.write().error_msg = Some(t!("operr.already_running").to_string());
        return;
    };

    // The Running status is set AFTER successful registration of the request —
    // otherwise, if a duplicate is rejected (above), the indicator would stay in
    // Running forever. Same order as in `run_diag` below.
    state.write().op_status = OpStatus::Running;
    state.write().error_msg = None;

    let mut state_clone = *state;
    spawn(async move {
        if let Ok(results) = rx.await {
            let total = u64::try_from(results.len()).unwrap_or(0);
            // A silent empty result on a non-empty batch = a webview failure.
            if total == 0 && expected > 0 {
                tracing::warn!("batch returned 0 results for {expected} rows — webview failure");
                let mut s = state_clone.write();
                s.op_status = OpStatus::Idle;
                s.error_msg = Some(t!("operr.webview_no_result").to_string());
                return;
            }
            let succeeded =
                u64::try_from(results.iter().filter(|r| r.outcome.is_ok()).count()).unwrap_or(0);
            let failed = total.saturating_sub(succeeded);
            tracing::info!("batch completed: succeeded {succeeded}, rejected {failed}");

            let failures: Vec<RowFailure> = results
                .iter()
                .filter_map(|r| match &r.outcome {
                    Ok(()) => None,
                    Err(reason) => Some(RowFailure {
                        username: r.username.clone(),
                        domain: r.domain.clone(),
                        reason: reason.clone(),
                    }),
                })
                .collect();

            // Session-expiry detector: if ALL rows failed with a sign of session
            // loss (HTTP 401/403 or a redirect to /login), return the user to the
            // login screen (the "Session active" badge was misleading).
            let session_lost = total > 0
                && succeeded == 0
                && results.iter().all(|r| {
                    is_session_expired(
                        r.status,
                        &r.outcome,
                        r.resp_url.as_deref(),
                        r.verify_url.as_deref(),
                    )
                });
            if session_lost {
                tracing::warn!("session expired during operation — returning to login screen");
                let mut s = state_clone.write();
                s.op_status = OpStatus::Idle;
                s.session_ok = false;
                s.auth_status = AuthStatus::None;
                s.error_msg = Some(t!("operr.session_expired").to_string());
                s.screen = Screen::Login;
                s.batch_result = None;
                s.csv = None;
                s.modals.pending_delete = false;
                return;
            }

            // Audit record: the (target × kind) → action mapping is in
            // `audit_action_for`.
            let action = audit_action_for(target, kind);
            // Credential snapshot for export. Passwords live only in
            // `editable_rows`, which is reset on target change (tab), after which
            // the export would lose data. Take the password from the snapshot
            // only for SUCCESSFUL operations (results come in row order): a
            // guarantee that the password matches what was sent to the server.
            // For edit/delete no accounts are created — the snapshot is already
            // empty.
            let created_credentials = successful_credentials(&credential_snapshot, &results);
            let result = BatchResult {
                succeeded,
                failed,
                failures,
                created_credentials,
            };
            let timestamp = now_rfc3339();
            // Audit record: clone the Arc<AuditWriter> in a short read-scope,
            // release the signal borrow BEFORE the blocking SQLite-INSERT —
            // otherwise `&state_clone.read().audit` is kept alive across
            // append_op, which is a reentrant anti-pattern (similarly to export).
            let audit = { state_clone.read().audit.clone() };
            if let Some(audit) = &audit
                && let Err(e) = audit.append_op(action, &result, &timestamp)
            {
                tracing::warn!("audit record failed: {e}");
            }
            let mut s = state_clone.write();
            s.batch_result = Some(Arc::new(result));
            s.op_status = OpStatus::Idle;
            s.refresh_audit();
        } else {
            tracing::warn!("operation-batch channel closed (cancelled?)");
            let mut s = state_clone.write();
            s.op_status = OpStatus::Idle;
            s.error_msg = Some(t!("operr.cancelled").to_string());
        }
    });
}

/// Maps a (target × kind) pair to an [`AuditAction`] for the audit log.
#[must_use]
const fn audit_action_for(target: OperationTarget, kind: BulkOperationKind) -> AuditAction {
    match (target, kind) {
        (OperationTarget::User, BulkOperationKind::Create) => AuditAction::CreateUser,
        (OperationTarget::User, BulkOperationKind::Delete) => AuditAction::DeleteUser,
        (OperationTarget::User, BulkOperationKind::Edit) => AuditAction::EditUser,
        (OperationTarget::Domain, BulkOperationKind::Create) => AuditAction::CreateDomain,
        (OperationTarget::Domain, BulkOperationKind::Delete) => AuditAction::DeleteDomain,
        (OperationTarget::Domain, BulkOperationKind::Edit) => AuditAction::EditDomain,
        (OperationTarget::Admin, BulkOperationKind::Create) => AuditAction::CreateAdmin,
        (OperationTarget::Admin, BulkOperationKind::Delete) => AuditAction::DeleteAdmin,
        (OperationTarget::Admin, BulkOperationKind::Edit) => AuditAction::EditAdmin,
    }
}

/// Filters the credential snapshot, keeping only successfully created accounts.
///
/// `results` come in the same order as the rows of the original batch (the
/// webview-JS processes rows sequentially and `push`es the result of each — see
/// `batch_iife_js`). So `results[i].outcome.is_ok()` unambiguously answers
/// whether `credential_snapshot[i]` was created. On a length mismatch (a webview
/// failure), intersect by the shorter side — fail-safe.
#[must_use]
fn successful_credentials(
    credential_snapshot: &[CredentialRow],
    results: &[crate::webview_ops::OpResult],
) -> Vec<CredentialRow> {
    credential_snapshot
        .iter()
        .zip(results.iter())
        .filter_map(|(cred, r)| {
            if r.outcome.is_ok() {
                Some(cred.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Form diagnostics: GET the user-creation page and return an HTML of the fields.
/// Shows the real field names, action URL, and CSRF for precise operation tuning.
pub fn run_diag(state: &mut Signal<AppState>) {
    let domain = state
        .read()
        .editable_rows
        .as_ref()
        .and_then(|rows| rows.first())
        .map(|r| r.domain.clone());
    let Some(domain) = domain.filter(|d| !d.trim().is_empty()) else {
        state.write().error_msg = Some(t!("operr.diag_need_csv").to_string());
        return;
    };
    let login_state = login_window::login_state();
    let Some(rx) = login_state.request_diag(domain) else {
        tracing::warn!("diagnostics: request rejected — an operation is already running");
        state.write().error_msg = Some(t!("operr.already_running").to_string());
        return;
    };
    state.write().op_status = OpStatus::Running;
    state.write().error_msg = Some(t!("operr.diag_running").to_string());
    let mut state_clone = *state;
    spawn(async move {
        // Guaranteed to reset the status even if parsing panics.
        let result = rx.await;
        state_clone.write().op_status = OpStatus::Idle;
        if let Ok(json) = result {
            tracing::info!("=== FORM DIAGNOSTICS ===\n{json}");
            let pretty = format_diag(&json);
            state_clone.write().error_msg =
                Some(t!("operr.diag_prefix", details = pretty).to_string());
        } else {
            state_clone.write().error_msg = Some(t!("operr.diag_cancelled").to_string());
        }
    });
}

/// Formats the form-diagnostics JSON response into a brief summary for the UI.
/// Extracts the status, URL, and the list of forms (action + field names).
fn format_diag(json: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return t!("operr.diag_parse_error", details = json).to_string();
    };
    let status = v
        .get("status")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let url = v
        .get("url")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    let form_summary = summarize_forms(&v);
    t!(
        "operr.diag_summary",
        status = status,
        url = url,
        forms = form_summary
    )
    .to_string()
}

/// A summary of forms from the diagnostics JSON: `action=<a>, fields=[f1, f2]; ...`.
fn summarize_forms(v: &serde_json::Value) -> String {
    let Some(forms) = v
        .get("forms_in_response")
        .or_else(|| v.get("forms_on_page"))
        .and_then(serde_json::Value::as_array)
    else {
        return t!("operr.diag_no_forms").to_string();
    };
    let mut parts = Vec::new();
    for f in forms {
        let action = f
            .get("action")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("?");
        let field_names: Vec<&str> = f
            .get("inputs")
            .and_then(serde_json::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|i| i.get("name").and_then(serde_json::Value::as_str))
                    .collect()
            })
            .unwrap_or_default();
        parts.push(format!(
            "action={action}, fields=[{}]",
            field_names.join(", ")
        ));
    }
    parts.join("; ")
}

/// Indicator of session expiry/absence by an operation result.
///
/// Relies on objective signals:
/// - **HTTP status** 401/403 — an explicit authentication error;
/// - **the final URL of the POST response** (`resp_url`) contains `/login` —
///   iRedAdmin redirects to the login form on session expiry (after `fetch` with
///   `redirect:'follow'` the status stays 200, but the URL reveals the redirect);
/// - **the final post-verification URL** (`verify_url`) contains `/login` — the
///   session may expire BETWEEN a successful POST and the verify-GET; previously
///   only the POST-url was checked, and expiry in the verify window was missed
///   (the reason "profile not found after create" carries no sign of session).
///   P0 fix;
/// - **the reason text** (fallback) — `login_required`, `csrf token not found`,
///   `401`/`403`.
///
/// A real server error (e.g. `NO_SUCH_ACCOUNT` at `status=200`) is NOT counted as
/// session expiry.
#[must_use]
fn is_session_expired(
    status: i64,
    outcome: &Result<(), String>,
    resp_url: Option<&str>,
    verify_url: Option<&str>,
) -> bool {
    if status == 401 || status == 403 {
        return true;
    }
    if resp_url.is_some_and(|u| u.contains("/login")) {
        return true;
    }
    if verify_url.is_some_and(|u| u.contains("/login")) {
        return true;
    }
    // Fallback by the reason text.
    let reason = match outcome {
        Ok(()) => "",
        Err(r) => r.as_str(),
    };
    is_session_expired_reason(reason)
}

/// Textual fallback indicator of session expiry (for export errors).
#[must_use]
pub fn is_session_expired_reason(reason: &str) -> bool {
    let r = reason.to_ascii_lowercase();
    r.contains("401")
        || r.contains("403")
        || r.contains("/login")
        || r.contains("login_required")
        || r.contains("csrf token not found")
}

#[cfg(test)]
#[path = "ops_tests.rs"]
mod tests;
