//! Generation of the JS code for a bulk **user** (User) operation executed
//! inside the login-WebView2.
//!
//! This is the registry branch for `OperationTarget::User` (see
//! [`super::build_batch_js`]). The entire OSE pipeline lives here: CSRF, doOp,
//! verifyOp.
//!
//! The module structure mirrors [`super::domain`] and [`super::admin`]:
//! `build_fields_js` (const fn) + `endpoints_for` (const fn) + `build_user_do_op_js`
//! + `csrf_mask_verify_js` + `build_user_batch_js`.
//!
//! # Operation success verdict
//! iRedAdmin returns HTTP 200 even on error (ALREADY_EXISTS, etc.), so the status
//! code alone is insufficient. The verdict is built from three signals (see
//! defects D1–D4):
//!   - **HTTP OK** (200/302/303);
//!   - **a positive marker** in the URL/body (`?msg=CREATED/UPDATED/DELETED`,
//!     `note-success`) — a mandatory condition (previously dead code, D1);
//!   - **absence of an error marker** (`note-error`/`note-warning`/`note-danger`/
//!     `ALREADY_EXISTS`/... — an extended list, D2).
//!
//! The markers are centralized in [`crate::webview_markers`] (the single source
//! of truth, covered by Rust tests and interpolated into JS via
//! `build_marker_js`). For create/delete, post-verification is additionally run
//! (D4): a repeated profile GET confirms the actual operation result.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use mailgrit_core_domain::{BulkOperationKind, SanitizedUserRow};

/// Returns a JS string defining the function `buildFields(csrf, row)` — the form
/// fields for a given OSE operation (create/edit/delete). Factored into a
/// separate `const fn` for the compactness of `build_user_batch_js` (symmetric
/// with domain/admin).
#[must_use]
const fn build_fields_js(kind: BulkOperationKind) -> &'static str {
    match kind {
        BulkOperationKind::Create => {
            r"
            function buildFields(csrf, row) {
                return [
                    ['csrf_token', csrf],
                    ['domainName', row.domain],
                    ['username', row.username],
                    ['newpw', row.password],
                    ['confirmpw', row.password],
                    ['cn', row.display_name],
                    ['preferredLanguage', 'en_US'],
                    ['mailQuota', String(row.quota)],
                    ['submit_add_user', 'Add']
                ];
            }
        "
        }
        BulkOperationKind::Edit => {
            r"
            function buildFields(csrf, row) {
                return [
                    ['csrf_token', csrf],
                    ['cn', row.display_name],
                    ['mailQuota', String(row.quota)],
                    ['accountStatus', 'active'],
                    ['mail', row.email]
                ];
            }
        "
        }
        BulkOperationKind::Delete => {
            r"
            function buildFields(csrf, row) {
                // iRedAdmin OSE deletes via the bulk action on the list page:
                // checkbox (mail=user@domain) + action=delete + submit_users=...
                return [
                    ['csrf_token', csrf],
                    ['mail', row.email],
                    ['action', 'delete'],
                    ['submit_users', 'Delete']
                ];
            }
        "
        }
    }
}

/// The endpoint, `formActionSuffix`, and the JS kind literal for a user operation.
///
/// Returns JS expressions for interpolation into `doOp` (via [`DoOpSpec`]):
/// - Create → `/create/user/<domain>`;
/// - Edit → `/profile/user/general/<email>`;
/// - Delete → the bulk action on `/users/<domain>`.
const fn endpoints_for(kind: BulkOperationKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        BulkOperationKind::Create => (
            r"base + '/create/user/' + encodeURIComponent(row.domain)",
            r"'/create/user/' + encodeURIComponent(row.domain)",
            "'create'",
        ),
        BulkOperationKind::Delete => {
            // Deletion via the user list page (bulk action).
            (
                r"base + '/users/' + encodeURIComponent(row.domain)",
                r"'/users/' + encodeURIComponent(row.domain)",
                "'delete'",
            )
        }
        // Edit: user profile (the OSE edit form).
        BulkOperationKind::Edit => (
            r"base + '/profile/user/general/' + encodeURIComponent(row.email)",
            r"'/profile/user/general/' + encodeURIComponent(row.email)",
            "'edit'",
        ),
    }
}

/// Builds the JS function `doOp` for a user operation (OSE forms), delegating the
/// unified pipeline to [`super::shared::build_do_op_js`].
fn build_user_do_op_js(kind: BulkOperationKind) -> String {
    let (path_fn, form_action_suffix, kind_js) = endpoints_for(kind);
    super::shared::build_do_op_js(&super::shared::DoOpSpec {
        path_fn,
        form_action_suffix,
        kind_js,
        // Email is the canonical user identifier for post-verification.
        verify_target_js: "row.email",
        log_tag: "OP",
        log_id_js: "row.username",
    })
}

/// JS fragment: the mfMask/getCsrf/verifyOp helper functions for a user.
///
/// Builds the JS code that runs ALL rows of a user operation batch sequentially
/// via fetch inside the login-webview. The result is sent through
/// `window.ipc.postMessage` (NOT via return — evaluate_script does not await a
/// Promise).
///
/// Delegates the shared skeleton to [`super::shared::build_target_batch_js`]; the
/// user target differs only by the profile segment `/profile/user/general/` and
/// the CSRF log tag. Before deduplication this skeleton was copied in each target
/// module.
pub(super) fn build_user_batch_js(
    id: u64,
    kind: BulkOperationKind,
    base_url: &str,
    rows: &[SanitizedUserRow],
    verify: bool,
) -> String {
    let build_fields = build_fields_js(kind);
    let do_op = build_user_do_op_js(kind);
    super::shared::build_target_batch_js(
        id,
        base_url,
        rows,
        verify,
        &super::shared::TargetBatchSpec {
            csrf_log_tag: "CSRF",
            profile_segment: "/profile/user/general/",
            build_fields,
            do_op: &do_op,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Endpoints: create → /create/user/, edit → /profile/user/general/,
    // delete → /users/.
    #[test]
    fn user_endpoints_for_all_kinds() {
        let (c, _, _) = endpoints_for(BulkOperationKind::Create);
        assert!(c.contains("/create/user/"));
        let (e, _, _) = endpoints_for(BulkOperationKind::Edit);
        assert!(e.contains("/profile/user/general/"));
        let (d, _, _) = endpoints_for(BulkOperationKind::Delete);
        assert!(d.contains("/users/"));
    }

    // buildFields create/edit/delete contain the required field names.
    #[test]
    fn user_build_fields_has_correct_form_names() {
        assert!(build_fields_js(BulkOperationKind::Create).contains("domainName"));
        assert!(build_fields_js(BulkOperationKind::Create).contains("submit_add_user"));
        assert!(build_fields_js(BulkOperationKind::Edit).contains("accountStatus"));
        assert!(build_fields_js(BulkOperationKind::Delete).contains("submit_users"));
        assert!(build_fields_js(BulkOperationKind::Delete).contains("'action', 'delete'"));
    }
}
