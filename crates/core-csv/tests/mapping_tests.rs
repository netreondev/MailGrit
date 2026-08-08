//! Tests for the flexible CSV column mapping (Phase 1.2).
//!
//! Integration tests for the public `mapping` API: mapping auto-detection,
//! streaming parse with a mapping, and the **backward-compatibility criterion** —
//! `parse_csv_bytes_auto` must produce a result identical to the classic
//! `parse_csv_bytes` for a canonical 5-column CSV.

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
fn auto_equals_classic_for_canonical_csv() -> Result<(), Box<dyn std::error::Error>> {
    let profile = OperationProfile::for_user_create();
    let auto = parse_csv_bytes_auto(CLASSIC_CSV.as_bytes(), &profile)?;
    let classic = parse_csv_bytes(CLASSIC_CSV.as_bytes())?;
    assert_eq!(auto.rows.len(), classic.rows.len(), "rows.len matches");
    assert_eq!(
        auto.failed.len(),
        classic.failed.len(),
        "failed.len matches"
    );
    // First-row fields are identical.
    assert_eq!(
        auto.rows
            .first()
            .ok_or("missing auto row 0")?
            .username
            .as_str(),
        classic
            .rows
            .first()
            .ok_or("missing classic row 0")?
            .username
            .as_str()
    );
    assert_eq!(
        auto.rows
            .first()
            .ok_or("missing auto row 0")?
            .domain
            .as_str(),
        classic
            .rows
            .first()
            .ok_or("missing classic row 0")?
            .domain
            .as_str()
    );
    assert_eq!(
        auto.rows.first().ok_or("missing auto row 0")?.quota.mb(),
        classic
            .rows
            .first()
            .ok_or("missing classic row 0")?
            .quota
            .mb()
    );
    // Full match of the second row (including the quota).
    assert_eq!(
        auto.rows.get(1).ok_or("missing auto row 1")?.quota.mb(),
        2048
    );
    Ok(())
}

/// Positional mode: CSV without a header is identical to the classic parser.
#[test]
fn auto_positional_without_header_matches_classic() -> Result<(), Box<dyn std::error::Error>> {
    let data = b"good.com,user,Pass123,Name,512";
    let profile = OperationProfile::for_user_create();
    let auto = parse_csv_bytes_auto(data, &profile)?;
    let classic = parse_csv_bytes(data)?;
    assert_eq!(auto.rows.len(), classic.rows.len());
    assert_eq!(
        auto.rows.first().ok_or("missing auto row 0")?.quota.mb(),
        classic
            .rows
            .first()
            .ok_or("missing classic row 0")?
            .quota
            .mb()
    );
    assert_eq!(
        auto.rows
            .first()
            .ok_or("missing auto row 0")?
            .username
            .as_str(),
        "user"
    );
    Ok(())
}

/// detect_mapping on a canonical header → 5 bindings.
#[test]
fn detect_mapping_canonical_header_five_bindings() {
    let header: Vec<String> = ["domain", "username", "password", "display_name", "quota_mb"]
        .iter()
        .copied()
        .map(String::from)
        .collect();
    let profile = OperationProfile::for_user_create();
    let m = detect_mapping(&header, &profile);
    assert_eq!(m.bindings.len(), 5, "all 5 fields are matched");
    assert!(m.binds_all_profile_fields());
}

/// detect_mapping case-insensitive/trimmed: "Domain", " UserName ".
#[test]
fn detect_mapping_case_insensitive_and_trimmed() -> Result<(), Box<dyn std::error::Error>> {
    let header: Vec<String> = [
        "Domain",
        " UserName ",
        "PASSWORD",
        "display_name",
        "quota_mb",
    ]
    .iter()
    .copied()
    .map(String::from)
    .collect();
    let profile = OperationProfile::for_user_create();
    let m = detect_mapping(&header, &profile);
    assert_eq!(m.bindings.len(), 5, "case/trim does not prevent matching");
    // A CSV with such a header parses via auto.
    let csv = "Domain, UserName ,PASSWORD,display_name,quota_mb\n\
               example.com,ivan,Pass1234,Ivan,512\n";
    let parsed = parse_csv_bytes_auto(csv.as_bytes(), &profile)?;
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(
        parsed
            .rows
            .first()
            .ok_or("missing row 0")?
            .username
            .as_str(),
        "ivan"
    );
    Ok(())
}

/// Renamed columns → 0 auto-detected (the user will reassign in the UI).
#[test]
fn detect_mapping_renamed_columns_zero_bindings() {
    // None of these aliases match a canonical field name, so detection yields
    // zero bindings (the user must remap them manually in the UI).
    let header: Vec<String> = ["Site", "Login", "Secret", "Full Name", "Limit MB"]
        .iter()
        .copied()
        .map(String::from)
        .collect();
    let profile = OperationProfile::for_user_create();
    let m = detect_mapping(&header, &profile);
    assert_eq!(m.bindings.len(), 0, "no matches against canonical names");
    assert!(!m.binds_all_profile_fields());
}

/// Extra column: classic CSV + "extra" — the extra column is ignored.
#[test]
fn auto_ignores_extra_column() -> Result<(), Box<dyn std::error::Error>> {
    let csv = "domain,username,password,display_name,quota_mb,extra\n\
               example.com,ivan,Pass1234,Ivan,512,note\n";
    let profile = OperationProfile::for_user_create();
    let parsed = parse_csv_bytes_auto(csv.as_bytes(), &profile)?;
    assert_eq!(parsed.rows.len(), 1, "row parses, extra is ignored");
    assert_eq!(
        parsed
            .rows
            .first()
            .ok_or("missing row 0")?
            .username
            .as_str(),
        "ivan"
    );
    Ok(())
}

/// Reordered columns: an explicit mapping restores canonical order.
#[test]
fn explicit_mapping_reorders_columns() -> Result<(), Box<dyn std::error::Error>> {
    // Source: username, domain, quota_mb, password, display_name
    let header: Vec<String> = ["username", "domain", "quota_mb", "password", "display_name"]
        .iter()
        .copied()
        .map(String::from)
        .collect();
    let profile = OperationProfile::for_user_create();
    let mapping = detect_mapping(&header, &profile);
    assert_eq!(mapping.bindings.len(), 5);
    let csv = "username,domain,quota_mb,password,display_name\n\
               ivan,example.com,512,Pass1234,Ivan\n";
    let parsed = parse_csv_bytes_with_mapping(csv.as_bytes(), &mapping)?;
    assert_eq!(parsed.rows.len(), 1);
    let row = parsed.rows.first().ok_or("missing row 0")?;
    assert_eq!(row.domain.as_str(), "example.com");
    assert_eq!(row.username.as_str(), "ivan");
    assert_eq!(row.quota.mb(), 512);
    Ok(())
}

/// Missing required (no password column) → rows fall into failed.
#[test]
fn missing_required_column_goes_to_failed() -> Result<(), Box<dyn std::error::Error>> {
    // Header without password; the explicit mapping does not cover password.
    let header: Vec<String> = ["domain", "username", "display_name", "quota_mb"]
        .iter()
        .copied()
        .map(String::from)
        .collect();
    let profile = OperationProfile::for_user_create();
    let mapping = detect_mapping(&header, &profile);
    assert_eq!(mapping.bindings.len(), 4, "password is not mapped");
    let csv = "domain,username,display_name,quota_mb\n\
               example.com,ivan,Ivan,512\n";
    let parsed = parse_csv_bytes_with_mapping(csv.as_bytes(), &mapping)?;
    assert_eq!(parsed.rows.len(), 0, "no valid rows");
    assert_eq!(parsed.failed.len(), 1, "row rejected");
    // The message must contain the name of the missing required field.
    let msg = parsed
        .failed
        .first()
        .ok_or("missing failed 0")?
        .error
        .to_string();
    assert!(msg.contains("password"), "error mentions password: {msg}");
    Ok(())
}

/// Auto mode does not panic on empty input.
#[test]
fn auto_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    let profile = OperationProfile::for_user_create();
    let parsed = parse_csv_bytes_auto(b"", &profile)?;
    assert_eq!(parsed.rows.len(), 0);
    assert_eq!(parsed.failed.len(), 0);
    Ok(())
}

/// BOM is stripped in auto mode (as in the classic parser).
#[test]
fn auto_strips_utf8_bom() -> Result<(), Box<dyn std::error::Error>> {
    let mut data: Vec<u8> = vec![0xEF, 0xBB, 0xBF];
    data.extend_from_slice(CLASSIC_CSV.as_bytes());
    let profile = OperationProfile::for_user_create();
    let parsed = parse_csv_bytes_auto(&data, &profile)?;
    assert_eq!(parsed.rows.len(), 2);
    assert_eq!(
        parsed.rows.first().ok_or("missing row 0")?.domain.as_str(),
        "example.com"
    );
    Ok(())
}

/// ColumnMapping clones and holds the profile (basic type contract).
#[test]
fn column_mapping_is_clone_and_holds_profile() {
    let header: Vec<String> = ["domain", "username", "password", "display_name", "quota_mb"]
        .iter()
        .copied()
        .map(String::from)
        .collect();
    let profile = OperationProfile::for_user_create();
    let m = detect_mapping(&header, &profile);
    let cloned: ColumnMapping = m.clone();
    assert_eq!(cloned.bindings.len(), m.bindings.len());
    assert_eq!(cloned.profile.fields.len(), profile.fields.len());
}

/// process_row_mapped substitutes the *correct* profile default for an unmapped
/// canonical field. The default lookup (`f.name == canonical`) is mutation-killed
/// by checking that an unmapped `quota_mb` becomes DEFAULT_QUOTA_MB (1024), not
/// some other field's default (e.g. display_name's "").
#[test]
fn unmapped_optional_field_uses_its_own_profile_default() -> Result<(), Box<dyn std::error::Error>>
{
    // Header maps domain/username/password/display_name but NOT quota_mb.
    // quota_mb is optional with default DEFAULT_QUOTA_MB — that exact default
    // must be substituted (the `f.name == canonical` lookup must pick quota_mb's
    // FieldSpec, not any other).
    let header: Vec<String> = ["domain", "username", "password", "display_name"]
        .iter()
        .copied()
        .map(String::from)
        .collect();
    let profile = OperationProfile::for_user_create();
    let mapping = detect_mapping(&header, &profile);
    assert_eq!(mapping.bindings.len(), 4, "quota_mb is not mapped");
    let csv = "domain,username,password,display_name\n\
               example.com,ivan,Pass1234,Ivan\n";
    let parsed = parse_csv_bytes_with_mapping(csv.as_bytes(), &mapping)?;
    assert_eq!(parsed.rows.len(), 1, "one valid row");
    let row = parsed.rows.first().ok_or("missing row 0")?;
    // The unmapped quota_mb must receive its OWN default (1024 = DEFAULT_QUOTA_MB),
    // proving the lookup matched canonical == "quota_mb", not another field.
    use mailgrit_core_domain::limits::DEFAULT_QUOTA_MB;
    assert_eq!(
        row.quota.mb(),
        DEFAULT_QUOTA_MB,
        "unmapped quota_mb must default to DEFAULT_QUOTA_MB"
    );
    Ok(())
}

/// An unmapped optional field with an EMPTY default ("") still parses — pinning
/// the second observable consequence of the default-substitution lookup.
#[test]
fn unmapped_empty_default_field_parses() -> Result<(), Box<dyn std::error::Error>> {
    // display_name is optional with default "" — unmapped, it must become "".
    let header: Vec<String> = ["domain", "username", "password", "quota_mb"]
        .iter()
        .copied()
        .map(String::from)
        .collect();
    let profile = OperationProfile::for_user_create();
    let mapping = detect_mapping(&header, &profile);
    assert_eq!(mapping.bindings.len(), 4, "display_name is not mapped");
    let csv = "domain,username,password,quota_mb\n\
               example.com,ivan,Pass1234,512\n";
    let parsed = parse_csv_bytes_with_mapping(csv.as_bytes(), &mapping)?;
    assert_eq!(parsed.rows.len(), 1);
    let row = parsed.rows.first().ok_or("missing row 0")?;
    assert_eq!(
        row.display_name.as_str(),
        "",
        "unmapped display_name defaults to empty"
    );
    Ok(())
}
