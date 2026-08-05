//! Common low-level helpers for CSV parsers (private module).
//!
//! Previously the line-length limit and the field splitting/validation logic were
//! duplicated in [`crate::parser`] and [`crate::mapping`] (the additive layer). Here
//! they share a single source of truth, so changing a limit or rule affects both parsers.

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
/// (`MAX_CSV_FIELD_BYTES`). Returns `Err` on a limit breach with `line_no: 0`
/// (the caller fills in the real line number in `FailedRow`).
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
