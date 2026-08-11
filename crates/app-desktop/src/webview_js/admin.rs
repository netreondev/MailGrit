//! Generation of the JS code for a bulk **admin** (Admin) operation executed
//! inside the login-WebView2.
//!
//! This is the registry branch for `OperationTarget::Admin` (see
//! [`super::build_batch_js`]). It implements the iRedAdmin OSE forms
//! (create/edit/delete admins).
//!
//! # OSE endpoints (iRedAdmin open-source, `controllers/sql/admin.py`)
//! - **Create**: `POST /create/admin`. Form fields:
//!   `csrf_token, mail (admin email), newpw, confirmpw, submit_add_admin`.
//! - **Edit**: `POST /profile/admin/general/<email>`. Fields:
//!   `csrf_token, mail, ...`.
//! - **Delete**: the bulk action on `/admins`:
//!   `csrf_token, mail=<email>, action=delete, submit_admins=Delete`.
//!
//! The admin email = `username@domain` (computed in JS from row).
//!
//! # Success verdict
//! The universal iRedAdmin markers from [`crate::webview_markers`] are reused
//! (`?msg=CREATED/UPDATED/DELETED`, `note-error/note-success`).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use mailgrit_core_domain::{BulkOperationKind, SanitizedUserRow};

/// Returns a JS string with `buildFields(csrf, row)` — the form fields for an
/// admin operation (create/edit/delete), OSE.
///
/// email = `row.username + '@' + row.domain` (in JS). The form field is named
/// `mail` (the iRedAdmin contract). The password is taken from `row.password`.
#[must_use]
const fn build_fields_js(kind: BulkOperationKind) -> &'static str {
    match kind {
        BulkOperationKind::Create => {
            r"
            function buildFields(csrf, row) {
                const mail = row.username + '@' + row.domain;
                return [
                    ['csrf_token', csrf],
                    ['mail', mail],
                    ['newpw', row.password],
                    ['confirmpw', row.password],
                    ['submit_add_admin', 'Add']
                ];
            }
        "
        }
        BulkOperationKind::Edit => {
            // Edit: mail identifies the target; display_name → cn.
            r"
            function buildFields(csrf, row) {
                const mail = row.username + '@' + row.domain;
                return [
                    ['csrf_token', csrf],
                    ['mail', mail],
                    ['cn', row.display_name],
                    ['accountStatus', 'active']
                ];
            }
        "
        }
        BulkOperationKind::Delete => {
            // Deletion via the admin list page (bulk action).
            r"
            function buildFields(csrf, row) {
                const mail = row.username + '@' + row.domain;
                return [
                    ['csrf_token', csrf],
                    ['mail', mail],
                    ['action', 'delete'],
                    ['submit_admins', 'Delete']
                ];
            }
        "
        }
    }
}

/// The endpoint and `formActionSuffix` for an admin operation by kind.
const fn endpoints_for(kind: BulkOperationKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        BulkOperationKind::Create => (
            // formUrl: POST to the admin-creation page (email NOT in the path).
            r"base + '/create/admin'",
            r"'/create/admin'",
            "'create'",
        ),
        BulkOperationKind::Edit => (
            // formUrl: admin profile (email in the path).
            r"base + '/profile/admin/general/' + encodeURIComponent(row.username + '@' + row.domain)",
            r"'/profile/admin/general/'",
            "'edit'",
        ),
        BulkOperationKind::Delete => (
            // formUrl: admin list page (bulk action).
            r"base + '/admins'",
            r"'/admins'",
            "'delete'",
        ),
    }
}

/// Builds the JS function `doOp` for a single row of an admin operation (OSE),
/// delegating the unified pipeline to [`super::shared::build_do_op_js`]. The
/// endpoints and form fields are admin-specific; the post-verification target is
/// the email (`username@domain`, computed in JS).
fn build_admin_do_op_js(kind: BulkOperationKind) -> String {
    let (path_fn, form_action_suffix, kind_js) = endpoints_for(kind);
    super::shared::build_do_op_js(&super::shared::DoOpSpec {
        path_fn,
        form_action_suffix,
        kind_js,
        // Admin email = username@domain — the profile post-verification target.
        verify_target_js: "row.username + '@' + row.domain",
        log_tag: "OP-ADMIN",
        log_id_js: "row.username + '@' + row.domain",
    })
}

/// Builds the JS code of an admin operation batch (OSE forms).
///
/// Delegates the shared skeleton to [`super::shared::build_target_batch_js`]; the
/// admin target differs only by the profile segment `/profile/admin/general/` and
/// the CSRF log tag. Before deduplication this skeleton was copied in each target
/// module. The result is posted over IPC.
pub(super) fn build_admin_batch_js(
    id: u64,
    kind: BulkOperationKind,
    base_url: &str,
    rows: &[SanitizedUserRow],
    verify: bool,
) -> String {
    let build_fields = build_fields_js(kind);
    let do_op = build_admin_do_op_js(kind);
    super::shared::build_target_batch_js(
        id,
        base_url,
        rows,
        verify,
        &super::shared::TargetBatchSpec {
            csrf_log_tag: "CSRF-ADMIN",
            profile_segment: "/profile/admin/general/",
            build_fields,
            do_op: &do_op,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_endpoints_for_all_kinds() {
        let (c, _, _) = endpoints_for(BulkOperationKind::Create);
        assert!(c.contains("/create/admin"));
        let (e, _, _) = endpoints_for(BulkOperationKind::Edit);
        assert!(e.contains("/profile/admin/general/"));
        let (d, _, _) = endpoints_for(BulkOperationKind::Delete);
        assert!(d.contains("/admins"));
    }

    #[test]
    fn admin_build_fields_has_mail_and_submit() {
        assert!(build_fields_js(BulkOperationKind::Create).contains("'mail'"));
        assert!(build_fields_js(BulkOperationKind::Create).contains("submit_add_admin"));
        assert!(build_fields_js(BulkOperationKind::Delete).contains("submit_admins"));
        assert!(build_fields_js(BulkOperationKind::Delete).contains("'action', 'delete'"));
    }
}
