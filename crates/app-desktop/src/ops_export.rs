//! Export: local saving of the CSV of the last operation.
//!
//! Extracted from [`crate::ops`] to comply with the spec's file-size limit of
//! ≤400 lines. The user picks a format in the modal: encrypted
//! (XChaCha20-Poly1305, the default) or plain CSV. Passwords in a plain CSV are
//! stored unencrypted — the warning is reflected in the choice UI.
//!
//! ## Execution flow (crash fix)
//!
//! Previously `rfd::FileDialog::save_file()` was called synchronously right in
//! `onclick` on the single Dioxus event-loop thread. The native Windows dialog
//! spins its own message loop and reentrantly enters the Dioxus runtime →
//! `RefCell already borrowed`. Now the whole export pipeline (dialog → KDF →
//! file write → audit) runs in a background task: the dialog is
//! `rfd::AsyncFileDialog` (it spawns its own thread, does not block the UI),
//! KDF+`std::fs::write` run via `spawn_blocking` on the tokio runtime, and the
//! audit record is written on an already-cloned `Arc<AuditWriter>` without
//! borrowing the signal.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use crate::audit_ui::AuditWriter;
use crate::batch::BatchResult;
use crate::state::AppState;
use crate::util::now_rfc3339;
use dioxus::prelude::*;
use mailgrit_core_domain::SanitizedUserRow;
use mailgrit_core_storage::AuditAction;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;

/// The outcome of the background export file write (from `spawn_blocking`).
/// `String` is a human-readable error (localized in the UI task).
type WriteOutcome = std::result::Result<(), String>;

/// Opens the export-format selection modal (encrypted/plain).
/// First checks that there is something to export.
pub fn open_export_choice(state: &mut Signal<AppState>) {
    let (has_csv, has_result) = {
        let read = state.read();
        (read.csv.is_some(), read.batch_result.is_some())
    };
    if !has_csv && !has_result {
        state.write().error_msg = Some(t!("export.nothing").to_string());
        return;
    }
    state.write().export.pending_export_choice = true;
}

/// Performs the export in the selected mode. When `encrypt == true`, the export
/// is sealed via XChaCha20-Poly1305 (the master password is required); when
/// `false`, a plain CSV is written.
///
/// Encrypted file format: `salt(16) || nonce(24) || ciphertext+tag`. The key is
/// NOT written next to the file: decryption requires the master password.
///
/// The whole pipeline goes into a background task (`spawn`), so the method
/// returns instantly and does not block the UI thread. A repeated launch is
/// blocked by the `export_in_progress` flag.
pub fn do_export(state: &mut Signal<AppState>, encrypt: bool) {
    // Reentrancy guard: if an export is already running — ignore the click.
    if state.read().export.export_in_progress {
        return;
    }

    // Snapshot the data in a short read-scope: everything the task needs is
    // cloned here, so the signal borrow is NOT held during the dialog/KDF/IO.
    // Canonicalized rows are taken from the editable table (actual passwords),
    // not from the original CSV snapshot — see `build_export_text`.
    let rows = crate::screens::csv_load::collect_sanitized_rows(state).0;
    let master_password;
    let audit;
    let result_snapshot;
    {
        let read = state.read();
        master_password = read.master_password.clone();
        audit = read.audit.clone();
        result_snapshot = read.batch_result.clone();
    }
    if rows.is_empty() && result_snapshot.is_none() {
        state.write().error_msg = Some(t!("export.nothing").to_string());
        return;
    }
    let plaintext = build_export_text_from(&rows, result_snapshot.as_deref());

    // For encrypted export, the master password is checked BEFORE starting the
    // task: if it is missing, record the export intent and open the
    // password-entry modal. After a successful unlock (`unlock_audit`), the
    // export resumes automatically via `pending_export_after_unlock` — previously
    // the intent was lost (the format-choice modal was already closed), and for
    // the user the export "did not work".
    if encrypt && master_password.is_none() {
        state.write().export.pending_export_after_unlock = true;
        state.write().modals.pending_master_password = true;
        state.write().error_msg = Some(t!("master_password.export_no_audit").to_string());
        return;
    }

    state.write().export.export_in_progress = true;
    // Row count for the audit message: the credential snapshot takes precedence
    // (create); otherwise the editable table (edit/delete/before an operation).
    let row_count = result_snapshot
        .as_ref()
        .map_or(rows.len(), |r| r.created_credentials.len());

    let mut state_clone = *state;
    spawn(async move {
        // 1. Native save dialog. AsyncFileDialog on Windows spawns a separate
        //    thread for the blocking part — the UI thread does not freeze and
        //    does not reenter the Dioxus runtime (root-cause crash fix).
        let title = tr!("ops.diag_file_dialog");
        let handle = rfd::AsyncFileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("mailgrit-export.csv")
            .set_title(title)
            .save_file()
            .await;
        let Some(handle) = handle else {
            // The user cancelled — reset the flag and exit quietly.
            state_clone.write().export.export_in_progress = false;
            return;
        };
        let path: PathBuf = handle.path().to_path_buf();
        let path_display = path.display().to_string();

        // 2. KDF + file write — CPU/IO-heavy work (Argon2id memory-hard, a
        //    blocking write). Offloaded to `spawn_blocking` on the tokio
        //    runtime's blocking pool: the UI does not freeze, no signal borrow
        //    is needed.
        let plaintext_bytes = plaintext.into_bytes();
        let outcome: WriteOutcome = if encrypt {
            let Some(master_password) = master_password.as_deref() else {
                // Unreachable: checked above, but fail-closed.
                let mut s = state_clone.write();
                s.export.export_in_progress = false;
                s.error_msg = Some(t!("master_password.export_no_audit").to_string());
                return;
            };
            let pw = master_password.to_string();
            let join = crate::tokio_runtime().spawn_blocking(move || -> WriteOutcome {
                let file_bytes = build_encrypted_bytes(&pw, &plaintext_bytes)?;
                std::fs::write(&path, file_bytes).map_err(|e| e.to_string())
            });
            match join.await {
                Ok(res) => res,
                Err(e) => Err(e.to_string()),
            }
        } else {
            let join = crate::tokio_runtime().spawn_blocking(move || -> WriteOutcome {
                std::fs::write(&path, plaintext_bytes).map_err(|e| e.to_string())
            });
            match join.await {
                Ok(res) => res,
                Err(e) => Err(e.to_string()),
            }
        };

        match outcome {
            Ok(()) => record_export_success(
                &mut state_clone,
                audit.as_ref(),
                encrypt,
                row_count,
                &path_display,
            ),
            Err(e) => {
                let mut s = state_clone.write();
                s.export.export_in_progress = false;
                s.error_msg = Some(t!("export.save_error", error = e).to_string());
            }
        }
    });
}

/// A pure builder of the export text (with no `Signal` dependency): extracted so
/// it can be covered by a unit test on a fixture. Header + rows + an operation
/// summary.
///
/// ## Data source (password-loss fix)
///
/// Passwords live only in `editable_rows`, which is reset on target change (the
/// User/Domain/Admin tab). Therefore an export that relied only on the editable
/// table yielded an empty file without passwords after switching the tab.
///
/// The source priority is now:
/// 1. **`result.created_credentials`** — a snapshot of the passwords actual at
///    the time of the operation (valid, confirmed by the server). Available even
///    if `editable_rows` is already cleared. Applied for create.
/// 2. **`rows` (the editable table)** — a fallback for edit/delete (no accounts
///    are created → no snapshot) and for an export before the first operation.
///
/// The historical "stale passwords" fix: the data is taken from the editable
/// table (`collect_sanitized_rows` in the caller) and reuses the same typestate
/// pipeline as `launch_op`, reflecting the current state (not the original CSV
/// snapshot).
fn build_export_text_from(rows: &[SanitizedUserRow], result_opt: Option<&BatchResult>) -> String {
    let mut out = String::new();
    out.push_str("# MailGrit export\n");
    out.push_str("domain,username,password,display_name,quota_mb\n");

    // Priority 1: the credential snapshot from BatchResult (create) — available
    // even after editable_rows is cleared. RFC 4180-escape each field.
    let mut wrote_from_credentials = false;
    if let Some(result) = result_opt {
        for c in &result.created_credentials {
            let _ = writeln!(
                out,
                "{},{},{},{},{}",
                csv_escape(&c.domain),
                csv_escape(&c.username),
                csv_escape(&c.password),
                csv_escape(&c.display_name),
                c.quota_mb
            );
            wrote_from_credentials = true;
        }
    }

    // Priority 2 (fallback): rows from the editable table — for edit/delete or
    // an export before an operation. Applied only if no row was exported from
    // the snapshot (otherwise the created accounts would be duplicated).
    if !wrote_from_credentials {
        for row in rows {
            let _ = writeln!(
                out,
                "{},{},{},{},{}",
                csv_escape(row.domain.as_str()),
                csv_escape(row.username.as_str()),
                csv_escape(row.password.as_secret_str()),
                csv_escape(row.display_name.as_str()),
                row.quota.mb()
            );
        }
    }

    if let Some(result) = result_opt {
        let succ = t!("result.succeeded", count = result.succeeded);
        let fail = t!("result.failed", count = result.failed);
        let _ = writeln!(out, "\n# {succ} / {fail}");
        for f in &result.failures {
            let _ = writeln!(
                out,
                "# FAIL {}@{}: {}",
                csv_escape(&f.username),
                csv_escape(&f.domain),
                csv_escape(&f.reason)
            );
        }
    }
    out
}

/// Escapes a CSV field per RFC 4180: wraps it in double quotes if the field
/// contains a comma, quote, newline, or carriage return; inside quotes, quotes
/// are doubled. The password generator produces values without these characters,
/// but `display_name` (and a FAIL reason) may contain a comma/quote (e.g.
/// "Petrov, Ivan"), which without escaping would break the column count in the
/// export.
fn csv_escape(field: &str) -> String {
    let needs_quoting = field.chars().any(|c| matches!(c, ',' | '"' | '\n' | '\r'));
    if !needs_quoting {
        return field.to_owned();
    }
    // Double the quotes and wrap the field in quotes.
    let mut escaped = String::with_capacity(field.len().saturating_add(2));
    escaped.push('"');
    for c in field.chars() {
        if c == '"' {
            escaped.push('"');
        }
        escaped.push(c);
    }
    escaped.push('"');
    escaped
}

/// Encrypts the plaintext with the master password: returns
/// `salt(16) || nonce || ciphertext+tag`. File format: the salt (for the KDF) in
/// the clear + AEAD-ciphertext. The key is NOT stored.
fn build_encrypted_bytes(master_password: &str, plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let salt = mailgrit_core_security::generate_salt();
    let derived = mailgrit_core_security::derive_key(master_password.as_bytes(), &salt)
        .map_err(|e| e.to_string())?;
    let export_key =
        mailgrit_core_security::EncryptionKey::from_bytes(&derived).map_err(|e| e.to_string())?;
    let ciphertext = mailgrit_core_security::encrypt(&export_key, plaintext, b"MailGrit-export-v1")
        .map_err(|e| e.to_string())?;
    // Assemble the file: salt(16) || ciphertext (the nonce is already included in
    // the ciphertext).
    let mut file_bytes = Vec::with_capacity(salt.len().saturating_add(ciphertext.len()));
    file_bytes.extend_from_slice(&salt);
    file_bytes.extend_from_slice(&ciphertext);
    Ok(file_bytes)
}

/// Records a successful export in the audit log and updates the UI state.
///
/// Takes an already-cloned `Arc<AuditWriter>` (rather than borrowing the signal):
/// a SQLite write under a live `&state.read().audit` was a reentrant
/// anti-pattern. Now the signal borrow is held only for a short write-scope at
/// the very end.
fn record_export_success(
    state: &mut Signal<AppState>,
    audit: Option<&Arc<AuditWriter>>,
    encrypt: bool,
    row_count: usize,
    path_display: &str,
) {
    let timestamp = now_rfc3339();
    let detail = if encrypt {
        format!("local encrypted CSV export: {row_count} rows → {path_display}")
    } else {
        format!("local plain CSV export: {row_count} rows → {path_display}")
    };
    // Audit record on a cloned Arc — without borrowing the signal.
    if let Some(audit) = audit
        && let Err(e) = audit.append_simple(AuditAction::Export, &detail, &timestamp)
    {
        tracing::warn!("export audit record not written: {e}");
    }
    let msg = if encrypt {
        t!("export.encrypted_ok", path = path_display).to_string()
    } else {
        t!("export.plain_ok", path = path_display).to_string()
    };
    // One write-scope at the end: update the audit list, the flag, and the message.
    let mut s = state.write();
    s.refresh_audit();
    s.export.export_in_progress = false;
    s.error_msg = Some(msg);
}

#[cfg(test)]
mod tests {
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
        assert_eq!(csv_escape("ivan.petrov"), "ivan.petrov");
        assert_eq!(csv_escape(""), "");
        assert_eq!(csv_escape("example.com"), "example.com");
    }

    /// RFC 4180: a comma in a field (e.g. "Petrov, Ivan") → wrap in quotes.
    /// Previously such a display_name broke the column count in the export.
    #[test]
    fn csv_escape_quotes_comma_field() {
        assert_eq!(csv_escape("Petrov, Ivan"), "\"Petrov, Ivan\"");
    }

    /// RFC 4180: a double quote inside → double it and wrap the field.
    #[test]
    fn csv_escape_doubles_inner_quotes() {
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    /// RFC 4180: a newline in a field also requires quotes.
    #[test]
    fn csv_escape_newline_field() {
        assert_eq!(csv_escape("line1\nline2"), "\"line1\nline2\"");
        assert_eq!(csv_escape("a\rb"), "\"a\rb\"");
    }

    /// Password-loss regression: after switching the tab, editable_rows is empty,
    /// but BatchResult.created_credentials holds a snapshot of the created
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

    /// Source priority: if created_credentials are present, the rows from the
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
}
