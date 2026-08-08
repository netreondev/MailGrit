use super::*;

use mailgrit_core_domain::{
    BulkOperationKind, CsvRowError, OperationTarget, RawCsvRow, SanitizedUserRow,
};

/// A minimal row for JS generation (built via the canonical parser, since
/// SanitizedUserRow cannot be constructed directly — the typestate pipeline).
fn sample_row() -> Result<SanitizedUserRow, CsvRowError> {
    RawCsvRow::new(vec![
        "dnipr.gp.gov.ua".into(),
        "ivan.petrov".into(),
        "S3cur3P@ss1".into(),
        "Ivan Petrov".into(),
        "1024".into(),
    ])
    .parse()
}

// D1/D2: the verdict requires a positive marker and the extended error list.
#[test]
fn create_js_requires_success_marker_and_marks() -> Result<(), Box<dyn std::error::Error>> {
    let js = build_batch_js(
        1,
        OperationTarget::User,
        BulkOperationKind::Create,
        "https://x/iredadmin",
        &[sample_row()?],
        false,
    );
    assert!(js.contains("CREATED"), "must include CREATED");
    assert!(js.contains("UPDATED"));
    assert!(js.contains("DELETED"));
    assert!(js.contains("note-success"));
    assert!(
        js.contains("note-warning"),
        "note-warning must be recognized"
    );
    assert!(js.contains("note-danger"), "note-danger must be recognized");
    assert!(js.contains("mfHasSuccess"), "mfHasSuccess must be called");
    assert!(js.contains("mfHasError"));
    assert!(js.contains("finalOk = ok && hasSuccess && !isErrorMsg"));
    Ok(())
}

// D4: post-verification for create/delete (verify=true), absent for edit.
#[test]
fn d4_verification_present_for_create_and_delete_absent_for_edit()
-> Result<(), Box<dyn std::error::Error>> {
    for (kind, expect_verify) in [
        (BulkOperationKind::Create, true),
        (BulkOperationKind::Delete, true),
        (BulkOperationKind::Edit, false),
    ] {
        let js = build_batch_js(
            7,
            OperationTarget::User,
            kind,
            "https://x/iredadmin",
            &[sample_row()?],
            true,
        );
        if expect_verify {
            assert!(
                js.contains("verifyOp") && js.contains("verified"),
                "D4: verifyOp/verified must be present for {kind:?}"
            );
        } else {
            assert!(js.contains("kind === 'create' || kind === 'delete'"));
        }
    }
    Ok(())
}

// D4: the verify=false flag disables verification.
#[test]
fn d4_verify_flag_false_disables_verification() -> Result<(), Box<dyn std::error::Error>> {
    let js = build_batch_js(
        1,
        OperationTarget::User,
        BulkOperationKind::Delete,
        "https://x/iredadmin",
        &[sample_row()?],
        false,
    );
    assert!(js.contains("const MF_VERIFY = false"));
    Ok(())
}

// D5: PII masking in the dump via mfMask.
#[test]
fn d5_pii_masking_function_present() -> Result<(), Box<dyn std::error::Error>> {
    let js = build_batch_js(
        1,
        OperationTarget::User,
        BulkOperationKind::Create,
        "https://x/iredadmin",
        &[sample_row()?],
        false,
    );
    assert!(js.contains("function mfMask"), "mfMask must be defined");
    assert!(js.contains("mfMask(k, v)"), "requestFields must be masked");
    assert!(js.contains("csrfToken: mfMask('csrf_token', csrf)"));
    Ok(())
}

// E1: the error field uses mfExtractMessage, NOT respBody.slice.
#[test]
fn e1_error_field_uses_extract_message_not_html_slice() -> Result<(), Box<dyn std::error::Error>> {
    let js = build_batch_js(
        1,
        OperationTarget::User,
        BulkOperationKind::Edit,
        "https://x/iredadmin",
        &[sample_row()?],
        false,
    );
    assert!(
        js.contains("mfExtractMessage(respUrl, respBody)"),
        "E1: the error field must use mfExtractMessage"
    );
    assert!(js.contains("MF_ERROR_MAP"));
    assert!(js.contains("NO_SUCH_ACCOUNT"));
    Ok(())
}

// Registry: User×Create/Edit/Delete have correct OSE paths.
#[test]
fn user_paths_for_all_kinds() -> Result<(), Box<dyn std::error::Error>> {
    let js = build_batch_js(
        1,
        OperationTarget::User,
        BulkOperationKind::Create,
        "https://x/iredadmin",
        &[sample_row()?],
        false,
    );
    assert!(js.contains("/create/user/"));
    let js = build_batch_js(
        1,
        OperationTarget::User,
        BulkOperationKind::Edit,
        "https://x/iredadmin",
        &[sample_row()?],
        false,
    );
    assert!(js.contains("/profile/user/general/"));
    let js = build_batch_js(
        1,
        OperationTarget::User,
        BulkOperationKind::Delete,
        "https://x/iredadmin",
        &[sample_row()?],
        false,
    );
    assert!(js.contains("/users/"));
    Ok(())
}

// IIFE structure and brace balance for all kinds.
#[test]
fn generated_js_is_balanced_iife_for_all_kinds() -> Result<(), Box<dyn std::error::Error>> {
    for kind in [
        BulkOperationKind::Create,
        BulkOperationKind::Edit,
        BulkOperationKind::Delete,
    ] {
        let js = build_batch_js(
            99,
            OperationTarget::User,
            kind,
            "https://x/iredadmin",
            &[sample_row()?],
            true,
        );
        assert!(js.starts_with("(async () => {"));
        assert!(js.ends_with(")()"));
        let open = js.matches('{').count();
        let close = js.matches('}').count();
        assert_eq!(open, close, "brace imbalance for {kind:?}");
    }
    Ok(())
}
