//! Bounded RFC-4180 record reader (the single CSV wire-format layer).
//!
//! Hard memory guarantees, enforced BEFORE buffering:
//! - no physical line is ever buffered beyond `MAX_RECORD_BYTES + 1`
//!   (`BufRead::take` + `read_until`, not `BufRead::lines()` which allocates
//!   the whole line before any limit can run);
//! - an endless line (no `\n` over the cap) is drained in bounded chunks and
//!   reported as a per-record failure — the parser stays in sync and memory
//!   stays O(cap);
//! - a record spanning several physical lines (a quoted field containing
//!   newlines) is bounded by the per-field cap on the accumulated field
//!   content plus the bounded raw display copy.
//!
//! Format support (RFC 4180): quoted fields, doubled quotes inside quotes,
//! commas inside quotes, newlines inside quotes. Lenient (like the `csv`
//! crate): a quote in the middle of an unquoted field is data; an unterminated
//! quote at EOF closes at EOF.
//!
//! Per-record failures (over-limit line/field, invalid UTF-8) do NOT abort the
//! file — they come back as [`RecordOutcome::Failed`] and the next record is
//! still parsed (the "failed rows accumulate" contract). Only I/O errors are
//! fatal.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::parser::CsvParseError;
use crate::util::MAX_LINE_BYTES;
use mailgrit_core_domain::MAX_CSV_FIELD_BYTES;
use std::io::{BufRead, Read};

/// Maximum raw size of one physical CSV line in bytes. The reader never
/// buffers more than this + 1; same budget the old line-based parser enforced.
pub const MAX_RECORD_BYTES: usize = MAX_LINE_BYTES;

/// The split outcome for one raw record.
pub enum RecordOutcome {
    /// Successfully split into unquoted fields.
    Fields(Vec<String>),
    /// The record is rejected (line/field over limit, invalid UTF-8); the
    /// reader has already resynchronized to the next record boundary.
    Failed(CsvParseError),
}

/// One CSV record.
pub struct Record {
    /// 1-based number of the FIRST physical line of the record.
    pub line_no: usize,
    /// Raw record text (lossy UTF-8, bounded by [`MAX_RECORD_BYTES`]); used
    /// for `FailedRow` display.
    pub raw: String,
    /// Split fields, or the per-record failure.
    pub outcome: RecordOutcome,
}

/// Reads CSV records from a `BufRead` with the memory bounds described in the
/// module docs.
pub struct RecordReader<R: BufRead> {
    inner: R,
    line_no: usize,
}

impl<R: BufRead> RecordReader<R> {
    /// Wraps a reader. Line numbers start at 1.
    #[must_use]
    pub const fn new(inner: R) -> Self {
        Self { inner, line_no: 0 }
    }

    /// Returns the next record, or `None` at EOF.
    ///
    /// # Errors
    ///
    /// - [`std::io::Error`] — only genuine I/O failures; limit breaches and
    ///   invalid UTF-8 are per-record [`RecordOutcome::Failed`], not `Err`.
    pub fn next_record(&mut self) -> Result<Option<Record>, std::io::Error> {
        let start_line = self.line_no.saturating_add(1);
        let mut acc = Accumulator::new(start_line);
        let mut got_any = false;
        while let Some(chunk) = self.read_bounded_line()? {
            got_any = true;
            self.line_no = self.line_no.saturating_add(1);
            let PhysicalLine {
                bytes,
                had_newline,
                truncated,
            } = chunk;
            if truncated {
                // The physical line exceeds the cap: fail the record and drain
                // the remainder of THIS line in bounded chunks (the record
                // cannot span further — it ends at this line's newline).
                acc.fail(CsvParseError::LineTooLong {
                    line_no: start_line,
                    max: MAX_RECORD_BYTES,
                });
                self.drain_rest_of_line()?;
                break;
            }
            if let Ok(text) = std::str::from_utf8(&bytes) {
                acc.feed_line(text, had_newline);
            } else {
                acc.fail(CsvParseError::InvalidUtf8 {
                    line_no: start_line,
                });
                // Bounded lossy copy for FailedRow display.
                acc.push_raw_lossy(&bytes);
                acc.state_only_feed(had_newline);
            }
            if had_newline && !acc.in_quotes {
                break; // record boundary reached outside quotes
            }
        }
        if !got_any {
            return Ok(None);
        }
        let error = acc.error.take();
        let raw = std::mem::take(&mut acc.raw);
        let fields = acc.finish();
        let outcome = error.map_or_else(|| RecordOutcome::Fields(fields), RecordOutcome::Failed);
        Ok(Some(Record {
            line_no: start_line,
            raw,
            outcome,
        }))
    }

    /// Reads one physical line, never buffering more than `MAX_RECORD_BYTES + 1`
    /// bytes. `truncated` means the line continues past the cap (no newline
    /// within it) — the caller must drain the remainder.
    fn read_bounded_line(&mut self) -> Result<Option<PhysicalLine>, std::io::Error> {
        let cap = u64::try_from(MAX_RECORD_BYTES.saturating_add(1)).unwrap_or(u64::MAX);
        let mut buf = Vec::new();
        let n = (&mut self.inner).take(cap).read_until(b'\n', &mut buf)?;
        if n == 0 {
            return Ok(None);
        }
        let had_newline = buf.last() == Some(&b'\n');
        if had_newline {
            buf.pop();
            if buf.last() == Some(&b'\r') {
                buf.pop(); // CRLF
            }
        }
        Ok(Some(PhysicalLine {
            bytes: buf,
            had_newline,
            truncated: !had_newline && n == MAX_RECORD_BYTES.saturating_add(1),
        }))
    }

    /// Discards the rest of an over-limit physical line in bounded chunks.
    fn drain_rest_of_line(&mut self) -> Result<(), std::io::Error> {
        let cap = u64::try_from(MAX_RECORD_BYTES.saturating_add(1)).unwrap_or(u64::MAX);
        loop {
            let mut scratch = Vec::new();
            let n = (&mut self.inner)
                .take(cap)
                .read_until(b'\n', &mut scratch)?;
            if n == 0 || scratch.last() == Some(&b'\n') {
                return Ok(());
            }
        }
    }
}

/// One physical line as read from the wire.
struct PhysicalLine {
    bytes: Vec<u8>,
    had_newline: bool,
    truncated: bool,
}

/// Incremental RFC-4180 field splitter over physical lines.
struct Accumulator {
    line_no: usize,
    fields: Vec<String>,
    current: String,
    current_bytes: usize,
    raw: String,
    in_quotes: bool,
    field_opened: bool,
    error: Option<CsvParseError>,
}

impl Accumulator {
    const fn new(line_no: usize) -> Self {
        Self {
            line_no,
            fields: Vec::new(),
            current: String::new(),
            current_bytes: 0,
            raw: String::new(),
            in_quotes: false,
            field_opened: false,
            error: None,
        }
    }

    fn fail(&mut self, err: CsvParseError) {
        if self.error.is_none() {
            self.error = Some(err);
        }
    }

    /// Feeds one valid-UTF-8 physical line. `had_newline`: the line ended with
    /// a newline; if the record continues (inside quotes), that newline is
    /// field data.
    fn feed_line(&mut self, line: &str, had_newline: bool) {
        self.account_raw(line, had_newline);
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            if self.in_quotes {
                if ch == '"' {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        self.push_field_char('"');
                    } else {
                        self.in_quotes = false;
                    }
                } else {
                    self.push_field_char(ch);
                }
            } else if ch == ',' {
                self.finish_field();
            } else if ch == '"' && !self.field_opened {
                // A quote opens a quoted field only at the field start.
                self.in_quotes = true;
                self.field_opened = true;
            } else {
                self.field_opened = true;
                self.push_field_char(ch);
            }
        }
        if had_newline && self.in_quotes {
            // The record continues: the newline belongs to the quoted field.
            self.push_field_char('\n');
        }
    }

    /// Feeds a physical line whose bytes are NOT valid UTF-8: only the quote
    /// state is tracked (to find the record end); content is discarded.
    fn state_only_feed(&mut self, had_newline: bool) {
        // Content is unrecoverable for this record; the only thing that matters
        // is whether an unterminated quote keeps the record open. The raw
        // bytes are gone, so assume no new quote opens in them — at worst the
        // record ends one line early and the next line is reported on its own.
        if had_newline && self.in_quotes {
            self.push_field_char('\n');
        }
    }

    fn push_field_char(&mut self, ch: char) {
        if self.error.is_some() {
            return; // already failed: keep state, drop content
        }
        self.current_bytes = self.current_bytes.saturating_add(ch.len_utf8());
        if self.current_bytes > MAX_CSV_FIELD_BYTES {
            self.fail(CsvParseError::FieldTooLong {
                line_no: self.line_no,
                max: MAX_CSV_FIELD_BYTES,
            });
            return;
        }
        self.current.push(ch);
    }

    fn finish_field(&mut self) {
        self.fields.push(std::mem::take(&mut self.current));
        self.current_bytes = 0;
        self.field_opened = false;
    }

    /// Counts a fed line into the bounded raw display copy (lossy, joined
    /// with `\n` across physical lines) for `FailedRow` display.
    fn account_raw(&mut self, line: &str, had_newline: bool) {
        if self.raw.len() >= MAX_RECORD_BYTES {
            return;
        }
        if !self.raw.is_empty() {
            self.raw.push('\n');
        }
        self.raw.push_str(line);
        if had_newline && self.raw.len() < MAX_RECORD_BYTES {
            self.raw.push('\n');
        }
    }

    /// Appends lossily decoded bytes to the raw copy (invalid-UTF-8 record).
    fn push_raw_lossy(&mut self, bytes: &[u8]) {
        if self.raw.len() < MAX_RECORD_BYTES {
            self.raw.push_str(&String::from_utf8_lossy(bytes));
        }
    }

    fn finish(mut self) -> Vec<String> {
        // The trailing field (even empty: "a," ends with an empty field).
        self.finish_field();
        self.fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn records(data: &[u8]) -> Result<Vec<Record>, std::io::Error> {
        let mut reader = RecordReader::new(BufReader::new(data));
        let mut out = Vec::new();
        while let Some(r) = reader.next_record()? {
            out.push(r);
        }
        Ok(out)
    }

    /// Checked accessor (repo style: no direct indexing).
    fn at(recs: &[Record], i: usize) -> Result<&Record, String> {
        recs.get(i).ok_or_else(|| format!("missing record {i}"))
    }

    fn fields_of(r: &Record) -> Result<Vec<String>, String> {
        match &r.outcome {
            RecordOutcome::Fields(f) => Ok(f.clone()),
            RecordOutcome::Failed(e) => Err(format!("expected fields, got failure: {e}")),
        }
    }

    #[test]
    fn plain_lines_split_on_comma() -> TestResult {
        let recs = records(
            b"a,b,c
",
        )?;
        assert_eq!(recs.len(), 1);
        assert_eq!(fields_of(at(&recs, 0)?)?, ["a", "b", "c"]);
        assert_eq!(at(&recs, 0)?.line_no, 1);
        Ok(())
    }

    #[test]
    fn crlf_endings_stripped() -> TestResult {
        let recs = records(
            b"a,b
c,d
",
        )?;
        assert_eq!(fields_of(at(&recs, 0)?)?, ["a", "b"]);
        assert_eq!(fields_of(at(&recs, 1)?)?, ["c", "d"]);
        Ok(())
    }

    #[test]
    fn last_line_without_newline_is_a_record() -> TestResult {
        let recs = records(b"a,b")?;
        assert_eq!(recs.len(), 1);
        assert_eq!(fields_of(at(&recs, 0)?)?, ["a", "b"]);
        Ok(())
    }

    #[test]
    fn trailing_comma_yields_empty_last_field() -> TestResult {
        let recs = records(
            b"a,b,
",
        )?;
        assert_eq!(fields_of(at(&recs, 0)?)?, ["a", "b", ""]);
        Ok(())
    }

    #[test]
    fn quoted_comma_is_data() -> TestResult {
        let recs = records(
            b"\"Petrov, Ivan\",x
",
        )?;
        assert_eq!(fields_of(at(&recs, 0)?)?, ["Petrov, Ivan", "x"]);
        Ok(())
    }

    #[test]
    fn doubled_quote_unescapes() -> TestResult {
        let recs = records(
            b"\"say \"\"hi\"\"\"
",
        )?;
        assert_eq!(fields_of(at(&recs, 0)?)?, ["say \"hi\""]);
        Ok(())
    }

    #[test]
    fn quoted_newline_spans_physical_lines() -> TestResult {
        let recs = records(
            b"\"line1
line2\",second
",
        )?;
        assert_eq!(recs.len(), 1, "one logical record across two lines");
        assert_eq!(
            fields_of(at(&recs, 0)?)?,
            [
                "line1
line2",
                "second"
            ]
        );
        assert_eq!(
            at(&recs, 0)?.line_no,
            1,
            "line_no of the FIRST physical line"
        );
        Ok(())
    }

    #[test]
    fn quote_mid_field_is_data_lenient() -> TestResult {
        let recs = records(
            b"a\"b,c
",
        )?;
        assert_eq!(fields_of(at(&recs, 0)?)?, ["a\"b", "c"]);
        Ok(())
    }

    #[test]
    fn empty_quoted_field_is_empty() -> TestResult {
        let recs = records(
            b"a,\"\",b
",
        )?;
        assert_eq!(fields_of(at(&recs, 0)?)?, ["a", "", "b"]);
        Ok(())
    }

    // The DoS regression: an endless line (no newline) over the cap must be a
    // per-record failure that does NOT consume the following records, and the
    // reader must never buffer more than cap+1 bytes of it.
    #[test]
    fn endless_line_is_failed_record_and_parser_resyncs() -> TestResult {
        let mut data = vec![b'a'; MAX_RECORD_BYTES + 100];
        data.extend_from_slice(
            b"
good.com,u,p,n,100
",
        );
        let recs = records(&data)?;
        assert_eq!(recs.len(), 2, "failed record + the next valid record");
        assert!(matches!(
            at(&recs, 0)?.outcome,
            RecordOutcome::Failed(CsvParseError::LineTooLong { line_no: 1, .. })
        ));
        assert_eq!(
            fields_of(at(&recs, 1)?)?,
            ["good.com", "u", "p", "n", "100"]
        );
        Ok(())
    }

    #[test]
    fn invalid_utf8_is_per_record_failure() -> TestResult {
        let mut data = b"good.com,u,p,n,100
"
        .to_vec();
        data.extend_from_slice(&[0xff, 0xfe, b'\n']);
        data.extend_from_slice(
            b"good.com,u2,p2,n2,200
",
        );
        let recs = records(&data)?;
        assert_eq!(recs.len(), 3);
        assert_eq!(fields_of(at(&recs, 0)?)?.len(), 5);
        assert!(matches!(
            at(&recs, 1)?.outcome,
            RecordOutcome::Failed(CsvParseError::InvalidUtf8 { line_no: 2 })
        ));
        assert_eq!(
            fields_of(at(&recs, 2)?)?,
            ["good.com", "u2", "p2", "n2", "200"]
        );
        Ok(())
    }

    #[test]
    fn field_over_cap_is_failed_record() -> TestResult {
        let big = "x".repeat(MAX_CSV_FIELD_BYTES + 1);
        let data = format!(
            "{big},u
"
        );
        let recs = records(data.as_bytes())?;
        assert!(matches!(
            at(&recs, 0)?.outcome,
            RecordOutcome::Failed(CsvParseError::FieldTooLong { line_no: 1, .. })
        ));
        Ok(())
    }

    // A quoted record spanning lines accumulates field content across lines;
    // the per-field cap rejects it (the record fails, the reader resyncs).
    #[test]
    fn multi_line_record_over_field_cap_fails() -> TestResult {
        let mut data = String::from("\"");
        data.push_str(&"a".repeat(MAX_CSV_FIELD_BYTES - 10));
        data.push('\n'); // newline INSIDE quotes: the record continues
        data.push_str(&"b".repeat(MAX_CSV_FIELD_BYTES));
        data.push_str(
            "\"
next
",
        );
        let recs = records(data.as_bytes())?;
        assert!(
            matches!(
                at(&recs, 0)?.outcome,
                RecordOutcome::Failed(CsvParseError::FieldTooLong { .. })
            ),
            "an over-cap multi-line quoted field must fail the record"
        );
        // The reader resynchronized on the next record.
        assert_eq!(fields_of(at(&recs, 1)?)?, ["next"]);
        Ok(())
    }

    // Round-trip with the escape side (see escape.rs): every field escaped by
    // escape_field parses back to the original value (for values that do not
    // need formula neutralization).
    #[test]
    fn round_trip_with_escape_field() -> TestResult {
        let values = [
            "plain",
            "with,comma",
            "with \"quote\"",
            "multi
line",
            "",
            "tab	here",
        ];
        let mut line = String::new();
        for v in values {
            line.push_str(&crate::escape::escape_field(v));
            line.push(',');
        }
        // strip the trailing comma + newline-terminate
        let joined = line.strip_trailing_comma();
        let recs = records(
            format!(
                "{joined}
"
            )
            .as_bytes(),
        )?;
        assert_eq!(fields_of(at(&recs, 0)?)?, values);
        Ok(())
    }

    /// Test-only helper: strips one trailing comma.
    trait StripTrailingComma {
        fn strip_trailing_comma(&self) -> String;
    }
    impl StripTrailingComma for String {
        fn strip_trailing_comma(&self) -> String {
            self.strip_suffix(',')
                .map_or_else(|| self.clone(), str::to_string)
        }
    }
}
