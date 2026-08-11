//! Common low-level helpers for CSV parsers (private module).
//!
//! Previously the line-length limit and the field splitting/validation logic were
//! duplicated in [`crate::parser`] and [`crate::mapping`] (the additive layer). Here
//! they share a single source of truth, so changing a limit or rule affects both parsers.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use mailgrit_core_domain::MAX_CSV_FIELD_BYTES;

/// Maximum length of a single CSV line (bytes), including delimiters.
///
/// Protects against abnormally long lines (e.g. a single endless line with no `\n`)
/// and against DoS via huge input.
pub const MAX_LINE_BYTES: usize = 16 * 1024;

/// Strips the UTF-8 BOM (`EF BB BF`) from the start of the data, if present.
///
/// Excel/Word add a BOM; without stripping it, it attaches to the first field
/// (domain) and breaks validation. Returns the slice without the BOM (or the
/// original if no BOM is present).
pub fn strip_bom(data: &[u8]) -> &[u8] {
    data.strip_prefix(b"\xef\xbb\xbf").unwrap_or(data)
}

/// Splits a line into cells by comma, validating length limits.
///
/// Checks: overall line length (`MAX_LINE_BYTES`) and each field length
/// (`MAX_CSV_FIELD_BYTES`, measured in **bytes**). Returns `Err` on a limit
/// breach with `line_no: 0` (the caller fills in the real line number in
/// `FailedRow`).
///
/// Note: this byte-budget check is a coarse DoS guard and is deliberately
/// measured in **bytes**, whereas the downstream semantic limits in
/// `core-domain` (username / domain / display_name / password length) are
/// measured in **Unicode chars**. A field that passes here may still be rejected
/// downstream on a char-count basis. See `MAX_CSV_FIELD_BYTES` for the
/// rationale and the invariant test that keeps the two regimes consistent.
///
/// # Errors
///
/// - [`CsvParseError::LineTooLong`] — line exceeds `MAX_LINE_BYTES`.
/// - [`CsvParseError::FieldTooLong`] — field exceeds `MAX_CSV_FIELD_BYTES`.
pub fn split_cells(line: &str) -> Result<Vec<String>, crate::parser::CsvParseError> {
    if line.len() > MAX_LINE_BYTES {
        return Err(crate::parser::CsvParseError::LineTooLong {
            line_no: 0,
            max: MAX_LINE_BYTES,
        });
    }
    let mut cells = Vec::new();
    for cell in line.split(',') {
        if cell.len() > MAX_CSV_FIELD_BYTES {
            return Err(crate::parser::CsvParseError::FieldTooLong {
                line_no: 0,
                max: MAX_CSV_FIELD_BYTES,
            });
        }
        cells.push(cell.to_string());
    }
    Ok(cells)
}
