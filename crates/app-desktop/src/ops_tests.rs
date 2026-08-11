// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use super::*;

/// Helper for an error result mirroring the real bulk-operation result type.
fn err(reason: &str) -> Result<(), String> {
    Err(reason.to_string())
}

// A successful result mirrors the real bulk-operation type `Result<(), String>`.
// Written inline as `&Ok::<(), String>(())` at call sites (clippy::unnecessary_wraps
// flags a helper that always returns Ok, so the helper is avoided).

// --- P0: is_session_expired by objective signals ---

#[test]
fn session_expired_http_401() {
    assert!(is_session_expired(401, &err("HTTP 401"), None, None));
}

#[test]
fn session_expired_http_403() {
    assert!(is_session_expired(403, &err("Forbidden"), None, None));
}

#[test]
fn session_expired_login_redirect_in_url() {
    // After session expiry, iRedAdmin redirects to /login; the status stays 200,
    // and the reason is "HTTP 200". The detector must trigger by the URL.
    assert!(is_session_expired(
        200,
        &err("HTTP 200"),
        Some("https://x/iredadmin/login"),
        None
    ));
}

#[test]
fn session_expired_login_redirect_in_verify_url() {
    // P0 fix for the blind spot: the session expired BETWEEN a successful POST
    // and the verify-GET. The POST-url is clean (?msg=CREATED), but the
    // verify-GET went to /login. Previously the detector checked only the
    // POST-url and missed this case.
    assert!(is_session_expired(
        200,
        &err("profile not found after create"),
        Some("https://x/iredadmin/profile/user/general/u@d?msg=CREATED"),
        Some("https://x/iredadmin/login?msg=LOGIN_REQUIRED"),
    ));
}

#[test]
fn session_not_expired_verify_url_clean() {
    // The verify-GET reached the profile page — the session is alive.
    assert!(!is_session_expired(
        200,
        &Ok::<(), String>(()),
        Some("https://x/?msg=CREATED"),
        Some("https://x/iredadmin/profile/user/general/u@d"),
    ));
}

#[test]
fn session_not_expired_real_server_error() {
    // A real server error: the account does not exist. Status 200, URL without
    // /login. NOT session expiry.
    assert!(!is_session_expired(
        200,
        &err("Account does not exist"),
        Some("https://x/iredadmin/users/d?msg=NO_SUCH_ACCOUNT"),
        None
    ));
}

#[test]
fn session_not_expired_successful_op() {
    assert!(!is_session_expired(
        200,
        &Ok::<(), String>(()),
        Some("https://x/?msg=CREATED"),
        None
    ));
}

#[test]
fn session_expired_csrf_token_missing() {
    // Fallback by text: CSRF is missing → not logged in.
    assert!(is_session_expired(
        0,
        &err("CSRF token not found at ..."),
        None,
        None
    ));
}

#[test]
fn session_expired_reason_401_in_text() {
    // The status may be in the error body.
    assert!(is_session_expired(
        0,
        &err("HTTP 401: Unauthorized"),
        None,
        None
    ));
}

#[test]
fn session_expired_network_failure_status_0() {
    // A network failure (status 0, no URL) is NOT counted as session expiry.
    assert!(!is_session_expired(0, &err("Failed to fetch"), None, None));
}

// --- is_session_expired_reason (text fallback for export) ---

#[test]
fn reason_401() {
    assert!(is_session_expired_reason("HTTP 401"));
}

#[test]
fn reason_login_path() {
    assert!(is_session_expired_reason("redirected to /login"));
}

#[test]
fn reason_clean_error() {
    assert!(!is_session_expired_reason("Domain does not exist"));
}

// --- audit_action_for: User/Domain/Admin ---

#[test]
fn audit_action_user_mappings() {
    assert_eq!(
        audit_action_for(OperationTarget::User, BulkOperationKind::Create),
        AuditAction::CreateUser
    );
    assert_eq!(
        audit_action_for(OperationTarget::User, BulkOperationKind::Edit),
        AuditAction::EditUser
    );
    assert_eq!(
        audit_action_for(OperationTarget::User, BulkOperationKind::Delete),
        AuditAction::DeleteUser
    );
}

#[test]
fn audit_action_domain_mappings() {
    assert_eq!(
        audit_action_for(OperationTarget::Domain, BulkOperationKind::Create),
        AuditAction::CreateDomain
    );
    assert_eq!(
        audit_action_for(OperationTarget::Domain, BulkOperationKind::Edit),
        AuditAction::EditDomain
    );
    assert_eq!(
        audit_action_for(OperationTarget::Domain, BulkOperationKind::Delete),
        AuditAction::DeleteDomain
    );
}

#[test]
fn audit_action_admin_mappings() {
    assert_eq!(
        audit_action_for(OperationTarget::Admin, BulkOperationKind::Create),
        AuditAction::CreateAdmin
    );
    assert_eq!(
        audit_action_for(OperationTarget::Admin, BulkOperationKind::Edit),
        AuditAction::EditAdmin
    );
    assert_eq!(
        audit_action_for(OperationTarget::Admin, BulkOperationKind::Delete),
        AuditAction::DeleteAdmin
    );
}
