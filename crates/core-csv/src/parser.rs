//! Streaming CSV parser for bulk user imports.
//!
//! Schema: `domain,username,password,display_name,quota_mb`.
//! Streaming line-by-line parsing with strict limits
//! ([`MAX_CSV_ROWS`],
//! [`MAX_CSV_FIELD_BYTES`](mailgrit_core_domain::MAX_CSV_FIELD_BYTES)) protects
//! against OOM. Returns a
//! [`RawCsvRow`](mailgrit_core_domain::RawCsvRow) (Unverified); field validation
//! happens in `core-domain` via the typestate.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::util::{split_cells, strip_bom};
use mailgrit_core_domain::{CsvRowError, EXPECTED_CSV_COLUMNS, MAX_CSV_ROWS};
use std::io::{BufRead, BufReader};

/// Canonical CSV column names, in `RawCsvRow` order.
pub const CSV_HEADER: [&str; EXPECTED_CSV_COLUMNS] =
    ["domain", "username", "password", "display_name", "quota_mb"];

/// Streaming CSV parse error.
#[derive(Debug, thiserror::Error)]
pub enum CsvParseError {
    /// Maximum number of rows exceeded.
    #[error("maximum number of CSV rows exceeded: {actual} > {max}")]
    TooManyRows {
        /// Actual number of rows.
        actual: usize,
        /// Allowed maximum.
        max: usize,
    },
    /// A CSV line exceeds the allowed length.
    #[error("line {line_no} exceeds {max} bytes")]
    LineTooLong {
        /// Line number (1-based).
        line_no: usize,
        /// Allowed maximum bytes.
        max: usize,
    },
    /// A field exceeds the allowed length.
    #[error("field on line {line_no} exceeds {max} bytes")]
    FieldTooLong {
        /// Line number (1-based).
        line_no: usize,
        /// Allowed maximum bytes.
        max: usize,
    },
    /// Error reading from the source (I/O).
    #[error("CSV read error: {0}")]
    Io(#[from] std::io::Error),
    /// Validation error for a specific row (core-domain domain rules).
    #[error("line {line_no}: {source}")]
    Row {
        /// Line number (1-based).
        line_no: usize,
        /// Domain validation error.
        #[source]
        source: CsvRowError,
    },
}

/// Parse result: valid rows and rejected ones (with reasons).
#[derive(Debug, Default)]
pub struct ParsedCsv {
    /// Valid, sanitized rows.
    pub rows: Vec<mailgrit_core_domain::SanitizedUserRow>,
    /// Rejected rows: (line number, raw fields, error).
    pub failed: Vec<FailedRow>,
}

/// A rejected CSV row with a reason.
#[derive(Debug)]
pub struct FailedRow {
    /// Line number in the source file (1-based).
    pub line_no: usize,
    /// Raw fields of the row.
    pub fields: Vec<String>,
    /// Reason for rejection.
    pub error: CsvParseError,
}

/// Parses CSV from any `BufRead`; the header is skipped and failed rows
/// accumulate in `failed` (parsing is not interrupted).
///
/// # Errors
///
/// Returns `Err` on fatal errors: I/O, exceeding `MAX_CSV_ROWS`,
/// or an abnormally long line. Individual failed rows do NOT interrupt parsing —
/// they accumulate in `ParsedCsv::failed`.
pub fn parse_csv<R: BufRead>(reader: R) -> Result<ParsedCsv, CsvParseError> {
    parse_csv_with_limit(reader, MAX_CSV_ROWS)
}

/// Parses CSV with a configurable row-count limit.
///
/// # Errors
///
/// See [`parse_csv`].
pub fn parse_csv_with_limit<R: BufRead>(
    reader: R,
    max_rows: usize,
) -> Result<ParsedCsv, CsvParseError> {
    let mut result = ParsedCsv::default();
    let mut header_seen = false;

    for (index, line_result) in reader.lines().enumerate() {
        let line_no = index.saturating_add(1);
        let line = line_result?;

        if !header_seen && is_header(&line) {
            header_seen = true;
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }

        // saturating_add protects against counter overflow.
        let processed = result.rows.len().saturating_add(result.failed.len());
        if processed >= max_rows {
            return Err(CsvParseError::TooManyRows {
                actual: processed.saturating_add(1),
                max: max_rows,
            });
        }

        match split_cells(&line) {
            Ok(fields) => match try_parse_row(&fields) {
                Ok(sanitized) => result.rows.push(sanitized),
                Err(err) => result.failed.push(FailedRow {
                    line_no,
                    fields,
                    error: CsvParseError::Row {
                        line_no,
                        source: err,
                    },
                }),
            },
            Err(err) => result.failed.push(FailedRow {
                line_no,
                fields: vec![line],
                error: err,
            }),
        }
    }

    Ok(result)
}

/// Parses CSV from bytes (a convenience wrapper for tests and small files).
///
/// # Errors
///
/// See [`parse_csv`].
pub fn parse_csv_bytes(data: &[u8]) -> Result<ParsedCsv, CsvParseError> {
    // Strip the UTF-8 BOM (EF BB BF) — Excel/Word add it; without stripping,
    // the BOM attaches to the first field (domain) → validation error.
    parse_csv(BufReader::new(strip_bom(data)))
}

/// Checks whether a line is the CSV header.
///
/// Case-insensitive and whitespace-trimming, so a header exported by Excel/
/// LibreOffice with different casing (e.g. `Domain,Username,...`) is still
/// recognized and skipped. Mirrors the additive mapping parser's
/// case-insensitive column matching.
fn is_header(line: &str) -> bool {
    let mut cells = line.split(',').map(str::trim);
    let mut expected = CSV_HEADER.iter().copied();
    loop {
        match (cells.next(), expected.next()) {
            (Some(c), Some(h)) => {
                if !c.eq_ignore_ascii_case(h) {
                    return false;
                }
            }
            (None, None) => return true,
            // one iterator longer than the other → not a header
            _ => return false,
        }
    }
}

/// Parses raw fields into a `SanitizedUserRow` via the core-domain typestate pipeline.
fn try_parse_row(fields: &[String]) -> Result<mailgrit_core_domain::SanitizedUserRow, CsvRowError> {
    let raw = mailgrit_core_domain::RawCsvRow::new(fields.to_vec());
    raw.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_CSV: &str = concat!(
        "domain,username,password,display_name,quota_mb\n",
        "example.com,ivan.petrov,S3cur3P@ss1,Ivan Petrov,1024\n",
        "example.com,anna.kovalenko,Str0ngPwd!2,Anna Kovalenko,2048\n",
    );

    #[test]
    fn parses_valid_csv_with_header() -> Result<(), Box<dyn std::error::Error>> {
        let parsed = parse_csv_bytes(VALID_CSV.as_bytes())?;
        assert_eq!(parsed.rows.len(), 2);
        assert!(parsed.failed.is_empty());
        assert_eq!(
            parsed
                .rows
                .first()
                .ok_or("missing row 0")?
                .username
                .as_str(),
            "ivan.petrov"
        );
        assert_eq!(parsed.rows.get(1).ok_or("missing row 1")?.quota.mb(), 2048);
        Ok(())
    }

    #[test]
    fn parses_without_header() -> Result<(), Box<dyn std::error::Error>> {
        let data = b"example.com,user,Pass123,Name,512";
        let parsed = parse_csv_bytes(data)?;
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.rows.first().ok_or("missing row 0")?.quota.mb(), 512);
        Ok(())
    }

    #[test]
    fn collects_failed_rows_without_stopping() -> Result<(), Box<dyn std::error::Error>> {
        // Second line is invalid (email instead of domain), third is valid.
        let data = concat!(
            "domain,username,password,display_name,quota_mb\n",
            "user@bad.com,u,p,n,100\n",
            "good.com,u2,p2,n2,200\n",
        );
        let parsed = parse_csv_bytes(data.as_bytes())?;
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.failed.len(), 1);
        assert_eq!(parsed.failed.first().ok_or("missing failed 0")?.line_no, 2);
        assert_eq!(
            parsed
                .rows
                .first()
                .ok_or("missing row 0")?
                .username
                .as_str(),
            "u2"
        );
        Ok(())
    }

    #[test]
    fn skips_blank_lines() -> Result<(), Box<dyn std::error::Error>> {
        let data = concat!(
            "\n",
            "domain,username,password,display_name,quota_mb\n",
            "\n",
            "good.com,u,p,n,200\n",
            "\n",
        );
        let parsed = parse_csv_bytes(data.as_bytes())?;
        assert_eq!(parsed.rows.len(), 1);
        assert!(parsed.failed.is_empty());
        Ok(())
    }

    #[test]
    fn rejects_too_many_rows() -> Result<(), Box<dyn std::error::Error>> {
        let data = "good.com,u,p,n,200\ngood.com,u2,p2,n2,200\ngood.com,u3,p3,n3,300\n";
        match parse_csv_with_limit(data.as_bytes(), 2) {
            Ok(_) => return Err("expected TooManyRows error, got Ok".into()),
            Err(CsvParseError::TooManyRows { actual, max }) => {
                assert_eq!(max, 2);
                assert_eq!(actual, 3);
            }
            Err(other) => return Err(format!("expected TooManyRows, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn limit_allows_exact_boundary() -> Result<(), Box<dyn std::error::Error>> {
        // Checks the boundary condition `processed >= max_rows` (strict inequality).
        let data = "good.com,u,p,n,200\ngood.com,u2,p2,n2,200\n";
        let parsed = parse_csv_with_limit(data.as_bytes(), 2)?;
        assert_eq!(parsed.rows.len(), 2);
        Ok(())
    }

    #[test]
    fn empty_input_yields_empty_result() -> Result<(), Box<dyn std::error::Error>> {
        let parsed = parse_csv_bytes(b"")?;
        assert_eq!(parsed.rows.len(), 0);
        assert_eq!(parsed.failed.len(), 0);
        Ok(())
    }

    #[test]
    fn quota_defaults_when_empty_field() -> Result<(), Box<dyn std::error::Error>> {
        let data = b"good.com,u,p,n,";
        let parsed = parse_csv_bytes(data)?;
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(
            parsed.rows.first().ok_or("missing row 0")?.quota.mb(),
            mailgrit_core_domain::DEFAULT_QUOTA_MB
        );
        Ok(())
    }

    #[test]
    fn wrong_column_count_goes_to_failed() -> Result<(), Box<dyn std::error::Error>> {
        let data = b"good.com,u,p"; // 3 columns instead of 5
        let parsed = parse_csv_bytes(data)?;
        assert_eq!(parsed.rows.len(), 0);
        assert_eq!(parsed.failed.len(), 1);
        Ok(())
    }

    #[test]
    fn detects_header_case_insensitively_trimmed() {
        // Canonical lower-case header.
        assert!(is_header("domain,username,password,display_name,quota_mb"));
        // Whitespace-trimmed.
        assert!(is_header(
            " domain , username , password , display_name , quota_mb "
        ));
        // Different casing (as exported by Excel/LibreOffice) — must still match.
        assert!(is_header("Domain,Username,Password,Display_Name,Quota_MB"));
        assert!(is_header("DOMAIN,USERNAME,PASSWORD,DISPLAY_NAME,QUOTA_MB"));
        // A real data line must NOT be mistaken for a header.
        assert!(!is_header("domain,username"));
        assert!(!is_header("example.com,user,S3cur3P@ss1,User,1024"));
    }

    // Boundary limit tests (kill the `>` ↔ `==`/`>=` mutants in util::split_cells):
    // a line of exactly MAX_LINE_BYTES passes, at +1 byte it is rejected.
    #[test]
    fn line_at_max_bytes_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
        let prefix = "d.com,u,p,n,";
        let filler_len = crate::util::MAX_LINE_BYTES - prefix.len();
        let filler = "a".repeat(filler_len);
        let line = format!("{prefix}{filler}");
        assert_eq!(line.len(), crate::util::MAX_LINE_BYTES);
        let parsed = parse_csv_bytes(line.as_bytes())?;
        assert!(
            parsed
                .failed
                .iter()
                .all(|f| !matches!(f.error, crate::CsvParseError::LineTooLong { .. })),
            "a line at the limit boundary must not be LineTooLong"
        );
        Ok(())
    }

    #[test]
    fn line_over_max_bytes_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        // A line 1 byte longer than MAX_LINE_BYTES → LineTooLong.
        let prefix = "d.com,u,p,n,";
        let filler_len = crate::util::MAX_LINE_BYTES
            .saturating_sub(prefix.len())
            .saturating_add(1);
        let filler = "a".repeat(filler_len);
        let line = format!("{prefix}{filler}");
        assert!(line.len() > crate::util::MAX_LINE_BYTES);
        let parsed = parse_csv_bytes(line.as_bytes())?;
        assert!(
            parsed
                .failed
                .iter()
                .any(|f| matches!(f.error, crate::CsvParseError::LineTooLong { .. })),
            "a line over the limit must be LineTooLong"
        );
        Ok(())
    }

    // A field of exactly MAX_CSV_FIELD_BYTES passes, at +1 byte it is FieldTooLong.
    #[test]
    fn field_at_max_bytes_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
        let max = mailgrit_core_domain::MAX_CSV_FIELD_BYTES;
        let domain = "a".repeat(max);
        let line = format!("{domain},u,p,n,100");
        let parsed = parse_csv_bytes(line.as_bytes())?;
        assert!(
            parsed
                .failed
                .iter()
                .all(|f| !matches!(f.error, crate::CsvParseError::FieldTooLong { .. })),
            "a field at the limit boundary must not be FieldTooLong"
        );
        Ok(())
    }

    #[test]
    fn field_over_max_bytes_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let max = mailgrit_core_domain::MAX_CSV_FIELD_BYTES;
        let domain = "a".repeat(max.saturating_add(1));
        let line = format!("{domain},u,p,n,100");
        let parsed = parse_csv_bytes(line.as_bytes())?;
        assert!(
            parsed
                .failed
                .iter()
                .any(|f| matches!(f.error, crate::CsvParseError::FieldTooLong { .. })),
            "a field over the limit must be FieldTooLong"
        );
        Ok(())
    }
}
