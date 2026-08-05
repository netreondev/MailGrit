//! Helper functions for loading CSV on the dashboard.
//!
//! Extracted from [`dashboard`](super) to keep each file under the 400-line
//! limit (the RSX screen is structurally large). This file handles native file
//! picking, auto column mapping, and CSV parsing.

use crate::csv_summary::CsvSummary;
use crate::state::AppState;
use dioxus::prelude::*;
use mailgrit_core_csv::{CsvParseError, detect_mapping, parse_csv_bytes_auto};
use mailgrit_core_domain::{EditableUserRow, SanitizedUserRow};
use std::sync::Arc;

/// Loads and parses the selected CSV file into [`AppState`].
///
/// File reading runs in `spawn_blocking` (so it does not block the Dioxus
/// event-loop on large CSVs); parsing and state updates happen after the bytes
/// are returned.
///
/// Auto-mapping uses the profile of the selected target (for the classic
/// 5-column CSV the result is identical to `parse_csv_bytes`). Valid rows are
/// also duplicated into the editable layer `editable_rows` (plain `String`),
/// which the user edits directly in the table; on execution, rows are
/// re-validated via [`EditableUserRow::to_sanitized`].
pub fn load_csv_file(state: &mut Signal<AppState>, path: &std::path::Path) {
    // File reading is blocking IO; offloaded to block_in_place on the tokio
    // runtime so the Dioxus event-loop does not stall on large CSVs.
    let path = path.to_path_buf();
    let bytes = match tokio::task::block_in_place(|| std::fs::read(&path)) {
        Ok(b) => b,
        Err(e) => {
            state.write().error_msg = Some(t!("csv.read_error", error = e).to_string());
            return;
        }
    };
    let profile = state.read().effective_profile();
    match parse_csv_bytes_auto(&bytes, &profile) {
        Ok(parsed) => {
            let header = extract_header(&bytes);
            let mapping = detect_mapping(&header, &profile);
            let summary = CsvSummary::from_parsed(&parsed);
            // Editable layer: copy valid rows into plain `String`.
            let editable: Vec<EditableUserRow> =
                parsed.rows.iter().map(EditableUserRow::from).collect();
            let mut s = state.write();
            s.csv = Some(Arc::new(parsed));
            s.column_mapping = Some(Arc::new(mapping));
            s.editable_rows = Some(editable);
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

/// Extracts the CSV header (the first non-empty line) from the bytes for
/// auto-detecting the column mapping. The logic matches the auto-parser:
/// strip the UTF-8 BOM, skip empty lines, split on commas and trim each cell.
fn extract_header(data: &[u8]) -> Vec<String> {
    let clean = data.strip_prefix(b"\xef\xbb\xbf").unwrap_or(data);
    let text = match std::str::from_utf8(clean) {
        Ok(t) => t,
        Err(e) => {
            // Invalid UTF-8: column auto-mapping is impossible. Return an empty
            // header (mapping yields 0 bindings) and log the reason — do not
            // mask the failure with a silent empty result.
            tracing::warn!("CSV contains invalid UTF-8, column auto-mapping disabled: {e}");
            return Vec::new();
        }
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return trimmed.split(',').map(|c| c.trim().to_string()).collect();
        }
    }
    // No non-empty lines — empty header (mapping yields 0 bindings).
    Vec::new()
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
    let Some(rows) = read.editable_rows.as_ref() else {
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
