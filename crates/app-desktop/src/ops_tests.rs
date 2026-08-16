// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use super::*;
use mailgrit_core_domain::EditableUserRow;

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

// Regression: a plain contains("401") also matched numbers that merely
// EMBED the code — "quota 4012 exceeded" and "id 14012" are not auth errors.
#[test]
fn reason_does_not_match_embedded_status_digits() {
    assert!(!is_session_expired_reason("quota 4012 exceeded"));
    assert!(!is_session_expired_reason("id 14012 not found"));
    assert!(!is_session_expired_reason("error 4030"));
    // …while real codes at token boundaries still match.
    assert!(is_session_expired_reason("HTTP 403"));
    assert!(is_session_expired_reason("status=401 unauthorized"));
}

#[test]
fn reason_login_path() {
    assert!(is_session_expired_reason("redirected to /login"));
}

// Regression: hosts whose NAME starts with `login` must not be mistaken for a
// /login redirect — `contains("/login")` matched the `//login` of the authority
// and silently reset the session (wiping the loaded CSV) whenever a deployment
// sat on e.g. login.mail.example.com.
#[test]
fn reason_login_hostname_is_not_a_path() {
    assert!(!is_session_expired_reason(
        "connect ECONNREFUSED https://login.example.com/iredadmin"
    ));
    assert!(!is_session_expired_reason("host login.example.com is down"));
    assert!(!is_session_expired_reason("path /login2 blocked"));
    // …while real path mentions at segment boundaries still match.
    assert!(is_session_expired_reason(
        "redirected to /login?msg=LOGIN_REQUIRED"
    ));
    assert!(is_session_expired_reason(
        "final url https://x/iredadmin/login"
    ));
}

// The URL-based detector: same hostname-vs-path distinction, on the parsed URLs.
#[test]
fn session_not_expired_login_hostname() {
    // An all-rows-fail batch on a login.* host with a real server error —
    // previously misclassified as session expiry (session reset + CSV wipe).
    assert!(!is_session_expired(
        200,
        &err("Account already exists"),
        Some("https://login.example.com/iredadmin/users/d?msg=ALREADY_EXISTS"),
        None
    ));
}

#[test]
fn session_expired_login_path_on_login_hostname() {
    // A REAL login redirect on a login.* host still matches (the path segment).
    assert!(is_session_expired(
        200,
        &err("HTTP 200"),
        Some("https://login.example.com/iredadmin/login"),
        None
    ));
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

// --- credential_snapshot_for / successful_credentials (pure helpers) ---

fn sanitized_fixture() -> Result<mailgrit_core_domain::SanitizedUserRow, Box<dyn std::error::Error>>
{
    Ok(mailgrit_core_domain::RawCsvRow::new(vec![
        "example.com".into(),
        "ivan.petrov".into(),
        "S3cur3P@ss1".into(),
        "Ivan Petrov".into(),
        "1024".into(),
    ])
    .parse()?)
}

/// Only Create produces a credential snapshot (passwords for the export);
/// Edit/Delete create no accounts, so their snapshot must be empty.
#[test]
fn credential_snapshot_only_for_create() -> Result<(), Box<dyn std::error::Error>> {
    let rows = vec![sanitized_fixture()?, sanitized_fixture()?];
    let create = credential_snapshot_for(BulkOperationKind::Create, &rows);
    assert_eq!(create.len(), 2, "create snapshots every row");
    let first = create.first().ok_or("first snapshot row")?;
    assert_eq!(first.domain, "example.com");
    assert_eq!(first.username, "ivan.petrov");
    assert_eq!(first.password, "S3cur3P@ss1");
    assert_eq!(first.display_name, "Ivan Petrov");
    assert_eq!(first.quota_mb, 1024);
    assert!(
        credential_snapshot_for(BulkOperationKind::Edit, &rows).is_empty(),
        "edit creates no accounts"
    );
    assert!(
        credential_snapshot_for(BulkOperationKind::Delete, &rows).is_empty(),
        "delete creates no accounts"
    );
    Ok(())
}

/// The snapshot is filtered by per-row outcome (results arrive in row order).
#[test]
fn successful_credentials_keeps_only_successful_rows() -> Result<(), Box<dyn std::error::Error>> {
    let snapshot = vec![
        CredentialRow {
            domain: "example.com".into(),
            username: "ok.user".into(),
            password: "Pw1!aaaa".into(),
            display_name: "Ok".into(),
            quota_mb: 512,
        },
        CredentialRow {
            domain: "example.com".into(),
            username: "bad.user".into(),
            password: "Pw2!bbbb".into(),
            display_name: "Bad".into(),
            quota_mb: 512,
        },
    ];
    let results = vec![
        crate::webview_ops::OpResult {
            username: "ok.user".into(),
            domain: "example.com".into(),
            outcome: Ok(()),
            status: 200,
            resp_url: None,
            verify_url: None,
        },
        crate::webview_ops::OpResult {
            username: "bad.user".into(),
            domain: "example.com".into(),
            outcome: Err("NO_SUCH_DOMAIN".into()),
            status: 200,
            resp_url: None,
            verify_url: None,
        },
    ];
    let kept = successful_credentials(&snapshot, &results);
    assert_eq!(kept.len(), 1, "only the successful row is kept");
    let first = kept.first().ok_or("the kept credential")?;
    assert_eq!(first.username, "ok.user");
    assert_eq!(first.password, "Pw1!aaaa");
    Ok(())
}

// --- format_diag / summarize_forms (pure JSON summary builders) ---

#[test]
fn format_diag_summarizes_status_url_and_forms() {
    let json = r#"{
        "status": 200,
        "url": "https://mail.example.com/create/user",
        "forms_in_response": [
            {"action": "/iredadmin/create/user", "inputs": [{"name": "domain"}, {"name": "username"}]}
        ]
    }"#;
    let summary = format_diag(json);
    assert!(summary.contains("200"), "status: {summary}");
    assert!(
        summary.contains("https://mail.example.com/create/user"),
        "url: {summary}"
    );
    assert!(
        summary.contains("action=/iredadmin/create/user"),
        "form action: {summary}"
    );
    assert!(
        summary.contains("fields=[domain, username]"),
        "field names in order: {summary}"
    );
}

#[test]
fn format_diag_without_forms_says_so() {
    let json = r#"{"status": 404, "url": "https://x/notfound"}"#;
    let summary = format_diag(json);
    assert!(summary.contains("404"), "status: {summary}");
    // No forms_in_response array -> the localized no-forms text (non-empty).
    assert!(
        !summary.contains("action="),
        "no form actions invented: {summary}"
    );
}

#[test]
fn summarize_forms_lists_each_form() -> Result<(), Box<dyn std::error::Error>> {
    let v: serde_json::Value = serde_json::from_str(
        r#"{"forms_in_response": [
            {"action": "/a", "inputs": [{"name": "x"}]},
            {"action": "/b", "inputs": []}
        ]}"#,
    )?;
    let s = summarize_forms(&v);
    assert!(s.contains("action=/a"), "first form: {s}");
    assert!(s.contains("action=/b"), "second form: {s}");
    assert!(s.contains("fields=[x]"), "fields of the first form: {s}");
    Ok(())
}

// --- Signal-coupled handlers (Dioxus runtime harness) ---
//
// launch_op / validate_and_collect / apply_op_results / run_diag operate on a
// `Signal<AppState>`, which needs a Dioxus runtime + scope. A minimal
// VirtualDom provides both; the async continuations (`spawn`) are never polled
// by the harness, so the synchronous, observable state transitions are tested
// in isolation from the webview/JS layer.

/// Runs `body` with a fresh `Signal<AppState>` inside a minimal `Dioxus`
/// runtime (a leaked `VirtualDom` whose ROOT scope owns the signal). The signal
/// must be created, read, and written inside the same runtime/scope context,
/// so both the setup and the code under test run within `in_scope`.
fn with_app_state<O>(
    mut setup: impl FnMut(&mut AppState),
    body: impl FnOnce(&mut Signal<AppState>) -> O,
) -> O {
    let vdom = Box::leak(Box::new(VirtualDom::new(|| rsx! {})));
    let runtime = vdom.runtime();
    runtime.in_scope(ScopeId::ROOT, || {
        let mut sig = Signal::new(AppState::default());
        setup(&mut sig.write());
        body(&mut sig)
    })
}

/// A valid editable row (passes the typestate re-validation).
fn valid_editable_row() -> EditableUserRow {
    EditableUserRow {
        domain: "example.com".into(),
        username: "ivan.petrov".into(),
        password: "S3cur3P@ss1".into(),
        display_name: "Ivan Petrov".into(),
        quota: "1024".into(),
    }
}

#[test]
fn validate_and_collect_happy_path_returns_base_url_and_rows()
-> Result<(), Box<dyn std::error::Error>> {
    with_app_state(
        |s| {
            s.session_ok = true;
            s.base_url = "https://mail.example.com".into();
            s.csv.editable_rows = Some(vec![valid_editable_row()]);
        },
        |sig| -> Result<(), Box<dyn std::error::Error>> {
            let (base_url, rows) =
                validate_and_collect(sig).ok_or("a valid batch must pass pre-flight")?;
            assert_eq!(base_url, "https://mail.example.com");
            assert_eq!(rows.len(), 1);
            let sanitized = rows.first().ok_or("the sanitized row")?;
            assert_eq!(sanitized.username.as_str(), "ivan.petrov");
            Ok(())
        },
    )?;
    Ok(())
}

#[test]
fn validate_and_collect_requires_a_session() {
    with_app_state(
        |s| {
            s.session_ok = false;
            s.csv.editable_rows = Some(vec![valid_editable_row()]);
        },
        |sig| {
            assert!(
                validate_and_collect(sig).is_none(),
                "no session -> the operation must not start"
            );
            assert!(
                sig.read().error_msg.is_some(),
                "the refusal reason must be surfaced to the user"
            );
        },
    );
}

#[test]
fn validate_and_collect_requires_rows() {
    with_app_state(
        |s| {
            s.session_ok = true;
            s.csv.editable_rows = None;
        },
        |sig| {
            assert!(
                validate_and_collect(sig).is_none(),
                "no rows -> the operation must not start"
            );
            assert!(sig.read().error_msg.is_some());
        },
    );
}

#[test]
fn launch_op_sets_running_status_on_a_valid_batch() {
    with_app_state(
        |s| {
            s.session_ok = true;
            s.base_url = "https://mail.example.com".into();
            s.csv.editable_rows = Some(vec![valid_editable_row()]);
        },
        |sig| {
            // This test thread's login-state is fresh, so request_op accepts.
            launch_op(sig, OperationTarget::User, BulkOperationKind::Create);
            let read = sig.read();
            assert_eq!(read.op_status, OpStatus::Running, "the batch must start");
            assert!(read.error_msg.is_none(), "no error on a clean start");
        },
    );
}

#[test]
fn launch_op_without_session_sets_error_and_stays_idle() {
    with_app_state(
        |s| {
            s.session_ok = false;
            s.csv.editable_rows = Some(vec![valid_editable_row()]);
        },
        |sig| {
            launch_op(sig, OperationTarget::User, BulkOperationKind::Create);
            let read = sig.read();
            assert_eq!(read.op_status, OpStatus::Idle, "nothing must start");
            assert!(read.error_msg.is_some(), "the user must see why");
        },
    );
}

#[test]
fn launch_op_with_an_already_running_batch_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // Pre-occupy this thread's login-state with a pending operation so that
    // request_op rejects the second one.
    let pending = crate::login_window::login_state();
    let rx = pending
        .request_op(
            OperationTarget::User,
            BulkOperationKind::Create,
            "https://mail.example.com".into(),
            vec![sanitized_fixture()?],
        )
        .ok_or("first request registers")?;
    std::mem::forget(rx); // keep the request pending for the whole test

    with_app_state(
        |s| {
            s.session_ok = true;
            s.base_url = "https://mail.example.com".into();
            s.csv.editable_rows = Some(vec![valid_editable_row()]);
        },
        |sig| {
            launch_op(sig, OperationTarget::User, BulkOperationKind::Create);
            let read = sig.read();
            assert_eq!(
                read.op_status,
                OpStatus::Idle,
                "a duplicate batch must never flip the status to Running"
            );
            assert!(read.error_msg.is_some(), "the rejection must be visible");
        },
    );
    Ok(())
}

fn op_result(
    username: &str,
    outcome: Result<(), String>,
    status: i64,
) -> crate::webview_ops::OpResult {
    crate::webview_ops::OpResult {
        username: username.into(),
        domain: "example.com".into(),
        outcome,
        status,
        resp_url: None,
        verify_url: None,
    }
}

#[test]
fn apply_op_results_reports_mixed_outcomes() -> Result<(), Box<dyn std::error::Error>> {
    let snapshot = vec![CredentialRow {
        domain: "example.com".into(),
        username: "ivan.petrov".into(),
        password: "S3cur3P@ss1".into(),
        display_name: "Ivan Petrov".into(),
        quota_mb: 1024,
    }];
    let results = vec![
        op_result("ivan.petrov", Ok(()), 200),
        op_result("maria", Err("ALREADY_EXISTS".into()), 200),
    ];
    with_app_state(
        |s| {
            s.session_ok = true;
            s.op_status = OpStatus::Running;
            s.error_msg = None;
        },
        |sig| -> Result<(), Box<dyn std::error::Error>> {
            apply_op_results(
                sig,
                OperationTarget::User,
                BulkOperationKind::Create,
                2,
                &snapshot,
                &results,
            );
            let read = sig.read();
            assert_eq!(read.op_status, OpStatus::Idle, "the batch is finished");
            assert!(read.error_msg.is_none(), "a mixed batch is not an error");
            let result = read.csv.batch_result.as_ref().ok_or("result stored")?;
            assert_eq!(result.succeeded, 1);
            assert_eq!(result.failed, 1);
            assert_eq!(result.failures.len(), 1);
            let failure0 = result.failures.first().ok_or("the failure row")?;
            assert_eq!(failure0.username, "maria");
            assert_eq!(
                result.created_credentials.len(),
                1,
                "only the successful row's password is exported"
            );
            let cred0 = result
                .created_credentials
                .first()
                .ok_or("the created credential")?;
            assert_eq!(cred0.username, "ivan.petrov");
            assert!(read.session_ok, "a server rejection is not session loss");
            Ok(())
        },
    )?;
    Ok(())
}

#[test]
fn apply_op_results_all_401_resets_the_session() {
    let results = vec![
        op_result("ivan.petrov", Err("HTTP 401".into()), 401),
        op_result("maria", Err("HTTP 401".into()), 401),
    ];
    with_app_state(
        |s| {
            s.session_ok = true;
            s.op_status = OpStatus::Running;
            s.error_msg = None;
        },
        |sig| {
            apply_op_results(
                sig,
                OperationTarget::User,
                BulkOperationKind::Create,
                2,
                &[],
                &results,
            );
            let read = sig.read();
            assert!(!read.session_ok, "all-401 means the session is gone");
            assert!(read.error_msg.is_some(), "the user must be told why");
            assert!(
                read.csv.batch_result.is_none(),
                "a session loss is not a normal batch result"
            );
        },
    );
}

#[test]
fn apply_op_results_empty_on_nonempty_batch_is_webview_failure() {
    with_app_state(
        |s| {
            s.op_status = OpStatus::Running;
            s.error_msg = None;
        },
        |sig| {
            apply_op_results(
                sig,
                OperationTarget::User,
                BulkOperationKind::Create,
                2,
                &[],
                &[],
            );
            let read = sig.read();
            assert_eq!(read.op_status, OpStatus::Idle);
            assert!(
                read.error_msg.is_some(),
                "0 results for 2 rows is a webview failure, not success"
            );
            assert!(read.csv.batch_result.is_none());
        },
    );
}

#[test]
fn apply_op_results_empty_on_empty_batch_is_not_failure() {
    // expected == 0 with an empty result list must NOT trigger the
    // webview-failure guard nor a session reset.
    with_app_state(
        |s| {
            s.session_ok = true;
            s.op_status = OpStatus::Running;
            s.error_msg = None;
        },
        |sig| {
            apply_op_results(
                sig,
                OperationTarget::User,
                BulkOperationKind::Create,
                0,
                &[],
                &[],
            );
            let read = sig.read();
            assert_eq!(read.op_status, OpStatus::Idle);
            assert!(read.error_msg.is_none(), "0/0 is not an error");
            assert!(
                read.csv.batch_result.is_some(),
                "the (empty) result is still recorded"
            );
            assert!(read.session_ok, "no session was ever lost here");
        },
    );
}

#[test]
fn run_diag_requires_a_csv_row() {
    with_app_state(
        |s| {
            s.csv.editable_rows = None;
        },
        |sig| {
            run_diag(sig);
            let read = sig.read();
            assert_eq!(read.op_status, OpStatus::Idle, "diag must not start");
            assert!(
                read.error_msg.is_some(),
                "the user must be told to load a CSV"
            );
        },
    );
}

#[test]
fn run_diag_ignores_a_whitespace_domain_row() {
    with_app_state(
        |s| {
            s.csv.editable_rows = Some(vec![EditableUserRow {
                domain: "   ".into(),
                username: "ivan.petrov".into(),
                password: "S3cur3P@ss1".into(),
                display_name: "Ivan Petrov".into(),
                quota: "1024".into(),
            }]);
        },
        |sig| {
            run_diag(sig);
            let read = sig.read();
            assert_eq!(
                read.op_status,
                OpStatus::Idle,
                "a whitespace-only domain must not start diagnostics"
            );
            assert!(read.error_msg.is_some());
        },
    );
}

#[test]
fn run_diag_sets_running_on_a_valid_domain() {
    with_app_state(
        |s| {
            s.csv.editable_rows = Some(vec![valid_editable_row()]);
        },
        |sig| {
            run_diag(sig);
            let read = sig.read();
            assert_eq!(read.op_status, OpStatus::Running, "diag must start");
            assert!(read.error_msg.is_some(), "progress note is expected");
        },
    );
}
