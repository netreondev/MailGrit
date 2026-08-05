use super::*;

// --- has_ose_success -------------------------------------------------

#[test]
fn success_url_created() {
    assert!(has_ose_success(
        "https://x/iredadmin/profile/user/general/u@d?msg=CREATED",
        ""
    ));
}

#[test]
fn success_url_updated() {
    assert!(has_ose_success("https://x/?msg=UPDATED", ""));
}

#[test]
fn success_url_deleted() {
    assert!(has_ose_success("https://x/users/d/page/1?msg=DELETED", ""));
}

#[test]
fn success_body_note_success() {
    assert!(has_ose_success("", "User created note-success"));
}

#[test]
fn success_absent_when_no_marker() {
    // Defect D1: previously HTTP 200 without a marker was treated as success.
    assert!(!has_ose_success(
        "https://x/profile/user/general/u@d",
        "Profile"
    ));
}

// --- has_ose_error ---------------------------------------------------

#[test]
fn error_note_error() {
    assert!(has_ose_error(
        "",
        "<div class='note-error'>Account already exists</div>"
    ));
}

#[test]
fn error_note_warning_d2_extension() {
    // Defect D2: note-warning was previously not recognized.
    assert!(has_ose_error(
        "",
        "<div class='note-warning'>Quota exceeded</div>"
    ));
}

#[test]
fn error_note_danger_d2_extension() {
    assert!(has_ose_error("", "class='note-danger'"));
}

#[test]
fn error_already_exists() {
    assert!(has_ose_error("", "ALREADY_EXISTS"));
}

#[test]
fn error_account_exists_d2_extension() {
    assert!(has_ose_error("", "ACCOUNT_EXISTS"));
}

#[test]
fn error_ldap_signature() {
    assert!(has_ose_error("", "LDAP: constraint violation"));
}

#[test]
fn error_msg_in_url() {
    assert!(has_ose_error("https://x/?msg=ERROR", ""));
}

#[test]
fn error_absent_on_clean_success() {
    assert!(!has_ose_error("?msg=CREATED", "User created note-success"));
}

// --- ose_final_ok: composition (D1 — critical fix) -------------------

#[test]
fn final_ok_happy_path_created() {
    assert!(ose_final_ok(
        true,
        "https://x/?msg=CREATED",
        "note-success User created"
    ));
}

#[test]
fn final_ok_happy_path_updated() {
    assert!(ose_final_ok(
        true,
        "https://x/?msg=UPDATED",
        "Profile has been updated."
    ));
}

#[test]
fn final_ok_happy_path_deleted() {
    assert!(ose_final_ok(
        true,
        "https://x/users/d/page/1?msg=DELETED",
        "Selected accounts were deleted."
    ));
}

#[test]
fn final_ok_false_when_http_error() {
    assert!(!ose_final_ok(false, "?msg=CREATED", "note-success"));
}

#[test]
fn final_ok_false_when_no_success_marker_d1() {
    // Main D1 regression case: HTTP 200 + no marker -> NOT a success.
    assert!(!ose_final_ok(
        true,
        "https://x/profile/user/general/u@d",
        "Profile"
    ));
}

#[test]
fn final_ok_false_when_error_marker_present() {
    // An error outweighs a success marker: note-error with ?msg=CREATED.
    assert!(!ose_final_ok(
        true,
        "?msg=CREATED",
        "note-error Account exists"
    ));
}

// --- build_marker_js: structure --------------------------------------

#[test]
fn marker_js_contains_all_arrays() {
    let js = build_marker_js();
    assert!(js.contains("MF_SUCCESS_URL"));
    assert!(js.contains("MF_SUCCESS_BODY"));
    assert!(js.contains("MF_ERROR_URL"));
    assert!(js.contains("MF_ERROR_BODY"));
    assert!(js.contains("CREATED"));
    assert!(js.contains("UPDATED"));
    assert!(js.contains("DELETED"));
    assert!(js.contains("note-success"));
    assert!(js.contains("note-warning"));
    assert!(js.contains("ALREADY_EXISTS"));
    assert!(js.contains("mfHasSuccess"));
    assert!(js.contains("mfHasError"));
}

#[test]
fn js_array_quoting() {
    assert_eq!(js_array(&["a", "b"]), "[\"a\", \"b\"]");
    assert_eq!(js_array(&[]), "[]");
}

// --- E2: extract_msg_code / map_error_code --------------------------

#[test]
fn extract_msg_code_no_such_account() {
    assert_eq!(
        extract_msg_code("https://x/users/d?msg=NO_SUCH_ACCOUNT"),
        Some("NO_SUCH_ACCOUNT")
    );
}

#[test]
fn extract_msg_code_with_other_params() {
    // The msg parameter is not first — it is correctly trimmed at '&'.
    assert_eq!(
        extract_msg_code("https://x/?foo=bar&msg=ALREADY_EXISTS&x=1"),
        Some("ALREADY_EXISTS")
    );
}

#[test]
fn extract_msg_code_created() {
    assert_eq!(extract_msg_code("https://x/?msg=CREATED"), Some("CREATED"));
}

#[test]
fn extract_msg_code_absent() {
    assert_eq!(extract_msg_code("https://x/no/msg/here"), None);
}

#[test]
fn extract_msg_code_empty_value() {
    assert_eq!(extract_msg_code("https://x/?msg="), None);
}

#[test]
fn map_error_code_known() {
    // map_error_code is localized — check Ukrainian values at locale=uk.
    rust_i18n::set_locale("uk");
    assert_eq!(
        map_error_code("NO_SUCH_ACCOUNT"),
        Some("Обліковий запис не існує".to_string())
    );
    assert_eq!(
        map_error_code("ALREADY_EXISTS"),
        Some("Обліковий запис уже існує".to_string())
    );
    assert_eq!(
        map_error_code("NOT_ALLOWED"),
        Some("Дію не дозволено".to_string())
    );
    // Restore the global locale to the default.
    rust_i18n::set_locale("en");
}

#[test]
fn map_error_code_unknown_returns_none() {
    // CREATED is a success code and must not map to an error.
    assert_eq!(map_error_code("CREATED"), None);
    assert_eq!(map_error_code("SOME_UNKNOWN_CODE"), None);
}

// E2: known error codes are now also caught from the URL (previously body only).
#[test]
fn error_caught_by_url_code() {
    assert!(has_ose_error("https://x/?msg=NO_SUCH_ACCOUNT", ""));
    assert!(has_ose_error("https://x/?msg=ALREADY_EXISTS_DOMAIN", ""));
    assert!(has_ose_error("https://x/?msg=NOT_ALLOWED", ""));
}

// --- build_error_map_js: structure (E1) ------------------------------

#[test]
fn error_map_js_contains_map_and_extractor() {
    // build_error_map_js localizes values — check Ukrainian at locale=uk.
    rust_i18n::set_locale("uk");
    let js = build_error_map_js();
    assert!(js.contains("MF_ERROR_MAP"), "must contain the map object");
    assert!(
        js.contains("mfExtractMessage"),
        "must contain mfExtractMessage"
    );
    assert!(js.contains("mfExtractMsgCode"));
    // Known codes from ERROR_CODE_MAP.
    assert!(js.contains("NO_SUCH_ACCOUNT"));
    assert!(js.contains("Обліковий запис не існує"));
    assert!(js.contains("ALREADY_EXISTS"));
    assert!(js.contains("NOT_ALLOWED"));
    // Restore the global locale to the default.
    rust_i18n::set_locale("en");
}

// P1a: the regex uses class-contains (\bnote-...\b) to match the double
// iRedAdmin class "notification note-error". The old class=["']note-... did not match.
#[test]
fn p1a_regex_uses_class_contains_for_double_class() {
    let js = build_error_map_js();
    assert!(
        js.contains("\\bnote-(?:error|warning|danger)\\b"),
        "P1a: the regex must use class-contains via \\bnote-...\\b"
    );
}
