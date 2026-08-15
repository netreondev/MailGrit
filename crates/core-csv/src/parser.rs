//! Streaming CSV parser for bulk user imports.
//!
//! Schema: `domain,username,password,display_name,quota_mb`.
//! Records are read by the bounded RFC-4180 [`RecordReader`](crate::record)
//! (quoted fields supported; no physical line is ever buffered past the
//! record cap — the real protection against OOM on hostile input). Strict
//! limits ([`MAX_CSV_ROWS`],
//! [`MAX_CSV_FIELD_BYTES`](mailgrit_core_domain::MAX_CSV_FIELD_BYTES), record
//! bytes) protect against DoS. Returns a
//! [`RawCsvRow`](mailgrit_core_domain::RawCsvRow) (Unverified); field validation
//! happens in `core-domain` via the typestate.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::record::{RecordOutcome, RecordReader};
use crate::util::strip_bom;
use mailgrit_core_domain::{CsvRowError, EXPECTED_CSV_COLUMNS, MAX_CSV_ROWS};
use std::io::{BufRead, BufReader};

/// Canonical CSV column names, in `RawCsvRow` order. Re-exported from
/// core-domain (`CLASSICAL_FIELD_NAMES`) — the single source of truth shared
/// with the operation profiles and the mapping layer.
pub use mailgrit_core_domain::CLASSICAL_FIELD_NAMES as CSV_HEADER;

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
    /// A record contains bytes that are not valid UTF-8. Per-row (the record
    /// is rejected); parsing of the remaining records continues.
    #[error("line {line_no} is not valid UTF-8")]
    InvalidUtf8 {
        /// Line number (1-based).
        line_no: usize,
    },
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
/// Returns `Err` only on fatal errors: I/O and exceeding `MAX_CSV_ROWS`.
/// Everything else — domain validation, over-limit lines/fields, invalid
/// UTF-8 — is a per-row failure accumulated in `ParsedCsv::failed`.
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
    let mut records = RecordReader::new(reader);

    while let Some(record) = records.next_record()? {
        let fields = match record.outcome {
            RecordOutcome::Fields(fields) => fields,
            RecordOutcome::Failed(err) => {
                // Limits and encoding problems are per-row failures (the old
                // parser made them fatal in the mapping layer and misreported
                // them in the classic one); the reader has already resynced.
                result.failed.push(FailedRow {
                    line_no: record.line_no,
                    fields: vec![record.raw],
                    error: err,
                });
                continue;
            }
        };

        if !header_seen && is_header(&fields) {
            header_seen = true;
            continue;
        }
        if is_blank_record(&fields) {
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

        match try_parse_row(&fields) {
            Ok(sanitized) => result.rows.push(sanitized),
            Err(err) => result.failed.push(FailedRow {
                line_no: record.line_no,
                fields,
                error: CsvParseError::Row {
                    line_no: record.line_no,
                    source: err,
                },
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

/// A blank record: a single field that is empty/whitespace (an empty line).
/// A line of only commas (",,,,") is NOT blank — it is a data row that fails
/// validation (same semantics as the line-based parser).
fn is_blank_record(fields: &[String]) -> bool {
    matches!(fields, [f] if f.trim().is_empty())
}

/// Checks whether the split fields form the CSV header.
///
/// Case-insensitive and whitespace-trimming, so a header exported by Excel/
/// LibreOffice with different casing (e.g. `Domain,Username,...`) — including
/// a quoted one — is still recognized and skipped.
fn is_header(fields: &[String]) -> bool {
    if fields.len() != EXPECTED_CSV_COLUMNS {
        return false;
    }
    fields
        .iter()
        .zip(CSV_HEADER.iter())
        .all(|(cell, expected)| cell.trim().eq_ignore_ascii_case(expected))
}

/// Parses raw fields into a `SanitizedUserRow` via the core-domain typestate pipeline.
fn try_parse_row(fields: &[String]) -> Result<mailgrit_core_domain::SanitizedUserRow, CsvRowError> {
    let raw = mailgrit_core_domain::RawCsvRow::new(fields.to_vec());
    raw.parse()
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
