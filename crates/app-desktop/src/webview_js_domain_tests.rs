//! Tests for the JS builders of domain/admin/user operations (OSE).
//!
//! After removing the Pro REST API, all builders work only with OSE forms. The
//! tests check form-field interpolation and brace balance.

use crate::webview_js::{admin, domain, user};
use mailgrit_core_domain::BulkOperationKind;

// Universal check: parsing { and } in a string yields a zero balance (all
// interpolated fragments are closed). A coarse guard against brace desync when
// editing templates.
fn braces_balanced(s: &str) -> bool {
    let mut depth = 0i64;
    for ch in s.chars() {
        match ch {
            '{' => depth = depth.saturating_add(1),
            '}' => {
                // A closing brace without a matching opener → unbalanced.
                if depth == 0 {
                    return false;
                }
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    depth == 0
}

#[test]
fn user_create_fields_contain_form_names() {
    let js = user::build_user_batch_js(1, BulkOperationKind::Create, "https://x", &[], false);
    assert!(js.contains("domainName"));
    assert!(js.contains("submit_add_user"));
    assert!(braces_balanced(&js));
}

#[test]
fn domain_create_fields_contain_form_names() {
    let js = domain::build_domain_batch_js(1, BulkOperationKind::Create, "https://x", &[], false);
    assert!(js.contains("domainName"));
    assert!(js.contains("submit_add_domain"));
    assert!(braces_balanced(&js));
}

#[test]
fn domain_delete_uses_domains_page() {
    let js = domain::build_domain_batch_js(1, BulkOperationKind::Delete, "https://x", &[], false);
    assert!(js.contains("/domains"));
    assert!(js.contains("'action', 'delete'"));
    assert!(braces_balanced(&js));
}

#[test]
fn admin_create_uses_create_admin() {
    let js = admin::build_admin_batch_js(1, BulkOperationKind::Create, "https://x", &[], false);
    assert!(js.contains("/create/admin"));
    assert!(js.contains("submit_add_admin"));
    assert!(braces_balanced(&js));
}

// The shared functions (mfMask/getCsrf/verifyOp) are present in every target
// and use that target's own post-verification profile path. Guards against
// regression when editing the unified implementation in webview_js/shared.rs.
#[test]
fn shared_helpers_present_for_all_targets() {
    // mfMask and getCsrf are defined exactly once in each batch.
    for (target, builder_kind) in [
        (
            user::build_user_batch_js(1, BulkOperationKind::Create, "https://x", &[], false),
            "user",
        ),
        (
            domain::build_domain_batch_js(1, BulkOperationKind::Create, "https://x", &[], false),
            "domain",
        ),
        (
            admin::build_admin_batch_js(1, BulkOperationKind::Create, "https://x", &[], false),
            "admin",
        ),
    ] {
        let (js, _kind) = (target, builder_kind);
        assert!(
            js.contains("function mfMask"),
            "mfMask must be present in the batch"
        );
        assert!(
            js.matches("function mfMask").count() == 1,
            "mfMask must be defined exactly once (no duplication)"
        );
        assert!(
            js.contains("async function getCsrf"),
            "getCsrf must be present in the batch"
        );
        assert!(
            js.matches("async function getCsrf").count() == 1,
            "getCsrf must be defined exactly once"
        );
        assert!(
            js.contains("async function verifyOp"),
            "verifyOp must be present in the batch"
        );
    }
}

// The post-verification profile paths differ by target — each target checks its
// own profile (user/domain/admin). This is the key semantics.
#[test]
fn verify_op_profile_paths_differ_per_target() {
    let user_js = user::build_user_batch_js(1, BulkOperationKind::Create, "https://x", &[], true);
    let domain_js =
        domain::build_domain_batch_js(1, BulkOperationKind::Create, "https://x", &[], true);
    let admin_js =
        admin::build_admin_batch_js(1, BulkOperationKind::Create, "https://x", &[], true);

    assert!(user_js.contains("/profile/user/general/"));
    assert!(user_js.contains("profile/user/general/"));
    assert!(domain_js.contains("/profile/domain/general/"));
    assert!(!domain_js.contains("/profile/user/general/"));
    assert!(admin_js.contains("/profile/admin/general/"));
    assert!(!admin_js.contains("/profile/user/general/"));
}

// batch_iife_js: the IPC correlation-id and the loop structure are correct for
// all targets.
#[test]
fn batch_iife_correlation_id_and_ipc_for_all_targets() {
    for (js, label) in [
        (
            user::build_user_batch_js(42, BulkOperationKind::Delete, "https://x", &[], false),
            "user",
        ),
        (
            domain::build_domain_batch_js(42, BulkOperationKind::Delete, "https://x", &[], false),
            "domain",
        ),
        (
            admin::build_admin_batch_js(42, BulkOperationKind::Delete, "https://x", &[], false),
            "admin",
        ),
    ] {
        assert!(
            js.contains("batch:42:"),
            "IPC id must be in the {label} batch"
        );
        assert!(
            js.contains("window.ipc.postMessage"),
            "IPC postMessage in {label}"
        );
        assert!(braces_balanced(&js), "brace balance in {label}");
    }
}
