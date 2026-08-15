//! Unit tests moved out of the production file (the `#[path]` pattern
//! used across the workspace; keeps the prod file under the 400-line spec).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

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

// ============================================================================
// Raw display copy (FailedRow.raw): exact pinning — the bounded-copy logic
// is observable ONLY through these assertions (mutation testing showed the
// boundaries untested).
// ============================================================================

// A single-line record: raw is the line itself, no separator/trailing newline.
#[test]
fn raw_display_single_line_is_exact() -> TestResult {
    let recs = records(b"a,b,c\n")?;
    assert_eq!(at(&recs, 0)?.raw, "a,b,c");
    Ok(())
}

// A record spanning lines: the physical lines are joined with '\n' in raw.
#[test]
fn raw_display_joins_physical_lines_with_newline() -> TestResult {
    let recs = records(b"\"line1\nline2\",second\n")?;
    assert_eq!(at(&recs, 0)?.raw, "\"line1\nline2\",second");
    Ok(())
}

// Boundary: a line of EXACTLY MAX_RECORD_BYTES fills raw to the cap; the
// trailing newline is NOT appended (raw.len() == cap, ends with data).
#[test]
fn raw_display_at_exact_cap_gets_no_trailing_newline() -> TestResult {
    let line = "a".repeat(MAX_RECORD_BYTES);
    let mut data = line.clone().into_bytes();
    data.push(b'\n');
    let recs = records(&data)?;
    let raw = &at(&recs, 0)?.raw;
    assert_eq!(raw.len(), MAX_RECORD_BYTES, "raw fills to the cap exactly");
    assert!(raw.ends_with('a'), "no trailing newline past the cap");
    Ok(())
}

// The raw copy never exceeds the cap even when a record spans many lines.
#[test]
fn raw_display_stays_bounded_across_lines() -> TestResult {
    let mut data = String::from("\"");
    data.push_str(&"a".repeat(MAX_RECORD_BYTES / 2));
    data.push('\n'); // inside quotes: the record continues
    data.push_str(&"b".repeat(MAX_RECORD_BYTES));
    data.push_str("\"\nnext\n");
    let recs = records(data.as_bytes())?;
    let r = at(&recs, 0)?;
    // The record itself fails (the quoted field exceeds the field cap) —
    // the raw copy is what FailedRow displays.
    assert!(matches!(r.outcome, RecordOutcome::Failed(_)));
    assert!(
        r.raw.len() <= 2 * MAX_RECORD_BYTES,
        "raw is bounded by cap + one line, got {}",
        r.raw.len()
    );
    Ok(())
}

// Invalid UTF-8: raw holds the lossy text (non-empty, bounded).
#[test]
fn raw_display_of_invalid_utf8_is_lossy_and_bounded() -> TestResult {
    let mut data = vec![0xff, 0xfe, b'\n'];
    data.extend_from_slice(b"next\n");
    let recs = records(&data)?;
    let r = at(&recs, 0)?;
    assert!(matches!(
        r.outcome,
        RecordOutcome::Failed(CsvParseError::InvalidUtf8 { .. })
    ));
    assert!(!r.raw.is_empty(), "the lossy raw copy is kept for display");
    assert!(r.raw.contains('\u{fffd}'), "invalid bytes decode lossily");
    assert!(r.raw.len() <= MAX_RECORD_BYTES);
    Ok(())
}

// Invalid UTF-8 on a LATER line of a record: the lossy part joins with the
// same '\n' separator as valid physical lines.
#[test]
fn raw_display_lossy_line_joins_with_separator() -> TestResult {
    let mut data = b"\"good\n".to_vec();
    data.push(0xff);
    data.push(b'\n');
    data.extend_from_slice(b"next\n");
    let recs = records(&data)?;
    let r = at(&recs, 0)?;
    assert!(matches!(
        r.outcome,
        RecordOutcome::Failed(CsvParseError::InvalidUtf8 { .. })
    ));
    // The valid first line verbatim, then the separator, then the lossy byte.
    assert!(
        r.raw.starts_with("\"good\n\u{fffd}"),
        "raw = {raw:?}",
        raw = r.raw
    );
    Ok(())
}
