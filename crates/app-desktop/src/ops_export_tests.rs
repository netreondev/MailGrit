//! Unit tests moved out of the production file (the `#[path]` pattern
//! used across the workspace; keeps the prod file under the 400-line spec).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use super::*;
use crate::batch::CredentialRow;

/// Regression for the "stale passwords" bug: the export must contain the
/// actual values from the editable table (after a password regeneration), not
/// from the original CSV snapshot. Here we check the pure builder on a
/// fixture.
#[test]
fn export_text_uses_edited_passwords() -> Result<(), mailgrit_core_domain::CsvRowError> {
    // Simulate an edited row with the new password "NewP@ss1!".
    let new_row = mailgrit_core_domain::RawCsvRow::new(vec![
        "example.com".into(),
        "ivan.petrov".into(),
        "NewP@ss1!".into(),
        "Ivan Petrov".into(),
        "1024".into(),
    ])
    .parse()?;
    let text = build_export_text_from(std::slice::from_ref(&new_row), None);
    assert!(
        text.contains("NewP@ss1!"),
        "the export must contain the actual password from the editable table: {text}"
    );
    assert!(text.contains("ivan.petrov"));
    assert!(text.contains("domain,username,password"));
    Ok(())
}

/// With no rows and no result — an empty export text (only the header). The
/// caller makes the "nothing to export" error decision.
#[test]
fn export_text_empty_has_header_only() {
    let text = build_export_text_from(&[], None);
    assert!(text.contains("domain,username,password"));
    assert!(!text.contains("FAIL"));
}

/// RFC 4180: a field without special characters is not escaped (left as is).
#[test]
fn csv_escape_plain_field_unchanged() {
    assert_eq!(escape_field("ivan.petrov"), "ivan.petrov");
    assert_eq!(escape_field(""), "");
    assert_eq!(escape_field("example.com"), "example.com");
}

/// RFC 4180: a comma in a field (e.g. "Petrov, Ivan") → wrap in quotes.
/// Previously such a `display_name` broke the column count in the export.
#[test]
fn csv_escape_quotes_comma_field() {
    assert_eq!(escape_field("Petrov, Ivan"), "\"Petrov, Ivan\"");
}

/// RFC 4180: a double quote inside → double it and wrap the field.
#[test]
fn csv_escape_doubles_inner_quotes() {
    assert_eq!(escape_field("say \"hi\""), "\"say \"\"hi\"\"\"");
}

/// RFC 4180: a newline in a field also requires quotes.
#[test]
fn csv_escape_newline_field() {
    assert_eq!(escape_field("line1\nline2"), "\"line1\nline2\"");
    assert_eq!(escape_field("a\rb"), "\"a\rb\"");
}

// CSV formula injection (audit finding): display_name and free-text FAIL
// reasons used to pass through unneutralized — opening the export in
// Excel/LibreOffice executed =HYPERLINK/@SUM/… cells. escape_field now
// prefixes formula-leading fields with an apostrophe.
#[test]
fn export_neutralizes_formula_injection() -> Result<(), mailgrit_core_domain::CsvRowError> {
    let row = mailgrit_core_domain::RawCsvRow::new(vec![
        "example.com".into(),
        "ivan.petrov".into(),
        "NewP@ss1!".into(),
        "=HYPERLINK(\"http://evil\")".into(),
        "1024".into(),
    ])
    .parse()?;
    let text = build_export_text_from(std::slice::from_ref(&row), None);
    assert!(
        !text.contains(",=HYPERLINK"),
        "a formula-leading display_name must be neutralized: {text}"
    );
    assert!(
        text.contains("\"'=HYPERLINK"),
        "the neutralized form must carry the apostrophe prefix: {text}"
    );

    // FAIL reasons (free text from the server) are neutralized too.
    let result = BatchResult {
        succeeded: 0,
        failed: 1,
        failures: vec![crate::batch::RowFailure {
            username: "ivan.petrov".into(),
            domain: "example.com".into(),
            reason: "@SUM(a1:a2) crashed".into(),
        }],
        created_credentials: Vec::new(),
    };
    let text = build_export_text_from(&[], Some(&result));
    assert!(
        text.contains("'@SUM"),
        "a formula-leading FAIL reason must be neutralized: {text}"
    );
    Ok(())
}

/// Password-loss regression: after switching the tab, `editable_rows` is empty,
/// but `BatchResult.created_credentials` holds a snapshot of the created
/// accounts — the export must export exactly those (with passwords).
#[test]
fn export_text_uses_created_credentials_when_rows_empty() {
    let result = BatchResult {
        succeeded: 1,
        failed: 0,
        failures: Vec::new(),
        created_credentials: vec![CredentialRow {
            domain: "dnipr.gp.gov.ua".into(),
            username: "ivan.petrov".into(),
            password: "S5v!i2&yQ9".into(),
            display_name: "Petrov, Ivan".into(),
            quota_mb: 1024,
        }],
    };
    // editable_rows is empty (simulating a clear by switching the tab).
    let text = build_export_text_from(&[], Some(&result));
    assert!(
        text.contains("S5v!i2&yQ9"),
        "the export must contain the password from created_credentials: {text}"
    );
    // A display_name with a comma is escaped.
    assert!(
        text.contains("\"Petrov, Ivan\""),
        "a display_name with a comma must be quoted: {text}"
    );
    // A data row is present (not only the header).
    assert!(text.contains("ivan.petrov"));
    assert!(text.contains("dnipr.gp.gov.ua"));
}

/// Source priority: if `created_credentials` are present, the rows from the
/// editable table are NOT duplicated in the export.
#[test]
fn export_text_does_not_duplicate_when_credentials_present()
-> Result<(), mailgrit_core_domain::CsvRowError> {
    let row = mailgrit_core_domain::RawCsvRow::new(vec![
        "example.com".into(),
        "dup.user".into(),
        "NewP@ss1!".into(),
        "Dup".into(),
        "1024".into(),
    ])
    .parse()?;
    let result = BatchResult {
        succeeded: 1,
        failed: 0,
        failures: Vec::new(),
        created_credentials: vec![CredentialRow {
            domain: "example.com".into(),
            username: "real.user".into(),
            password: "CreatedPass1!".into(),
            display_name: "Real".into(),
            quota_mb: 512,
        }],
    };
    let text = build_export_text_from(std::slice::from_ref(&row), Some(&result));
    // The row from created_credentials must be present …
    assert!(text.contains("real.user"));
    assert!(text.contains("CreatedPass1!"));
    // … and there must be NO duplicate from the editable table.
    assert!(
        !text.contains("dup.user"),
        "when created_credentials are present, the table rows are not duplicated: {text}"
    );
    Ok(())
}
