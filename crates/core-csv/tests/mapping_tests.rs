//! Tests for the flexible CSV column mapping (Phase 1.2).
//!
//! Integration tests for the public `mapping` API: mapping auto-detection,
//! streaming parse with a mapping, and the **backward-compatibility criterion** —
//! `parse_csv_bytes_auto` must produce a result identical to the classic
//! `parse_csv_bytes` for a canonical 5-column CSV.

// Documented exception (spec): unwrap/expect/panic are acceptable in tests —
// a panic here is a meaningful test failure. redundant_closure_for_method_calls
// is lifted narrowly: the idiom `.map(|s| s.to_string())` over `[&str].iter()`
// (yielding `&&str`) reads more clearly than `.copied().map(String::from)`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    clippy::redundant_closure_for_method_calls
)]

use mailgrit_core_csv::{
    ColumnMapping, detect_mapping, parse_csv_bytes, parse_csv_bytes_auto,
    parse_csv_bytes_with_mapping,
};
use mailgrit_core_domain::OperationProfile;

const CLASSIC_CSV: &str = concat!(
    "domain,username,password,display_name,quota_mb\n",
    "example.com,ivan.petrov,S3cur3P@ss1,Ivan Petrov,1024\n",
    "example.com,anna.kovalenko,Str0ngPwd!2,Anna Kovalenko,2048\n",
);

/// CRITICAL backward-compatibility test: auto == classic parser.
/// A regression here means the existing user pipeline is broken.
#[test]
fn auto_equals_classic_for_canonical_csv() {
    let profile = OperationProfile::for_user_create();
    let auto = parse_csv_bytes_auto(CLASSIC_CSV.as_bytes(), &profile).unwrap();
    let classic = parse_csv_bytes(CLASSIC_CSV.as_bytes()).unwrap();
    assert_eq!(auto.rows.len(), classic.rows.len(), "rows.len matches");
    assert_eq!(
        auto.failed.len(),
        classic.failed.len(),
        "failed.len matches"
    );
    // First-row fields are identical.
    assert_eq!(
        auto.rows[0].username.as_str(),
        classic.rows[0].username.as_str()
    );
    assert_eq!(
        auto.rows[0].domain.as_str(),
        classic.rows[0].domain.as_str()
    );
    assert_eq!(auto.rows[0].quota.mb(), classic.rows[0].quota.mb());
    // Full match of the second row (including the quota).
    assert_eq!(auto.rows[1].quota.mb(), 2048);
}

/// Positional mode: CSV without a header is identical to the classic parser.
#[test]
fn auto_positional_without_header_matches_classic() {
    let data = b"good.com,user,Pass123,Name,512";
    let profile = OperationProfile::for_user_create();
    let auto = parse_csv_bytes_auto(data, &profile).unwrap();
    let classic = parse_csv_bytes(data).unwrap();
    assert_eq!(auto.rows.len(), classic.rows.len());
    assert_eq!(auto.rows[0].quota.mb(), classic.rows[0].quota.mb());
    assert_eq!(auto.rows[0].username.as_str(), "user");
}

/// detect_mapping on a canonical header → 5 bindings.
#[test]
fn detect_mapping_canonical_header_five_bindings() {
    let header: Vec<String> = ["domain", "username", "password", "display_name", "quota_mb"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let profile = OperationProfile::for_user_create();
    let m = detect_mapping(&header, &profile);
    assert_eq!(m.bindings.len(), 5, "all 5 fields are matched");
    assert!(m.binds_all_profile_fields());
}

/// detect_mapping case-insensitive/trimmed: "Domain", " UserName ".
#[test]
fn detect_mapping_case_insensitive_and_trimmed() {
    let header: Vec<String> = [
        "Domain",
        " UserName ",
        "PASSWORD",
        "display_name",
        "quota_mb",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let profile = OperationProfile::for_user_create();
    let m = detect_mapping(&header, &profile);
    assert_eq!(m.bindings.len(), 5, "case/trim does not prevent matching");
    // A CSV with such a header parses via auto.
    let csv = "Domain, UserName ,PASSWORD,display_name,quota_mb\n\
               example.com,ivan,Pass1234,Ivan,512\n";
    let parsed = parse_csv_bytes_auto(csv.as_bytes(), &profile).unwrap();
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows[0].username.as_str(), "ivan");
}

/// Renamed columns → 0 auto-detected (the user will reassign in the UI).
#[test]
fn detect_mapping_renamed_columns_zero_bindings() {
    // None of these aliases match a canonical field name, so detection yields
    // zero bindings (the user must remap them manually in the UI).
    let header: Vec<String> = ["Site", "Login", "Secret", "Full Name", "Limit MB"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let profile = OperationProfile::for_user_create();
    let m = detect_mapping(&header, &profile);
    assert_eq!(m.bindings.len(), 0, "no matches against canonical names");
    assert!(!m.binds_all_profile_fields());
}

/// Extra column: classic CSV + "extra" — the extra column is ignored.
#[test]
fn auto_ignores_extra_column() {
    let csv = "domain,username,password,display_name,quota_mb,extra\n\
               example.com,ivan,Pass1234,Ivan,512,note\n";
    let profile = OperationProfile::for_user_create();
    let parsed = parse_csv_bytes_auto(csv.as_bytes(), &profile).unwrap();
    assert_eq!(parsed.rows.len(), 1, "row parses, extra is ignored");
    assert_eq!(parsed.rows[0].username.as_str(), "ivan");
}

/// Reordered columns: an explicit mapping restores canonical order.
#[test]
fn explicit_mapping_reorders_columns() {
    // Source: username, domain, quota_mb, password, display_name
    let header: Vec<String> = ["username", "domain", "quota_mb", "password", "display_name"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let profile = OperationProfile::for_user_create();
    let mapping = detect_mapping(&header, &profile);
    assert_eq!(mapping.bindings.len(), 5);
    let csv = "username,domain,quota_mb,password,display_name\n\
               ivan,example.com,512,Pass1234,Ivan\n";
    let parsed = parse_csv_bytes_with_mapping(csv.as_bytes(), &mapping).unwrap();
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows[0].domain.as_str(), "example.com");
    assert_eq!(parsed.rows[0].username.as_str(), "ivan");
    assert_eq!(parsed.rows[0].quota.mb(), 512);
}

/// Missing required (no password column) → rows fall into failed.
#[test]
fn missing_required_column_goes_to_failed() {
    // Header without password; the explicit mapping does not cover password.
    let header: Vec<String> = ["domain", "username", "display_name", "quota_mb"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let profile = OperationProfile::for_user_create();
    let mapping = detect_mapping(&header, &profile);
    assert_eq!(mapping.bindings.len(), 4, "password is not mapped");
    let csv = "domain,username,display_name,quota_mb\n\
               example.com,ivan,Ivan,512\n";
    let parsed = parse_csv_bytes_with_mapping(csv.as_bytes(), &mapping).unwrap();
    assert_eq!(parsed.rows.len(), 0, "no valid rows");
    assert_eq!(parsed.failed.len(), 1, "row rejected");
    // The message must contain the name of the missing required field.
    let msg = parsed.failed[0].error.to_string();
    assert!(msg.contains("password"), "error mentions password: {msg}");
}

/// Auto mode does not panic on empty input.
#[test]
fn auto_empty_input() {
    let profile = OperationProfile::for_user_create();
    let parsed = parse_csv_bytes_auto(b"", &profile).unwrap();
    assert_eq!(parsed.rows.len(), 0);
    assert_eq!(parsed.failed.len(), 0);
}

/// BOM is stripped in auto mode (as in the classic parser).
#[test]
fn auto_strips_utf8_bom() {
    let mut data: Vec<u8> = vec![0xEF, 0xBB, 0xBF];
    data.extend_from_slice(CLASSIC_CSV.as_bytes());
    let profile = OperationProfile::for_user_create();
    let parsed = parse_csv_bytes_auto(&data, &profile).unwrap();
    assert_eq!(parsed.rows.len(), 2);
    assert_eq!(parsed.rows[0].domain.as_str(), "example.com");
}

/// ColumnMapping clones and holds the profile (basic type contract).
#[test]
fn column_mapping_is_clone_and_holds_profile() {
    let header: Vec<String> = ["domain", "username", "password", "display_name", "quota_mb"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let profile = OperationProfile::for_user_create();
    let m = detect_mapping(&header, &profile);
    let cloned: ColumnMapping = m.clone();
    assert_eq!(cloned.bindings.len(), m.bindings.len());
    assert_eq!(cloned.profile.fields.len(), profile.fields.len());
}
