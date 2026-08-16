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

// --- ExportError Display (typed failure kinds stay distinguishable) ---

#[test]
fn export_error_display_distinguishes_kinds() {
    let io = ExportError::Io(std::io::Error::other("disk full")).to_string();
    assert!(
        io.contains("file write:") && io.contains("disk full"),
        "Io display: {io}"
    );
    let crypto = ExportError::Crypto(mailgrit_core_security::SecurityError::CiphertextTooShort {
        actual: 0,
        min: 24,
    })
    .to_string();
    assert!(
        crypto.contains("encryption:") && crypto.contains("ciphertext too short"),
        "Crypto display: {crypto}"
    );
}

// --- build_encrypted_bytes (file format: salt(16) || nonce || ct+tag) ---

#[test]
fn encrypted_export_file_layout_and_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let plaintext = b"domain,username,password\nexample.com,u,p\n";
    let file = build_encrypted_bytes("master-password", plaintext)?;
    // Layout: 16-byte salt + 24-byte nonce + ciphertext+tag (16-byte tag).
    assert_eq!(
        file.len(),
        16 + 24 + 16 + plaintext.len(),
        "salt(16) || nonce(24) || ciphertext+tag layout: {} bytes",
        file.len()
    );
    let (salt, ciphertext) = file.split_at(16);
    // Round-trip: derive from the same password + stored salt, decrypt with
    // the same AAD.
    let key_bytes = mailgrit_core_security::derive_key(b"master-password", salt)?;
    let key = mailgrit_core_security::EncryptionKey::from_bytes(key_bytes.as_slice())?;
    let decrypted = mailgrit_core_security::decrypt(&key, ciphertext, b"MailGrit-export-v1")?;
    assert_eq!(decrypted.as_slice(), plaintext);
    // A different AAD must fail authentication (the format tag is pinned).
    assert!(
        mailgrit_core_security::decrypt(&key, ciphertext, b"other-format").is_err(),
        "AAD mismatch must be rejected"
    );
    Ok(())
}

// --- Signal-coupled entry points (Dioxus runtime harness, as in ops_tests) ---

/// Same harness contract as `ops_tests::with_app_state`: setup + body run
/// inside a minimal Dioxus runtime ROOT scope (signals need it).
fn with_app_state<O>(
    mut setup: impl FnMut(&mut AppState),
    body: impl FnOnce(&mut dioxus::prelude::Signal<AppState>) -> O,
) -> O {
    use dioxus::prelude::*;
    let vdom = Box::leak(Box::new(VirtualDom::new(|| rsx! {})));
    let runtime = vdom.runtime();
    runtime.in_scope(ScopeId::ROOT, || {
        let mut sig = Signal::new(AppState::default());
        setup(&mut sig.write());
        body(&mut sig)
    })
}

fn parsed_csv() -> Result<std::sync::Arc<mailgrit_core_csv::ParsedCsv>, Box<dyn std::error::Error>>
{
    let data = b"domain,username,password,display_name,quota_mb\n\
                 example.com,ivan.petrov,S3cur3P@ss1,Ivan Petrov,1024\n";
    Ok(std::sync::Arc::new(mailgrit_core_csv::parse_csv_bytes(
        data,
    )?))
}

fn valid_editable_row() -> mailgrit_core_domain::EditableUserRow {
    mailgrit_core_domain::EditableUserRow {
        domain: "example.com".into(),
        username: "ivan.petrov".into(),
        password: "S3cur3P@ss1".into(),
        display_name: "Ivan Petrov".into(),
        quota: "1024".into(),
    }
}

#[test]
fn open_export_choice_without_data_is_refused() {
    with_app_state(
        |_| {},
        |sig| {
            open_export_choice(sig);
            let read = sig.read();
            assert!(
                !read.export.pending_export_choice,
                "nothing to export -> no modal"
            );
            assert!(
                read.error_msg.is_some(),
                "the user must be told there is nothing to export"
            );
        },
    );
}

#[test]
fn open_export_choice_with_loaded_csv_opens_the_picker() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parsed_csv()?;
    with_app_state(
        |s| {
            s.csv.rows = Some(parsed.clone());
        },
        |sig| {
            open_export_choice(sig);
            let read = sig.read();
            assert!(
                read.export.pending_export_choice,
                "a loaded CSV alone is exportable (before any operation)"
            );
            assert!(read.error_msg.is_none());
        },
    );
    Ok(())
}

#[test]
fn open_export_choice_with_batch_result_only_opens_the_picker() {
    // Password-loss regression scenario: the tab was switched (editable_rows
    // and rows are gone) but the batch result snapshot is still exportable.
    with_app_state(
        |s| {
            s.csv.batch_result = Some(std::sync::Arc::new(BatchResult {
                succeeded: 1,
                failed: 0,
                failures: Vec::new(),
                created_credentials: Vec::new(),
            }));
        },
        |sig| {
            open_export_choice(sig);
            assert!(
                sig.read().export.pending_export_choice,
                "a stored batch result alone is exportable"
            );
        },
    );
}

#[test]
fn plain_export_begins_the_background_pipeline() {
    with_app_state(
        |s| {
            s.csv.editable_rows = Some(vec![valid_editable_row()]);
        },
        |sig| {
            do_export(sig, false);
            let read = sig.read();
            assert!(
                read.export.export_in_progress,
                "the export task must be marked as running"
            );
            assert!(
                !read.export.pending_export_after_unlock,
                "a plain export never waits for the master password"
            );
            assert!(
                !read.modals.pending_master_password,
                "no password modal for a plain export"
            );
        },
    );
}

#[test]
fn encrypted_export_without_master_password_defers_to_unlock() {
    with_app_state(
        |s| {
            s.csv.editable_rows = Some(vec![valid_editable_row()]);
            s.master_password = None;
        },
        |sig| {
            do_export(sig, true);
            let read = sig.read();
            assert!(
                read.export.pending_export_after_unlock,
                "the export intent must be recorded to resume after unlock"
            );
            assert!(
                read.modals.pending_master_password,
                "the password modal must open"
            );
            assert!(
                !read.export.export_in_progress,
                "the pipeline must not start without the key"
            );
        },
    );
}

#[test]
fn record_export_success_finishes_and_reports_the_path() {
    with_app_state(
        |s| {
            s.export.begin();
            s.error_msg = None;
        },
        |sig| {
            record_export_success(sig, None, false, 3, "C:/tmp/out.csv");
            let read = sig.read();
            assert!(!read.export.export_in_progress, "the flag must clear");
            let msg = read.error_msg.as_deref().unwrap_or_default();
            assert!(
                msg.contains("out.csv"),
                "the success message must name the file: {msg}"
            );
        },
    );
}
