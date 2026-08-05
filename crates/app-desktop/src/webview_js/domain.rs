//! Generation of the JS code for a bulk **domain** (Domain) operation executed
//! inside the login-WebView2.
//!
//! This is the registry branch for `OperationTarget::Domain` (see
//! [`super::build_batch_js`]). It implements the iRedAdmin OSE forms
//! (create/edit/delete domains).
//!
//! # OSE endpoints (iRedAdmin open-source, `controllers/sql/domain.py`)
//! - **Create**: `POST /create/domain/<domain>` (domain in the path). Form fields:
//!   `csrf_token, domainName, quota, transport, is_backupmx, submit_add_domain`.
//! - **Edit**: `POST /profile/domain/general/<domain>`. Same fields.
//! - **Delete**: the bulk action on `/domains`:
//!   `csrf_token, domain=<domain>, action=delete, submit_domains=Delete`.
//!
//! # Success verdict
//! The universal iRedAdmin markers from [`crate::webview_markers`] are reused
//! (`?msg=CREATED/UPDATED/DELETED`, `note-error/note-success`) — they are the same
//! for all iRedAdmin HTML forms, so there is no duplication.

use mailgrit_core_domain::{BulkOperationKind, SanitizedUserRow};

/// Returns a JS string defining `buildFields(csrf, row)` — the form fields for a
/// domain operation (create/edit/delete), OSE. Factored into a separate
/// `const fn` for the compactness of `build_domain_batch_js`.
///
/// **Data note**: `SanitizedUserRow` is reused as the carrier (spec §2.2). For a
/// domain, only `row.domain` (the name) and `row.quota.mb()` (the quota in MiB)
/// are meaningful. `transport`/`is_backupmx` are fixed literals (the extended
/// profile fields never reach the row; see the `for_domain_create` profile).
#[must_use]
const fn build_fields_js(kind: BulkOperationKind) -> &'static str {
    match kind {
        BulkOperationKind::Create | BulkOperationKind::Edit => {
            // Create and edit of a domain go to different endpoints, but with
            // the same set of form fields (domainName, quota, transport, ...).
            r"
            function buildFields(csrf, row) {
                return [
                    ['csrf_token', csrf],
                    ['domainName', row.domain],
                    ['quota', String(row.quota)],
                    ['transport', 'dovecot'],
                    ['is_backupmx', '0'],
                    ['submit_add_domain', 'Add']
                ];
            }
        "
        }
        BulkOperationKind::Delete => {
            // Deletion via the domain list page (bulk action):
            // checkbox (domain=<domain>) + action=delete + submit_domains=...
            r"
            function buildFields(csrf, row) {
                return [
                    ['csrf_token', csrf],
                    ['domain', row.domain],
                    ['action', 'delete'],
                    ['submit_domains', 'Delete']
                ];
            }
        "
        }
    }
}

/// The endpoint and `formActionSuffix` for a domain operation by kind.
/// Returns JS expressions for interpolation into doOp.
const fn endpoints_for(kind: BulkOperationKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        BulkOperationKind::Create => (
            // formUrl: POST to the domain-creation page (domain in the path).
            r"base + '/create/domain/' + encodeURIComponent(row.domain)",
            r"'/create/domain/'",
            "'create'",
        ),
        BulkOperationKind::Edit => (
            // formUrl: domain profile (edit).
            r"base + '/profile/domain/general/' + encodeURIComponent(row.domain)",
            r"'/profile/domain/general/'",
            "'edit'",
        ),
        BulkOperationKind::Delete => (
            // formUrl: domain list page (bulk action).
            r"base + '/domains'",
            r"'/domains'",
            "'delete'",
        ),
    }
}

/// Builds the JS function `doOp` for a single row of a domain operation (OSE
/// forms), delegating the unified pipeline to [`super::shared::build_do_op_js`].
/// The endpoints and form fields are domain-specific; the rest of the pipeline is
/// identical to user/admin.
fn build_domain_do_op_js(kind: BulkOperationKind) -> String {
    let (path_fn, form_action_suffix, kind_js) = endpoints_for(kind);
    super::shared::build_do_op_js(&super::shared::DoOpSpec {
        path_fn,
        form_action_suffix,
        kind_js,
        // A domain identifies itself — used as the post-verification target.
        verify_target_js: "row.domain",
        log_tag: "OP-DOMAIN",
        log_id_js: "row.domain",
    })
}

/// JS fragment: the mfMask/getCsrf/verifyOp helper functions for a domain.
/// Builds the JS code of a domain operation batch (OSE forms).
///
/// Delegates the shared skeleton to [`super::shared::build_target_batch_js`]; the
/// domain target differs only by the profile segment `/profile/domain/general/`
/// and the CSRF log tag. Before deduplication this skeleton was copied in each
/// target module.
pub(super) fn build_domain_batch_js(
    id: u64,
    kind: BulkOperationKind,
    base_url: &str,
    rows: &[SanitizedUserRow],
    verify: bool,
) -> String {
    let build_fields = build_fields_js(kind);
    let do_op = build_domain_do_op_js(kind);
    super::shared::build_target_batch_js(
        id,
        base_url,
        rows,
        verify,
        &super::shared::TargetBatchSpec {
            csrf_log_tag: "CSRF-DOMAIN",
            profile_segment: "/profile/domain/general/",
            build_fields,
            do_op: &do_op,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Endpoints: create → /create/domain/, edit → /profile/domain/general/,
    // delete → /domains.
    #[test]
    fn domain_endpoints_for_all_kinds() {
        let (c, _, _) = endpoints_for(BulkOperationKind::Create);
        assert!(c.contains("/create/domain/"));
        let (e, _, _) = endpoints_for(BulkOperationKind::Edit);
        assert!(e.contains("/profile/domain/general/"));
        let (d, _, _) = endpoints_for(BulkOperationKind::Delete);
        assert!(d.contains("/domains"));
    }

    // buildFields create/delete contain the required field names.
    #[test]
    fn domain_build_fields_has_correct_form_names() {
        assert!(build_fields_js(BulkOperationKind::Create).contains("domainName"));
        assert!(build_fields_js(BulkOperationKind::Create).contains("submit_add_domain"));
        assert!(build_fields_js(BulkOperationKind::Edit).contains("quota"));
        assert!(build_fields_js(BulkOperationKind::Delete).contains("submit_domains"));
        assert!(build_fields_js(BulkOperationKind::Delete).contains("'action', 'delete'"));
    }
}
