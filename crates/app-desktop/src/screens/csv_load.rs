//! Helper functions for loading CSV on the dashboard.
//!
//! Extracted from [`dashboard`](super) to keep each file under the 400-line
//! limit (the RSX screen is structurally large). This file handles native file
//! picking, auto column mapping, and CSV parsing.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::csv_summary::CsvSummary;
use crate::state::AppState;
use dioxus::prelude::*;
use mailgrit_core_csv::{
    CsvParseError, RecordOutcome, RecordReader, detect_mapping, parse_csv_bytes_auto,
};
use mailgrit_core_domain::{EditableUserRow, SanitizedUserRow};
use std::sync::Arc;

/// Loads and parses the selected CSV file into [`AppState`].
///
/// File reading runs via `spawn_blocking` on the tokio runtime's blocking
/// pool (so the Dioxus event-loop never stalls on large CSVs, and nothing
/// depends on `block_in_place`, which panics on a current-thread runtime);
/// parsing and state updates happen on the async continuation after the bytes
/// return.
///
/// Auto-mapping uses the profile of the selected target (for the classic
/// 5-column CSV the result is identical to `parse_csv_bytes`). Valid rows are
/// also duplicated into the editable layer `editable_rows` (plain `String`),
/// which the user edits directly in the table; on execution, rows are
/// re-validated via [`EditableUserRow::to_sanitized`].
pub fn load_csv_file(state: &Signal<AppState>, path: &std::path::Path) {
    let mut state_clone = *state;
    let path = path.to_path_buf();
    spawn(async move {
        let bytes = match crate::tokio_runtime()
            .spawn_blocking(move || std::fs::read(&path))
            .await
        {
            Ok(Ok(b)) => b,
            Ok(Err(e)) => {
                state_clone.write().error_msg = Some(t!("csv.read_error", error = e).to_string());
                return;
            }
            Err(e) => {
                state_clone.write().error_msg = Some(t!("csv.read_error", error = e).to_string());
                return;
            }
        };
        apply_loaded_csv(&mut state_clone, &bytes);
    });
}

/// Parses the CSV bytes and applies the result to the state (split out of
/// [`load_csv_file`] so the parse/apply path is unit-testable).
fn apply_loaded_csv(state: &mut Signal<AppState>, bytes: &[u8]) {
    let profile = state.read().effective_profile();
    match parse_csv_bytes_auto(bytes, &profile) {
        Ok(parsed) => {
            let header = extract_header(bytes);
            let mapping = detect_mapping(&header, &profile);
            let summary = CsvSummary::from_parsed(&parsed);
            // Editable layer: copy valid rows into plain `String`.
            let editable: Vec<EditableUserRow> =
                parsed.rows.iter().map(EditableUserRow::from).collect();
            let mut s = state.write();
            s.csv.rows = Some(Arc::new(parsed));
            s.csv.column_mapping = Some(Arc::new(mapping));
            s.csv.editable_rows = Some(editable);
            s.error_msg = (summary.failed > 0).then(|| {
                t!(
                    "csv.loaded_summary",
                    valid = summary.valid,
                    failed = summary.failed
                )
                .to_string()
            });
        }
        Err(CsvParseError::Io(e)) => {
            state.write().error_msg = Some(t!("csv.parse_io_error", error = e).to_string());
        }
        Err(e) => {
            // Localized error via error_i18n (the core crate has no i18n).
            state.write().error_msg = Some(crate::error_i18n::csv_parse_error(&e));
        }
    }
}

/// Extracts the CSV header (the first non-empty record) from the bytes for
/// auto-detecting the column mapping. Uses the core-csv [`RecordReader`] (the
/// single wire-format layer: quoted headers like `"Domain","Username"` are
/// unquoted correctly, matching what the parser itself sees).
fn extract_header(data: &[u8]) -> Vec<String> {
    let clean = mailgrit_core_csv::strip_bom(data);
    let mut reader = RecordReader::new(std::io::BufReader::new(clean));
    loop {
        let record = match reader.next_record() {
            Ok(Some(r)) => r,
            Ok(None) => return Vec::new(), // no non-empty lines
            Err(e) => {
                tracing::warn!("CSV read error, column auto-mapping disabled: {e}");
                return Vec::new();
            }
        };
        match record.outcome {
            RecordOutcome::Fields(fields) => {
                let blank = matches!(fields.as_slice(), [f] if f.trim().is_empty());
                if !blank {
                    return fields.iter().map(|f| f.trim().to_string()).collect();
                }
            }
            // A first record that failed to split cannot serve as a header
            // (mapping yields 0 bindings).
            RecordOutcome::Failed(_) => return Vec::new(),
        }
    }
}

/// Collects valid rows from the editable table for operation execution.
///
/// Re-validates each `editable_rows` row via [`EditableUserRow::to_sanitized`]
/// (the canonical typestate pipeline). Returns:
/// - `rows` — valid [`SanitizedUserRow`] values (ready to send);
/// - `errors` — a list of `(index, reason)` for invalid rows, for highlighting/messaging.
///
/// Used by `launch_op` instead of reading `csv.rows` directly, so that the
/// user's edits in the editable table (including generated passwords) are taken
/// into account. Fail-soft: invalid rows are skipped and the operation runs over
/// the valid ones; if there are none, the caller reports an error.
///
/// Returns an empty `Vec` if `editable_rows` has not been loaded yet.
#[must_use]
pub fn collect_sanitized_rows(
    state: &Signal<AppState>,
) -> (Vec<SanitizedUserRow>, Vec<(usize, String)>) {
    let read = state.read();
    let Some(rows) = read.csv.editable_rows.as_ref() else {
        return (Vec::new(), Vec::new());
    };
    let mut valid = Vec::with_capacity(rows.len());
    let mut errors = Vec::new();
    for (idx, editable) in rows.iter().enumerate() {
        match editable.to_sanitized() {
            Ok(sanitized) => valid.push(sanitized),
            // Localized reason via error_i18n (CsvRowError is typed).
            Err(e) => errors.push((idx, crate::error_i18n::csv_row_error(&e))),
        }
    }
    (valid, errors)
}
