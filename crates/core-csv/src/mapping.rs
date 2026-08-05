//! Flexible CSV parser with column mapping.
//!
//! Maps arbitrary source columns to canonical field names of the operation
//! profile, reusing the `RawCsvRow::parse` typestate validation. Flow:
//! [`detect_mapping`] matches the header ↔ profile fields (case-insensitive,
//! trimmed); [`parse_csv_with_mapping`] / [`parse_csv_bytes_auto`] reorder the
//! columns into canonical order, fill in defaults, and validate.

use crate::parser::{CsvParseError, FailedRow, ParsedCsv};
use crate::util::{split_cells, strip_bom};
use mailgrit_core_domain::{EXPECTED_CSV_COLUMNS, MAX_CSV_ROWS, OperationProfile, RawCsvRow};
use std::io::{BufRead, BufReader};

/// Canonical names of the classic 5 fields (in `RawCsvRow` order).
const CLASSICAL_FIELDS: [&str; EXPECTED_CSV_COLUMNS] =
    ["domain", "username", "password", "display_name", "quota_mb"];

/// Mapping of source columns to canonical field names of the operation profile.
///
/// `bindings[i].0` is a header cell (as it appears in the CSV), `bindings[i].1`
/// is the canonical field name; unrecognized source columns are ignored. The full
/// header in source order is stored in [`header`](Self::header).
#[derive(Debug, Clone)]
pub struct ColumnMapping {
    /// Source CSV header (cells in read order).
    pub header: Vec<String>,
    /// Bindings "header cell → canonical field name" (only matched ones).
    pub bindings: Vec<(String, String)>,
    /// Operation profile that the mapping is validated against.
    pub profile: OperationProfile,
}

/// Auto-detects a mapping from the header and profile.
///
/// For each profile field it looks for a matching header cell (case-insensitive,
/// trimmed); the first match wins. Unrecognized required fields are left without
/// a binding — row validation below will then produce a clear error.
#[must_use]
pub fn detect_mapping(header: &[String], profile: &OperationProfile) -> ColumnMapping {
    let mut bindings: Vec<(String, String)> = Vec::new();
    for field in &profile.fields {
        let target = field.name.to_ascii_lowercase();
        for cell in header {
            if cell.trim().eq_ignore_ascii_case(&target) {
                bindings.push((cell.clone(), field.name.to_string()));
                break;
            }
        }
    }
    // Warn (once per mapping, not per row) about matched profile fields that are
    // not in CLASSICAL_FIELDS (e.g. for the domain profile: transport,
    // is_backupmx). Values of these CSV columns never enter the RawCsvRow
    // typestate pipeline and are replaced by profile defaults — make this drop
    // visible so a user who explicitly maps such columns does not silently lose
    // data.
    for (cell, name) in &bindings {
        if !CLASSICAL_FIELDS.contains(&name.as_str()) {
            tracing::warn!(
                "column \"{cell}\" is mapped to profile field \"{name}\", which is not \
                 part of the classic schema; the CSV value will be ignored \
                 (the profile default is used)"
            );
        }
    }
    ColumnMapping {
        header: header.to_vec(),
        bindings,
        profile: profile.clone(),
    }
}

impl ColumnMapping {
    /// True if the number of bindings equals the number of profile fields (a full match).
    #[must_use]
    pub const fn binds_all_profile_fields(&self) -> bool {
        self.bindings.len() == self.profile.fields.len()
    }

    /// Index of the value column for a canonical field, or `None`.
    fn column_index_for(&self, field_name: &str) -> Option<usize> {
        let header_cell = self
            .bindings
            .iter()
            .find(|(_, name)| name == field_name)
            .map(|(cell, _)| cell.as_str())?;
        self.header.iter().position(|h| h == header_cell)
    }

    /// Field value from a row per the mapping, or `None` (field not mapped).
    fn value_for(&self, row: &[String], field_name: &str) -> Option<String> {
        let idx = self.column_index_for(field_name)?;
        row.get(idx).cloned()
    }
}

/// Streaming CSV parse with an explicit mapping.
///
/// Values are reordered into canonical order, missing optional fields are filled
/// with defaults, and then `RawCsvRow::new(vec).parse()` is called. Failed rows
/// accumulate in `failed` (parsing is not interrupted).
///
/// # Errors
///
/// Fatal errors (I/O, exceeding `MAX_CSV_ROWS`, an abnormally long line) are
/// returned as `Err`; per-row errors go into `ParsedCsv::failed`.
pub fn parse_csv_with_mapping<R: BufRead>(
    reader: R,
    mapping: &ColumnMapping,
) -> Result<ParsedCsv, CsvParseError> {
    let mut result = ParsedCsv::default();
    let mut header_seen = false;
    for (index, line_result) in reader.lines().enumerate() {
        let line_no = index.saturating_add(1);
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }
        // The first non-empty line is the header (skipped); the mapping is already set.
        if !header_seen {
            header_seen = true;
            continue;
        }
        check_row_budget(&result)?;
        let cells = split_cells(&line)?;
        process_row_mapped(&cells, line_no, &mut result, mapping);
    }
    Ok(result)
}

/// Parses CSV from bytes with an explicit mapping (stripping the UTF-8 BOM).
///
/// # Errors
///
/// See [`parse_csv_with_mapping`].
pub fn parse_csv_bytes_with_mapping(
    data: &[u8],
    mapping: &ColumnMapping,
) -> Result<ParsedCsv, CsvParseError> {
    parse_csv_with_mapping(BufReader::new(strip_bom(data)), mapping)
}

/// Auto-parse: detects the mapping from the header and parses.
///
/// The first non-empty line is checked via [`detect_mapping`]: if it covers all
/// profile fields it is used as the header; otherwise positional mode engages
/// (columns in canonical order are treated as data).
///
/// # Errors
///
/// See [`parse_csv_with_mapping`].
pub fn parse_csv_bytes_auto(
    data: &[u8],
    profile: &OperationProfile,
) -> Result<ParsedCsv, CsvParseError> {
    let reader = BufReader::new(strip_bom(data));
    parse_auto(reader, profile)
}

/// Streaming auto-parse with header detection and positional fallback.
fn parse_auto<R: BufRead>(
    reader: R,
    profile: &OperationProfile,
) -> Result<ParsedCsv, CsvParseError> {
    let mut result = ParsedCsv::default();
    let mut header_consumed = false;
    // Set when a header is detected; until then, positional mode is used.
    let mut detected: Option<ColumnMapping> = None;
    for (index, line_result) in reader.lines().enumerate() {
        let line_no = index.saturating_add(1);
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }
        check_row_budget(&result)?;
        if !header_consumed {
            let cells = split_cells(&line)?;
            let candidate = detect_mapping(&cells, profile);
            if candidate.binds_all_profile_fields() {
                detected = Some(candidate);
                header_consumed = true;
                continue;
            }
            // Not a header — positional mode; the current line is data.
            header_consumed = true;
            process_row_positional(&cells, line_no, &mut result, &line);
            continue;
        }
        let cells = split_cells(&line)?;
        match &detected {
            Some(mapping) => process_row_mapped(&cells, line_no, &mut result, mapping),
            None => process_row_positional(&cells, line_no, &mut result, &line),
        }
    }
    Ok(result)
}

/// Guard against exceeding `MAX_CSV_ROWS` (saturating arithmetic).
const fn check_row_budget(result: &ParsedCsv) -> Result<(), CsvParseError> {
    let processed = result.rows.len().saturating_add(result.failed.len());
    if processed >= MAX_CSV_ROWS {
        return Err(CsvParseError::TooManyRows {
            actual: processed.saturating_add(1),
            max: MAX_CSV_ROWS,
        });
    }
    Ok(())
}

/// Positional parse: cells in canonical order → `RawCsvRow::parse`.
/// Per-row errors do not interrupt parsing — they accumulate in `failed`.
fn process_row_positional(
    cells: &[String],
    line_no: usize,
    result: &mut ParsedCsv,
    raw_line: &str,
) {
    match RawCsvRow::new(cells.to_vec()).parse() {
        Ok(sanitized) => {
            result.rows.push(sanitized);
        }
        Err(err) => result.failed.push(FailedRow {
            line_no,
            fields: vec![raw_line.to_string()],
            error: CsvParseError::Row {
                line_no,
                source: err,
            },
        }),
    }
}

/// Mapped parse: reorders cells into canonical order and validates.
/// Per-row errors do not interrupt parsing — they accumulate in `failed`.
fn process_row_mapped(
    cells: &[String],
    line_no: usize,
    result: &mut ParsedCsv,
    mapping: &ColumnMapping,
) {
    // Required fields without a binding are rejected with a clear error (field name):
    // the typestate pipeline does not catch this (e.g. an empty password passes).
    for field in &mapping.profile.fields {
        if field.required && mapping.column_index_for(field.name).is_none() {
            result.failed.push(FailedRow {
                line_no,
                fields: cells.to_vec(),
                error: CsvParseError::Row {
                    line_no,
                    source: mailgrit_core_domain::CsvRowError::MissingRequiredField {
                        field: field.name,
                    },
                },
            });
            return;
        }
    }
    let mut classical: Vec<String> = Vec::with_capacity(EXPECTED_CSV_COLUMNS);
    for canonical in CLASSICAL_FIELDS {
        let value = mapping.value_for(cells, canonical).unwrap_or_else(|| {
            // Field not mapped: substitute the profile default, else an empty string.
            mapping
                .profile
                .fields
                .iter()
                .find(|f| f.name == canonical)
                .and_then(|f| f.default)
                .map_or_else(String::new, str::to_string)
        });
        classical.push(value);
    }
    match RawCsvRow::new(classical).parse() {
        Ok(sanitized) => result.rows.push(sanitized),
        Err(err) => result.failed.push(FailedRow {
            line_no,
            fields: cells.to_vec(),
            error: CsvParseError::Row {
                line_no,
                source: err,
            },
        }),
    }
}
